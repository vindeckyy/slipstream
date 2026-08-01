//! Error type and the stable C ABI status codes it maps to.

use thiserror::Error;

/// The core's internal error type. Crosses the C ABI as a [`SlipstreamStatus`] code.
#[derive(Debug, Error)]
pub enum SlipstreamError {
    #[error("invalid argument: {0}")]
    InvalidArg(&'static str),
    #[error("fec error: {0}")]
    Fec(#[from] crate::fec::FecError),
    #[error("crypto seal/open failed")]
    Crypto,
    #[error("malformed packet")]
    BadPacket,
    #[error("no complete frame available yet")]
    NoFrame,
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("timed out")]
    Timeout,
    #[error("session closed")]
    Closed,
    /// The host deliberately turned this connection away and said why (a typed QUIC application
    /// close, [`crate::reject::RejectReason`]) — distinct from transport trouble ([`Self::Io`] /
    /// [`Self::Timeout`]) and from a failed PIN proof ([`Self::Crypto`]) so UIs can render the
    /// real cause instead of a generic "not accepted".
    #[error("rejected by host: {0}")]
    Rejected(crate::reject::RejectReason),
}

pub type Result<T> = core::result::Result<T, SlipstreamError>;

/// Stable C ABI status codes. `Ok` is 0; all errors are negative so callers can
/// test `rc < 0`. Do not renumber existing variants — only append.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlipstreamStatus {
    Ok = 0,
    InvalidArg = -1,
    Fec = -2,
    Crypto = -3,
    BadPacket = -4,
    NoFrame = -5,
    Unsupported = -6,
    Io = -7,
    NullPointer = -8,
    Timeout = -9,
    Closed = -10,
    // -11..-19 reserved for future generic errors. The -20 block mirrors
    // `crate::reject::RejectReason` one-to-one so FFI callers (Swift, JNI) can
    // render the host's actual rejection reason.
    RejectedNotArmed = -20,
    RejectedBoundOther = -21,
    RejectedRateLimited = -22,
    RejectedIdentityRequired = -23,
    RejectedDenied = -24,
    RejectedApprovalTimeout = -25,
    RejectedSuperseded = -26,
    RejectedWireVersion = -27,
    RejectedBusy = -28,
    RejectedSetupFailed = -29,
    Panic = -99,
}

impl SlipstreamError {
    /// Map to the C ABI status code.
    pub fn status(&self) -> SlipstreamStatus {
        match self {
            SlipstreamError::InvalidArg(_) => SlipstreamStatus::InvalidArg,
            SlipstreamError::Fec(_) => SlipstreamStatus::Fec,
            SlipstreamError::Crypto => SlipstreamStatus::Crypto,
            SlipstreamError::BadPacket => SlipstreamStatus::BadPacket,
            SlipstreamError::NoFrame => SlipstreamStatus::NoFrame,
            SlipstreamError::Unsupported(_) => SlipstreamStatus::Unsupported,
            SlipstreamError::Io(_) => SlipstreamStatus::Io,
            SlipstreamError::Timeout => SlipstreamStatus::Timeout,
            SlipstreamError::Closed => SlipstreamStatus::Closed,
            SlipstreamError::Rejected(r) => {
                use crate::reject::RejectReason as R;
                match r {
                    R::PairingNotArmed => SlipstreamStatus::RejectedNotArmed,
                    R::PairingBoundToOtherDevice => SlipstreamStatus::RejectedBoundOther,
                    R::PairingRateLimited => SlipstreamStatus::RejectedRateLimited,
                    R::IdentityRequired => SlipstreamStatus::RejectedIdentityRequired,
                    R::Denied => SlipstreamStatus::RejectedDenied,
                    R::ApprovalTimeout => SlipstreamStatus::RejectedApprovalTimeout,
                    R::Superseded => SlipstreamStatus::RejectedSuperseded,
                    R::WireVersionMismatch => SlipstreamStatus::RejectedWireVersion,
                    R::Busy => SlipstreamStatus::RejectedBusy,
                    R::SetupFailed => SlipstreamStatus::RejectedSetupFailed,
                }
            }
        }
    }
}
