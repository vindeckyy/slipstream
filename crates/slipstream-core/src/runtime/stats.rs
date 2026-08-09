//! Live counters for the frame-pacing / quality logic and the web UI.

use std::sync::atomic::{AtomicU64, Ordering};

/// Immutable snapshot, copied across the C ABI as `SlipstreamStats`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub frames_submitted: u64,
    pub frames_completed: u64,
    pub frames_dropped: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_dropped: u64,
    /// Packets the host could NOT hand to the kernel because the send buffer was full (WouldBlock)
    /// — the dominant loss mode at very high bitrate. Distinct from `packets_dropped` (recv-side
    /// reassembler rejects). A non-zero, growing value means the link/encoder is outrunning the
    /// send path; raise `net.core.wmem_max` / lower the bitrate, or wait for paced batched sending.
    pub packets_send_dropped: u64,
    pub fec_recovered_shards: u64,
    /// Shards counted into [`fec_recovered_shards`](Self::fec_recovered_shards) that later ARRIVED
    /// — reordered delivery lets a block reconstruct early from parity, so the still-in-flight
    /// shards it "recovered" were late, not lost. Loss estimators must net this out
    /// (`recovered - late`, see [`window_loss_ppm`](crate::quic::window_loss_ppm)) or plain
    /// reordering reads as packet loss and spooks adaptive FEC + the bitrate controller.
    /// Deliberately NOT mirrored into the C-ABI `SlipstreamStats` (loss windows run in-core).
    pub fec_late_shards: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    // --- Phase-1 latency/drop counters (mirrored into the C-ABI `SlipstreamStatsV2`; default 0
    // now — the capture/pacing paths that populate them land in later phases) ---
    /// Dropped past deadline before FEC/seal.
    pub frames_stale_dropped: u64,
    /// Dropped because the send channel was full.
    pub frames_backpressure_dropped: u64,
    /// Capture fence wait timed out.
    pub frames_fence_timeout: u64,
    /// Dropped during a recovery gap.
    pub frames_recovery_dropped: u64,
    /// Kernel ENOBUFS/EAGAIN send rejections.
    pub send_rejections: u64,
    /// Total time blocked enqueueing to the send channel.
    pub enqueue_blocked_us: u64,
    /// High-water mark of the send channel.
    pub send_queue_occupancy_max: u64,
    /// Actual SO_SNDBUF after the last set.
    pub socket_sndbuf_bytes: u64,
    /// 0/1: SO_TXTIME/ETF pacing active.
    pub so_txtime_active: u64,
    /// 0/1: UDP GSO active.
    pub gso_active: u64,
}

/// Atomic accumulators owned by a [`Session`](crate::session::Session). Snapshot to
/// [`Stats`] for readers. `Relaxed` ordering is fine: these are monotonic counters
/// read for display, never used to synchronize other memory.
#[derive(Default)]
pub struct StatsCounters {
    pub frames_submitted: AtomicU64,
    pub frames_completed: AtomicU64,
    pub frames_dropped: AtomicU64,
    pub packets_sent: AtomicU64,
    pub packets_received: AtomicU64,
    pub packets_dropped: AtomicU64,
    pub packets_send_dropped: AtomicU64,
    pub fec_recovered_shards: AtomicU64,
    pub fec_late_shards: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub frames_stale_dropped: AtomicU64,
    pub frames_backpressure_dropped: AtomicU64,
    pub frames_fence_timeout: AtomicU64,
    pub frames_recovery_dropped: AtomicU64,
    pub send_rejections: AtomicU64,
    pub enqueue_blocked_us: AtomicU64,
    pub send_queue_occupancy_max: AtomicU64,
    pub socket_sndbuf_bytes: AtomicU64,
    pub so_txtime_active: AtomicU64,
    pub gso_active: AtomicU64,
}

