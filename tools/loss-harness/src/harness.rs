use slipstream_core::config::{Config, FecConfig, FecScheme, ProtocolPhase, Role};
use slipstream_core::crypto::SessionKey;
use slipstream_core::error::SlipstreamError;
use slipstream_core::session::Session;
use slipstream_core::transport::loopback_pair;

pub(crate) fn config(role: Role, scheme: FecScheme, drop_period: u32) -> Config {
    Config {
        role,
        phase: match scheme {
            FecScheme::Gf8 => ProtocolPhase::P1GameStream,
            FecScheme::Gf16 => ProtocolPhase::P2Slipstream,
        },
        fec: FecConfig {
            scheme,
            fec_percent: 25,
            max_data_per_block: 64,
        },
        shard_payload: 1024,
        max_frame_bytes: 8 * 1024 * 1024,
        encrypt: false,
        key: SessionKey::Aes128Gcm([0u8; 16]),
        salt: [0u8; 4],
        loopback_drop_period: drop_period,
    }
}

/// Returns (frames_completed, frames_attempted) for a loss setting. `streamed` feeds each AU
/// through the VIDEO_CAP_STREAMED_AU path (three encoder-chunk pushes + finish — sentinel
/// blocks then real totals) instead of one whole-AU submit, so the two wire shapes' recovery
/// curves can be compared directly (the Phase-2 "more, smaller units must not regress FEC" gate).
pub(crate) fn run(
    scheme: FecScheme,
    drop_period: u32,
    frames: usize,
    frame_len: usize,
    streamed: bool,
) -> (usize, usize) {
    let (h, c) = loopback_pair(drop_period, 0);
    let mut host = Session::new(config(Role::Host, scheme, drop_period), Box::new(h)).unwrap();
    let mut client = Session::new(config(Role::Client, scheme, drop_period), Box::new(c)).unwrap();

    let send_wires = |host: &mut Session, wires: Vec<Vec<u8>>| {
        let refs: Vec<&[u8]> = wires.iter().map(|w| w.as_slice()).collect();
        host.send_sealed(&refs).unwrap();
        drop(refs);
        host.reclaim_wires(wires);
    };
    let mut completed = 0;
    for f in 0..frames {
        let frame: Vec<u8> = (0..frame_len).map(|b| (b ^ f) as u8).collect();
        if streamed {
            let mut au = host.begin_streamed_frame_at(f as u64, 0, f as u32).unwrap();
            for chunk in frame.chunks(frame_len / 3 + 1) {
                // slice_end=false: the harness exercises the legacy full-FEC-block granularity
                // (its loopback client never advertises the P2 slice wire).
                let wires = host.seal_streamed_chunk(&mut au, chunk, false).unwrap();
                send_wires(&mut host, wires);
            }
            let wires = host.seal_streamed_finish(au).unwrap();
            send_wires(&mut host, wires);
        } else {
            host.submit_frame(&frame, f as u64, 0).unwrap();
        }
        match client.poll_frame() {
            Ok(got) => {
                if got.data == frame {
                    completed += 1;
                }
            }
            Err(SlipstreamError::NoFrame) => {} // unrecoverable at this loss rate
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    (completed, frames)
}
