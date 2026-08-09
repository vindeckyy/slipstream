//! FFmpeg's Vulkan hwcontext surface (`AVVulkanDeviceContext`, `AVVulkanFramesContext`,
//! `AVVkFrame`), bindgen-generated from the system headers at build time — see build.rs
//! for why this must not be hand-transcribed.
//!
//! The raw bindings use vulkan.h's own handle types (pointers on 64-bit). The [`ash`]
//! conversion helpers below cross between them and ash's u64-newtype handles; both sides
//! are the same underlying Vulkan object handles, so the casts are value-preserving.

// Unsafe-proof program: every `unsafe {}` here carries a `// SAFETY:` proof.
#![deny(clippy::undocumented_unsafe_blocks)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]
// bindgen's layout tests deref-null-pointer by design; silence the lints they trip.
#![allow(deref_nullptr)]
#![allow(unnecessary_transmutes)]

#[cfg(target_os = "linux")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// Configure the queue families on a caller-owned FFmpeg Vulkan context.
///
/// Older FFmpeg headers expose only the role-specific queue fields. Newer headers also expose the
/// queue-family list, which is populated when bindgen finds it.
#[cfg(target_os = "linux")]
pub fn configure_device_queues(
    hwctx: &mut AVVulkanDeviceContext,
    graphics_qf: i32,
    decode_qf: i32,
    graphics_queue_flags: u32,
    decode_video_caps: u32,
) {
    hwctx.queue_family_index = graphics_qf;
    hwctx.nb_graphics_queues = 1;
    hwctx.queue_family_tx_index = graphics_qf;
    hwctx.nb_tx_queues = 1;
    hwctx.queue_family_comp_index = graphics_qf;
    hwctx.nb_comp_queues = 1;
    hwctx.queue_family_encode_index = -1;
    hwctx.nb_encode_queues = 0;
    hwctx.queue_family_decode_index = decode_qf;
    hwctx.nb_decode_queues = 1;

    #[cfg(ss_ffvk_has_queue_family_list)]
    {
        const VIDEO_DECODE_BIT: u32 = 0x20;
        if graphics_qf == decode_qf {
            hwctx.qf[0] = AVVulkanDeviceQueueFamily {
                idx: graphics_qf,
                num: 1,
                flags: (graphics_queue_flags | VIDEO_DECODE_BIT) as _,
                video_caps: decode_video_caps as _,
            };
            hwctx.nb_qf = 1;
        } else {
            hwctx.qf[0] = AVVulkanDeviceQueueFamily {
                idx: graphics_qf,
                num: 1,
                flags: graphics_queue_flags as _,
                video_caps: 0,
            };
            hwctx.qf[1] = AVVulkanDeviceQueueFamily {
                idx: decode_qf,
                num: 1,
                flags: VIDEO_DECODE_BIT as _,
                video_caps: decode_video_caps as _,
            };
            hwctx.nb_qf = 2;
        }
    }

    #[cfg(not(ss_ffvk_has_queue_family_list))]
    let _ = (graphics_queue_flags, decode_video_caps);
}

/// Conversions between the generated vulkan.h handle types and ash's.
#[cfg(target_os = "linux")]
pub mod ashx {
    use super::*;
    use ash::vk::Handle as _;

    /// vulkan.h non-dispatchable handles are `*mut T` on 64-bit; ash's are `u64`
    /// newtypes. Same bits either way.
    pub fn image(h: VkImage) -> ash::vk::Image {
        ash::vk::Image::from_raw(h as u64)
    }

    pub fn semaphore(h: VkSemaphore) -> ash::vk::Semaphore {
        ash::vk::Semaphore::from_raw(h as u64)
    }

    // bindgen's enum representation is target-dependent, so the cast keeps the
    // conversion explicit at the ash boundary.
    #[allow(clippy::unnecessary_cast)]
    pub fn image_layout(l: VkImageLayout) -> ash::vk::ImageLayout {
        ash::vk::ImageLayout::from_raw(l as i32)
    }

    // --- ash → vulkan.h (filling AVVulkanDeviceContext) ---------------------------------

    pub fn to_instance(h: ash::vk::Instance) -> VkInstance {
        h.as_raw() as VkInstance
    }

    pub fn to_physical_device(h: ash::vk::PhysicalDevice) -> VkPhysicalDevice {
        h.as_raw() as VkPhysicalDevice
    }

    pub fn to_device(h: ash::vk::Device) -> VkDevice {
        h.as_raw() as VkDevice
    }

    /// ash's loader-level `vkGetInstanceProcAddr` as the header's PFN type. Both are the
    /// same C ABI function pointer (`extern "system"` == `extern "C"` on the platforms
    /// this crate builds for).
    pub fn to_get_proc_addr(
        f: unsafe extern "system" fn(
            ash::vk::Instance,
            *const std::ffi::c_char,
        ) -> ash::vk::PFN_vkVoidFunction,
    ) -> PFN_vkGetInstanceProcAddr {
        // SAFETY: both sides are `extern "system"` fn pointers with the identical signature —
        // `(VkInstance, *const c_char) -> PFN_vkVoidFunction`. The transmute only reinterprets the
        // ash-side type alias as our bindgen-side one, which are the same ABI type.
        unsafe { std::mem::transmute(f) }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    /// The allocator runs (links against the system libavutil) and the struct is
    /// readable at the offsets bindgen computed — sem_value zero-initialized.
    #[test]
    fn vk_frame_alloc_links_and_zeroes() {
        // SAFETY: `av_vk_frame_alloc` is libavutil's own allocator and returns either null —
        // asserted against before any field is read — or a zero-initialized `AVVkFrame` valid for
        // the reads below. The frame is deliberately leaked, so nothing frees it twice.
        unsafe {
            let f = av_vk_frame_alloc();
            assert!(!f.is_null(), "av_vk_frame_alloc returned NULL");
            assert_eq!((*f).sem_value[0], 0);
            assert_eq!((*f).queue_family[0], 0);
            // Leak the one test frame rather than binding av_free here.
        }
    }

    /// AV_NUM_DATA_POINTERS-sized arrays came through with the right length.
    #[test]
    fn frame_arrays_are_av_num_data_pointers() {
        // SAFETY: `AVVkFrame` is a `repr(C)` POD of scalars, handles and fixed-size arrays, so
        // all-zeroes is a valid bit pattern for it; the test only reads array lengths.
        let f: AVVkFrame = unsafe { std::mem::zeroed() };
        assert_eq!(f.img.len(), 8);
        assert_eq!(f.sem_value.len(), 8);
    }
}
