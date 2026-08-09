//! Generate bindings for `libavutil/hwcontext_vulkan.h` against the SYSTEM headers.
//!
//! ffmpeg-sys-next binds a curated header list that omits every hwcontext_*.h; the
//! Vulkan hwcontext structs (`AVVulkanDeviceContext`, `AVVkFrame`) are what let us run
//! FFmpeg's Vulkan Video decoder on the presenter's own VkDevice and read the decoded
//! VkImages back. Their layout depends on compile-time FF_API_* deprecation gates in
//! libavutil/version.h, so bindgen over the installed header is the only ABI-safe
//! source of truth — hand transcription would silently skew on the next FFmpeg bump.
//!
//! Header discovery uses pkg-config on Linux. Other targets get an empty file so
//! platform-specific client workspaces can still inspect the package metadata.

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-env-changed=PF_FFVK_VULKAN_INCLUDE");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let includes = match target_os.as_str() {
        "linux" => linux_includes(),
        _ => {
            std::fs::write(
                &out,
                "// ss-ffvk: Linux-only, empty on this target\n",
            )
            .unwrap();
            return;
        }
    };

    let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
        // The whole point of this crate: the Vulkan hwcontext surface…
        .allowlist_type("AVVulkan.*")
        .allowlist_type("AVVkFrame.*")
        .allowlist_function("av_vk_frame_alloc")
        .allowlist_function("av_vkfmt_from_pixfmt")
        // The feature structs chained into AVVulkanDeviceContext.device_features (plain
        // vulkan.h types; generating them here keeps the chain in one type system).
        .allowlist_type("VkPhysicalDeviceVulkan11Features")
        .allowlist_type("VkPhysicalDeviceVulkan12Features")
        .allowlist_type("VkPhysicalDeviceVulkan13Features")
        // AVVulkanFramesContext.img_flags values (plane views need MUTABLE_FORMAT).
        .allowlist_type("VkImageCreateFlagBits")
        // Timeline-semaphore wait — the pump measures true GPU decode completion.
        .allowlist_type("VkSemaphoreWaitInfo")
        .allowlist_type("PFN_vkWaitSemaphores")
        .allowlist_type("PFN_vkGetDeviceProcAddr")
        // …plus nothing else of FFmpeg: the core types these structs reference only
        // ever appear behind pointers here, so keep them opaque instead of duplicating
        // ffmpeg-sys-next's definitions (callers cast pointers between the crates).
        .opaque_type("AVHWDeviceContext")
        .opaque_type("AVHWFramesContext")
        .opaque_type("AVBufferRef")
        .opaque_type("AVFrame")
        .derive_debug(false)
        .layout_tests(true);
    for dir in &includes {
        builder = builder.clang_arg(format!("-I{}", dir.display()));
    }
    let bindings = builder.generate().expect(
        "bindgen over libavutil/hwcontext_vulkan.h failed — is `vulkan-headers` installed? \
         (the header includes <vulkan/vulkan.h>)",
    );
    bindings.write_to_file(&out).unwrap();

    // The av_vk_* symbols live in libavutil, which ffmpeg-sys-next already links into
    // every consumer of this crate; no extra link flags needed.
    println!("cargo:rustc-link-lib=avutil");
}

/// Include paths from pkg-config (libavutil for the hwcontext header; the Vulkan
/// headers usually live in /usr/include, but honor a registered vulkan.pc too).
/// PF_FFVK_VULKAN_INCLUDE prepends an explicit Vulkan-Headers include dir — for
/// cross builds and boxes without the system package.
fn linux_includes() -> Vec<PathBuf> {
    let mut includes: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = env::var("PF_FFVK_VULKAN_INCLUDE") {
        includes.push(PathBuf::from(dir));
    }
    let avutil = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("libavutil")
        .expect("pkg-config: libavutil not found — install the FFmpeg dev package");
    includes.extend(avutil.include_paths);
    if let Ok(vk) = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("vulkan")
    {
        includes.extend(vk.include_paths);
    }
    includes
}
