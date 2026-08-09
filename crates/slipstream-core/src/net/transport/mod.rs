//! Pluggable packet I/O. The hot path calls [`Transport::send`] / [`Transport::recv`]
//! directly — no async runtime is involved.

mod loopback;
mod qos;
mod udp;

pub use loopback::{loopback_pair, LoopbackTransport};
pub use qos::{grow_socket_buffers, set_dscp_default, set_media_qos, MediaClass, QosFlow};
pub use udp::{spawn_data_punch, UdpTransport, PUNCH_MAGIC};
// Phase 5: the pacing-capability probe + GSO gate + low-latency socket-buffer target, surfaced
// for the host's send thread (records them in the session stats / latency artifact).
#[cfg(target_os = "linux")]
pub use udp::{gso_enabled, low_latency_send_target_bytes, pacing_capabilities};

/// A datagram transport. `recv` is non-blocking: it returns `Ok(None)` when no packet
/// is currently available, so the caller (decode/present thread) never blocks here.
pub trait Transport: Send + Sync {
    /// Send one packet. `Ok(true)` = handed to the kernel; `Ok(false)` = dropped locally because
    /// the send buffer was momentarily full (WouldBlock) — a non-fatal loss the FEC/keyframe path
    /// recovers, surfaced so the caller can count it (`packets_send_dropped`) instead of it being
    /// invisible. `Err` = a real send failure.
    fn send(&self, packet: &[u8]) -> std::io::Result<bool>;

    /// Send a whole frame's packets in as few syscalls as possible, returning how many were
    /// handed to the kernel (the caller counts `packets.len() - sent` as send-buffer drops). This
    /// is the 1 Gbps+ lever: the [`UdpTransport`](super::UdpTransport) override uses `sendmmsg`
    /// (~64 packets/syscall) instead of one `send` each — at ~125k pkt/s that is the difference
    /// between ~2k and ~125k syscalls/sec. The default is the scalar `send` loop (correct for the
    /// loopback transport). On a full send buffer it stops early and reports
    /// the partial count rather than blocking — same lossy, FEC-protected contract as `send`.
    fn send_batch(&self, packets: &[&[u8]]) -> std::io::Result<usize> {
        let mut sent = 0;
        for p in packets {
            if self.send(p)? {
                sent += 1;
            }
        }
        Ok(sent)
    }

    /// Send a frame's equal-size packets using UDP Generic Segmentation Offload where available:
    /// one `sendmsg` hands the kernel a big buffer it splits into `gso_size` UDP datagrams, building
    /// ~1 GSO skb per ≤64 segments instead of one skb per packet. This is the multi-Gbps lever —
    /// research shows ~2.4× throughput at equal CPU and ~40× fewer syscalls, and that `sendmmsg`
    /// batching alone is insufficient (it still builds one skb per datagram). The
    /// [`UdpTransport`](super::UdpTransport) Linux override implements it (opt-in via
    /// `SLIPSTREAM_GSO=1` pending pace-aware chunk spacing — see the `gso` module doc — with
    /// auto-fallback on any GSO error); the default just delegates to [`send_batch`](Self::send_batch),
    /// correct for loopback and non-Linux. Same lossy, FEC-protected short-count contract as `send_batch`.
    fn send_gso(&self, packets: &[&[u8]]) -> std::io::Result<usize> {
        self.send_batch(packets)
    }

    fn recv(&self) -> std::io::Result<Option<Vec<u8>>>;

    /// Receive up to `out.len()` datagrams in as few syscalls as possible, writing each into its
    /// `out[i]` buffer (sized ≥ a max datagram) and its byte count into `lens[i]`; returns how many
    /// arrived (`0` = none available; non-blocking). The recv counterpart of [`send_batch`]: the
    /// [`UdpTransport`](super::UdpTransport) override uses `recvmmsg` into a caller-owned, reused
    /// buffer ring — no per-packet allocation or syscall at line rate. The default does a single
    /// scalar [`recv`](Self::recv) into `out[0]` (correct for the loopback transport).
    fn recv_batch(&self, out: &mut [Vec<u8>], lens: &mut [usize]) -> std::io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        match self.recv()? {
            Some(pkt) => {
                let n = pkt.len().min(out[0].len());
                out[0][..n].copy_from_slice(&pkt[..n]);
                lens[0] = n;
                Ok(1)
            }
            None => Ok(0),
        }
    }

    /// The send-buffer target the transport should apply when the low-latency profile asks
    /// (Phase 5): the default is the balanced 32 MiB. The `UdpTransport` override maps
    /// `frame_bytes` to ≈2 frame payloads clamped to 256 KiB..4 MiB.
    fn latency_send_buffer_target(&self, frame_bytes: usize) -> usize {
        let _ = frame_bytes;
        crate::transport::qos::TARGET_SOCKBUF
    }

    /// Best-effort `SO_SNDBUF` apply. Default no-op — only the UDP transport has a socket.
    fn set_send_buffer_target(&mut self, _bytes: usize) {}

    /// The granted `SO_SNDBUF` (post-clamping) or 0 when the transport can't report one.
    fn send_buffer_granted(&self) -> u64 {
        0
    }

    /// The kernel's send-queue occupancy estimate (bytes), when the transport can report one.
    fn kernel_send_queue_bytes(&self) -> Option<u64> {
        None
    }
}
