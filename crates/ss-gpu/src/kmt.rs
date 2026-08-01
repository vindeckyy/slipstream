/// `KMTQUERYADAPTERINFOTYPE::KMTQAITYPE_ADAPTERTYPE` (d3dkmdt.h).
const KMTQAITYPE_ADAPTERTYPE: u32 = 15;

/// `D3DKMT_OPENADAPTERFROMLUID`: LUID in, kernel adapter handle out.
#[repr(C)]
struct OpenAdapterFromLuid {
    luid_low: u32,
    luid_high: i32,
    adapter: u32,
}
/// `D3DKMT_QUERYADAPTERINFO`.
#[repr(C)]
struct QueryAdapterInfo {
    adapter: u32,
    ty: u32,
    private_data: *mut core::ffi::c_void,
    private_data_size: u32,
}
/// `D3DKMT_CLOSEADAPTER`.
#[repr(C)]
struct CloseAdapter {
    adapter: u32,
}

#[link(name = "gdi32")]
extern "system" {
    fn D3DKMTOpenAdapterFromLuid(arg: *mut OpenAdapterFromLuid) -> i32;
    fn D3DKMTQueryAdapterInfo(arg: *mut QueryAdapterInfo) -> i32;
    fn D3DKMTCloseAdapter(arg: *mut CloseAdapter) -> i32;
}

/// The `D3DKMT_ADAPTERTYPE` bits for the adapter with this LUID, `None` when the kernel
/// query fails (callers fail open — better a listed twin than a hidden real GPU).
pub fn adapter_type_bits(luid_low: u32, luid_high: i32) -> Option<u32> {
    // SAFETY: every pointer handed to the three D3DKMT calls addresses a stack local that
    // outlives the call; NTSTATUS >= 0 is success. The kernel handle is closed on every
    // path that opened it, including a failed query.
    unsafe {
        let mut open = OpenAdapterFromLuid {
            luid_low,
            luid_high,
            adapter: 0,
        };
        if D3DKMTOpenAdapterFromLuid(&mut open) < 0 {
            return None;
        }
        let mut bits: u32 = 0;
        let mut query = QueryAdapterInfo {
            adapter: open.adapter,
            ty: KMTQAITYPE_ADAPTERTYPE,
            private_data: (&mut bits as *mut u32).cast(),
            private_data_size: size_of::<u32>() as u32,
        };
        let status = D3DKMTQueryAdapterInfo(&mut query);
        let mut close = CloseAdapter {
            adapter: open.adapter,
        };
        let _ = D3DKMTCloseAdapter(&mut close);
        (status >= 0).then_some(bits)
    }
}