impl StatsCounters {
    #[inline]
    pub fn add(counter: &AtomicU64, n: u64) {
        counter.fetch_add(n, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> Stats {
        let l = Ordering::Relaxed;
        Stats {
            frames_submitted: self.frames_submitted.load(l),
            frames_completed: self.frames_completed.load(l),
            frames_dropped: self.frames_dropped.load(l),
            packets_sent: self.packets_sent.load(l),
            packets_received: self.packets_received.load(l),
            packets_dropped: self.packets_dropped.load(l),
            packets_send_dropped: self.packets_send_dropped.load(l),
            fec_recovered_shards: self.fec_recovered_shards.load(l),
            fec_late_shards: self.fec_late_shards.load(l),
            bytes_sent: self.bytes_sent.load(l),
            bytes_received: self.bytes_received.load(l),
            frames_stale_dropped: self.frames_stale_dropped.load(l),
            frames_backpressure_dropped: self.frames_backpressure_dropped.load(l),
            frames_fence_timeout: self.frames_fence_timeout.load(l),
            frames_recovery_dropped: self.frames_recovery_dropped.load(l),
            send_rejections: self.send_rejections.load(l),
            enqueue_blocked_us: self.enqueue_blocked_us.load(l),
            send_queue_occupancy_max: self.send_queue_occupancy_max.load(l),
            socket_sndbuf_bytes: self.socket_sndbuf_bytes.load(l),
            so_txtime_active: self.so_txtime_active.load(l),
            gso_active: self.gso_active.load(l),
        }
    }

    /// Count one dropped frame under `reason` (Phase 5 — every drop has a reason and recovery
    /// state; see [`FrameDropReason`]).
    #[inline]
    pub fn note_frame_drop(&self, reason: FrameDropReason) {
        Self::add(&self.frames_dropped, 1);
        match reason {
            FrameDropReason::StaleDeadline => Self::add(&self.frames_stale_dropped, 1),
            FrameDropReason::SendBackpressure => Self::add(&self.frames_backpressure_dropped, 1),
            FrameDropReason::FenceTimeout => Self::add(&self.frames_fence_timeout, 1),
            FrameDropReason::EncoderRecovery => Self::add(&self.frames_recovery_dropped, 1),
            FrameDropReason::TransportRejection => Self::add(&self.frames_recovery_dropped, 1),
        }
    }

    /// Accumulate send-channel enqueue-block time (µs) and the channel's high-water mark.
    #[inline]
    pub fn note_enqueue_blocked(&self, blocked_us: u64) {
        Self::add(&self.enqueue_blocked_us, blocked_us);
    }

    /// Record the send channel's high-water mark (bytes/frames observed by the host this session).
    #[inline]
    pub fn note_send_occupancy_max(&self, max_seen: u64) {
        self.send_queue_occupancy_max
            .store(max_seen, Ordering::Relaxed);
    }

    /// Record the granted `SO_SNDBUF` (actual, post-clamping) and the pacing-option latches.
    #[inline]
    pub fn set_socket_sndbuf(&self, bytes: u64) {
        self.socket_sndbuf_bytes.store(bytes, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_pacing_flags(&self, so_txtime: bool, gso: bool) {
        self.so_txtime_active
            .store(u64::from(so_txtime), Ordering::Relaxed);
        self.gso_active.store(u64::from(gso), Ordering::Relaxed);
    }
}

/// Why a frame was dropped on the host pipeline (Phase 5). Every drop carries a reason, and the
/// host records the reason + recovery state in the latency artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameDropReason {
    /// Past its capture deadline before FEC/seal (the frame's `deadline` elapsed).
    StaleDeadline,
    /// The send channel was full (backpressure — the pipeline could not keep up).
    SendBackpressure,
    /// The capture implicit fence did not signal within the budget (Phase 3).
    FenceTimeout,
    /// Dropped during a recovery gap (a reference was lost; dependent frames wait for RFI/IDR).
    EncoderRecovery,
    /// The kernel/transport rejected the send (ENOBUFS/EAGAIN at the socket).
    TransportRejection,
}
