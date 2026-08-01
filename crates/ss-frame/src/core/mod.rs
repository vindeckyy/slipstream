//! Shared media-pipeline vocabulary and helpers (HDR, metronome, QoS, DXGI).

pub mod frame;
pub mod hdr;
pub mod metronome;
pub mod session_tuning;
pub mod thread_qos;

#[cfg(target_os = "windows")]
pub mod dxgi;
