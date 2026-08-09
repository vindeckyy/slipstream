//! Datagram demux: host → client audio/rumble (try_send: a lagging embedder drops the
//! newest packet rather than backing up the QUIC receive path).
//!
//! When the session negotiated audio FEC (`CLIENT_CAP_AUDIO_FEC` ∩ `HOST_CAP_AUDIO_FEC`), the
//! audio arm rebuilds a lost group member from its parity before forwarding — so a single
//! lost 5 ms Opus packet stops being a click/PLC gap. Rebuilds emit the group's data packets
//! in order (the parity datagram trails its group's data thanks to the host's send-side
//! reorder window), so the downstream decoder sees a gapless stream.

use super::*;

/// Max audio FEC groups held back for reassembly, in packets: 8 packets/group × 16 = 128
/// packets = 640 ms of audio. The parity datagram trails its group by at most one reorder
/// window (~8 packets), so this is generous headroom against a stalled parity datagram
/// while the group's data already arrived — the bound keeps memory flat and drops stale
/// groups (older than the window) so a dead parity datagram can't wedge the queue.
const AUDIO_FEC_GROUPS: usize = 16;

/// One audio FEC group in reassembly: the received data packets (in group order, by their
/// seq range) and the received parity shards. `parity_count` is the group's declared parity
/// strength (from its data datagrams) — the rebuild needs it even before the parity itself
/// arrives to size the shard budget.
#[derive(Clone)]
struct AudioFecGroup {
    /// Group id on the wire (wrapping u8; the host increments per group).
    group_id: u8,
    /// The group's parity strength as declared by its data packets.
    parity_count: u8,
    /// The group's data packets by group position (0..AUDIO_GROUP_LEN). `None` = lost (so
    /// far). Group position 0 ↔ the group's first seq (base_seq); later packets have
    /// `seq = base_seq + pos`.
    base_seq: u32,
    packets: Vec<Option<AudioPacket>>,
    /// Received parity shards: `(shard_index, bytes)`.
    parity: Vec<(usize, Vec<u8>)>,
}

/// The client-side audio FEC reassembler (design/audio-resilience.md).
///
/// Holds groups in flight until their parity datagram arrives (or they age out), rebuilds
/// a group's lost packets when it has enough parity, and forwards the group's data in order
/// to `audio_tx`. Runs in the datagram demux, so a rebuild is free of the downstream queue's
/// drop-newest policy — a rebuilt packet lands in order with its real seq, so the decoder's
/// `AudioGapTracker` never sees a gap for it.
///
/// Only active when the session negotiated audio FEC (both capability bits). Without it the
/// demux uses the plain path (every data datagram forwarded immediately).
struct AudioFecReassembler {
    audio_tx: std::sync::mpsc::SyncSender<AudioPacket>,
    groups: Vec<AudioFecGroup>,
    /// The newest group id fully released (wrapping), so a late data packet for an already
    /// released group is dropped rather than re-emitted out of order.
    last_released_group: Option<u8>,
    /// Rebuild scratch reused across calls so the steady state doesn't allocate.
    coder: Box<dyn crate::fec::ErasureCoder>,
}

impl AudioFecReassembler {
    fn new(audio_tx: std::sync::mpsc::SyncSender<AudioPacket>) -> Self {
        Self {
            audio_tx,
            groups: Vec::with_capacity(AUDIO_FEC_GROUPS),
            last_released_group: None,
            coder: crate::fec::audio_coder(),
        }
    }

    /// Feed one audio datagram. `data` is the raw datagram starting with [`AUDIO_MAGIC`].
    /// The reassembler only exists when FEC was negotiated, so the tail is always interpreted.
    fn push(&mut self, d: &[u8]) {
        let Some((seq, pts_ns, opus, tail)) = crate::quic::decode_audio_datagram_fec(d, true)
        else {
            return;
        };
        match tail {
            // Plain (pre-FEC host, or FEC off): forward immediately like before.
            None => self.release_plain(AudioPacket {
                seq,
                pts_ns,
                data: opus.to_vec(),
            }),
            Some(t) if t.kind == crate::quic::AUDIO_FEC_DATA => {
                let group_id = t.group_id;
                let parity_count = t.parity_count.min(crate::fec::AUDIO_MAX_PARITY as u8);
                self.push_data(
                    group_id,
                    parity_count,
                    AudioPacket {
                        seq,
                        pts_ns,
                        data: opus.to_vec(),
                    },
                );
            }
            Some(t) if t.kind == crate::quic::AUDIO_FEC_PARITY => {
                self.push_parity(t.group_id, t.parity_count, opus);
            }
            Some(_) => {} // unknown kind — ignore
        }
    }

    /// Forward a packet that arrived outside any FEC group (no tail).
    fn release_plain(&mut self, pkt: AudioPacket) {
        let _ = self.audio_tx.try_send(pkt);
    }

    /// The group position (0..AUDIO_GROUP_LEN) for `seq` given `base_seq` — the host emits
    /// exactly `AUDIO_GROUP_LEN` consecutive seqs per group. `None` if `seq` falls outside
    /// the group's range (a reordered packet from another group).
    fn group_pos(seq: u32, base_seq: u32) -> Option<usize> {
        let delta = seq.wrapping_sub(base_seq);
        if delta < crate::fec::AUDIO_GROUP_LEN as u32 {
            Some(delta as usize)
        } else {
            None
        }
    }

    fn push_data(&mut self, group_id: u8, parity_count: u8, pkt: AudioPacket) {
        // Reordered data for an already-released group: drop (the group is gone).
        if self
            .last_released_group
            .is_some_and(|l| crate::input::GamepadSnapshot::seq_newer(l, Some(group_id)))
        {
            return;
        }
        // Find or create the group. The host emits a group's packets strictly in order
        // (position 0 first), so the first packet of a new group is its base seq.
        let idx = match self.groups.iter().position(|g| g.group_id == group_id) {
            Some(i) => i,
            None => {
                // Drop the oldest group if we're over the cap (a parity datagram that never
                // arrives must not wedge the pipeline forever).
                if self.groups.len() >= AUDIO_FEC_GROUPS {
                    if let Some(old) = self.groups.first().cloned() {
                        self.release_group(old);
                        self.groups.remove(0);
                    }
                }
                self.groups.push(AudioFecGroup {
                    group_id,
                    parity_count,
                    base_seq: pkt.seq,
                    packets: (0..crate::fec::AUDIO_GROUP_LEN).map(|_| None).collect(),
                    parity: Vec::new(),
                });
                self.groups.len() - 1
            }
        };
        let g = &mut self.groups[idx];
        g.parity_count = parity_count.max(g.parity_count);
        let pos = match Self::group_pos(pkt.seq, g.base_seq) {
            Some(p) => p,
            None => return, // out-of-range (shouldn't happen) — drop
        };
        if g.packets[pos].is_none() {
            g.packets[pos] = Some(pkt);
        }
        // If we now have the whole group's data, release it immediately.
        if g.packets.iter().all(|p| p.is_some()) {
            let g = self.groups.remove(idx);
            self.release_group(g);
        }
    }

    fn push_parity(&mut self, group_id: u8, parity_count: u8, shards: &[u8]) {
        if self
            .last_released_group
            .is_some_and(|l| crate::input::GamepadSnapshot::seq_newer(l, Some(group_id)))
        {
            return; // group already released (and the parity is stale)
        }
        let idx = match self.groups.iter().position(|g| g.group_id == group_id) {
            Some(i) => i,
            None => return, // no data for this group — the group was released or lost
        };
        // Split the concatenated shards by the group's declared parity count.
        let count = parity_count.max(self.groups[idx].parity_count) as usize;
        if count == 0 || shards.is_empty() {
            return;
        }
        let shard_len = shards.len() / count;
        if shard_len == 0 {
            return;
        }
        let shards: Vec<(usize, Vec<u8>)> = (0..count)
            .map(|i| (i, shards[i * shard_len..(i + 1) * shard_len].to_vec()))
            .collect();
        let g = &mut self.groups[idx];
        g.parity_count = g.parity_count.max(count as u8);
        g.parity = shards;
        // Rebuild the missing packets and release.
        let g = self.groups.remove(idx);
        self.rebuild_and_release(g);
    }

    /// Rebuild a group's missing packets from its parity, then release all of it in order.
    fn rebuild_and_release(&mut self, mut g: AudioFecGroup) {
        let missing: Vec<usize> = g
            .packets
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_none())
            .map(|(i, _)| i)
            .collect();
        if !missing.is_empty() && !g.parity.is_empty() {
            // Rebuild only the missing slots (the codec's `reconstruct_into` fills them in
            // place). Equalize the received data to the group's max shard length like the
            // host did, so the codec's equal-length invariant holds.
            let data: Vec<crate::fec::AudioFecData> = g
                .packets
                .iter()
                .flatten()
                .map(|p| crate::fec::AudioFecData {
                    seq: p.seq,
                    data: p.data.clone(),
                })
                .collect();
            if let Ok(rebuilt) = crate::fec::rebuild(
                self.coder.as_ref(),
                &data,
                &missing,
                g.parity_count as usize,
                &g.parity,
            ) {
                for (i, payload) in rebuilt.into_iter().enumerate() {
                    let pos = missing[i];
                    let seq = g.base_seq.wrapping_add(pos as u32);
                    g.packets[pos] = Some(AudioPacket {
                        seq,
                        pts_ns: g
                            .packets
                            .iter()
                            .flatten()
                            .next()
                            .map(|p| p.pts_ns)
                            .unwrap_or(0),
                        data: payload,
                    });
                }
            }
            // If rebuild failed (parity too weak), leave the missing packets lost — the
            // downstream PLC path conceals them, same as no-FEC.
        }
        self.release_group(g);
    }

    /// Release a group's data packets in seq order (skip still-missing ones — the decoder's
    /// PLC conceals the gap, same as no-FEC).
    fn release_group(&mut self, g: AudioFecGroup) {
        // Only advance `last_released_group` if this group is newer than what we've released
        // (a stale group aged out could otherwise move the gate backward).
        let newer = self
            .last_released_group
            .is_none_or(|l| crate::input::GamepadSnapshot::seq_newer(g.group_id, Some(l)));
        if newer {
            self.last_released_group = Some(g.group_id);
        }
        for pkt in g.packets.into_iter().flatten() {
            let _ = self.audio_tx.try_send(pkt);
        }
    }
}

// One parameter per demuxed plane — grouping them into a struct would just move the field
// list one hop away from the single call site.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run(
    conn: quinn::Connection,
    audio_tx: std::sync::mpsc::SyncSender<AudioPacket>,
    rumble_tx: std::sync::mpsc::SyncSender<RumbleUpdate>,
    rumble_feed: super::super::rumble::RumbleFeed,
    hidout_tx: std::sync::mpsc::SyncSender<crate::quic::HidOutput>,
    hdr_meta_tx: std::sync::mpsc::SyncSender<crate::quic::HdrMeta>,
    host_timing_tx: std::sync::mpsc::SyncSender<crate::quic::HostTiming>,
    // The ABR encode signal's accumulator (see [`EncodeLatAcc`]) — fed HERE, not off
    // `host_timing_tx`: that channel is the overlay's, lossy and embedder-drained.
    encode_lat: Arc<Mutex<super::super::frame_channel::EncodeLatAcc>>,
    cursor_state_tx: std::sync::mpsc::SyncSender<crate::quic::CursorState>,
    // Audio FEC negotiated (client asked + host answered): rebuild lost group members.
    audio_fec: bool,
) {
    // Per-pad reorder gate for v2 rumble envelopes (the seq analog of the host's gamepad-state
    // gate): a datagram the network reordered must not roll a stopped motor back on. Legacy v1
    // datagrams carry no seq and bypass it (an old host's own periodic re-send is the only heal).
    let mut rumble_last_seq: [Option<u8>; crate::input::MAX_PADS] = [None; crate::input::MAX_PADS];
    let mut audio = audio_fec.then(|| AudioFecReassembler::new(audio_tx.clone()));
    while let Ok(d) = conn.read_datagram().await {
        match d.first() {
            Some(&crate::quic::AUDIO_MAGIC) => {
                if let Some(a) = &mut audio {
                    a.push(&d);
                } else {
                    // Plain path (no FEC negotiated): forward each data datagram immediately.
                    if let Some((seq, pts_ns, opus)) = crate::quic::decode_audio_datagram(&d) {
                        let _ = audio_tx.try_send(AudioPacket {
                            seq,
                            pts_ns,
                            data: opus.to_vec(),
                        });
                    }
                }
            }
            Some(&crate::quic::RUMBLE_MAGIC) => {
                if let Some(u) = crate::quic::decode_rumble_envelope(&d) {
                    // Gate v2 envelopes on their per-pad seq; forward v1 (envelope: None) as-is.
                    let fresh = match u.envelope {
                        Some(env) => {
                            let idx = u.pad as usize;
                            if idx < crate::input::MAX_PADS {
                                if crate::input::GamepadSnapshot::seq_newer(
                                    env.seq,
                                    rumble_last_seq[idx],
                                ) {
                                    rumble_last_seq[idx] = Some(env.seq);
                                    true
                                } else {
                                    false // reordered/duplicate — drop, keep the newer state
                                }
                            } else {
                                true // out-of-range pad (host never sends these): no gate
                            }
                        }
                        None => true,
                    };
                    if fresh {
                        let ttl = u.envelope.map(|e| e.ttl_ms);
                        // Both consumers are fed; an embedder drains exactly one of them
                        // (the legacy queue, or the policy engine's command API).
                        let _ = rumble_tx.try_send((u.pad, u.low, u.high, ttl));
                        rumble_feed.wire_update(u.pad, u.low, u.high, ttl);
                    }
                }
            }
            Some(&crate::quic::HIDOUT_MAGIC) => {
                if let Some(h) = HidOutput::decode(&d) {
                    let _ = hidout_tx.try_send(h);
                }
            }
            Some(&crate::quic::HDR_META_MAGIC) => {
                if let Some(m) = crate::quic::decode_hdr_meta_datagram(&d) {
                    let _ = hdr_meta_tx.try_send(m);
                }
            }
            Some(&crate::quic::HOST_TIMING_MAGIC) => {
                if let Some(t) = crate::quic::decode_host_timing_datagram(&d) {
                    if let Some(s) = &t.stages {
                        let mut acc = encode_lat.lock().unwrap();
                        acc.sum_us += s.encode_us as u64;
                        acc.count += 1;
                    }
                    let _ = host_timing_tx.try_send(t);
                }
            }
            Some(&crate::quic::CURSOR_STATE_MAGIC) => {
                if let Some(s) = crate::quic::decode_cursor_state_datagram(&d) {
                    let _ = cursor_state_tx.try_send(s);
                }
            }
            _ => {} // unknown tag — a newer host; ignore
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a data datagram for group `gid` at group position `pos`, seq `base + pos`.
    fn data_dgram(gid: u8, base: u32, pos: usize, payload: &[u8], parity: u8) -> Vec<u8> {
        crate::quic::encode_audio_datagram_fec(base + pos as u32, pos as u64, payload, gid, parity)
    }

    /// Build a parity datagram for group `gid` from the concatenated shards.
    fn parity_dgram(gid: u8, count: u8, shards: &[u8]) -> Vec<u8> {
        let mut d = crate::quic::encode_audio_datagram(0, 0, shards);
        d.push(gid);
        d.push(count);
        d.push(crate::quic::AUDIO_FEC_PARITY);
        d
    }

    #[test]
    fn reassembler_rebuilds_lost_packet_and_emits_in_order() {
        let (tx, rx) = std::sync::mpsc::sync_channel::<AudioPacket>(64);
        let mut r = AudioFecReassembler::new(tx);
        let gid = 1u8;
        let base = 1000u32;
        // Feed all 8 group packets except position 3 (the "lost" one).
        let mut payloads: Vec<Vec<u8>> = Vec::new();
        for pos in 0..crate::fec::AUDIO_GROUP_LEN {
            let payload: Vec<u8> = (0..(60 + pos * 5)).map(|b| (pos * 11 + b) as u8).collect();
            payloads.push(payload.clone());
            if pos != 3 {
                r.push(&data_dgram(gid, base, pos, &payload, 1));
            }
        }
        // Nothing emitted yet (group incomplete — parity not arrived).
        assert!(rx.try_recv().is_err());

        // Parity arrives → rebuild position 3, release all in order. The host generates the
        // parity over the FULL 8-packet group (before any loss), so the test must too.
        let full_group: Vec<crate::fec::AudioFecData> = (0..crate::fec::AUDIO_GROUP_LEN)
            .map(|p| crate::fec::AudioFecData {
                seq: base + p as u32,
                data: payloads[p].clone(),
            })
            .collect();
        let parity =
            crate::fec::generate_parity(crate::fec::audio_coder().as_ref(), &full_group, 1)
                .unwrap();
        r.push(&parity_dgram(gid, 1, &parity.concat()));

        // All 8 packets emitted, in order; the rebuilt one matches the original payload.
        let mut got: Vec<AudioPacket> = Vec::new();
        while let Ok(p) = rx.try_recv() {
            got.push(p);
        }
        assert_eq!(got.len(), crate::fec::AUDIO_GROUP_LEN);
        for (i, p) in got.iter().enumerate() {
            assert_eq!(p.seq, base + i as u32, "seq order at {i}");
            if i == 3 {
                assert_eq!(&p.data[..payloads[3].len()], payloads[3], "rebuilt payload");
            } else {
                assert_eq!(&p.data[..payloads[i].len()], payloads[i]);
            }
        }
    }

    #[test]
    fn reassembler_emits_complete_group_without_parity() {
        let (tx, rx) = std::sync::mpsc::sync_channel::<AudioPacket>(64);
        let mut r = AudioFecReassembler::new(tx);
        let gid = 2u8;
        let base = 2000u32;
        // Feed ALL 8 packets — group completes immediately, no parity needed.
        for pos in 0..crate::fec::AUDIO_GROUP_LEN {
            r.push(&data_dgram(gid, base, pos, &[pos as u8; 4], 1));
        }
        let mut got: Vec<AudioPacket> = Vec::new();
        while let Ok(p) = rx.try_recv() {
            got.push(p);
        }
        assert_eq!(got.len(), crate::fec::AUDIO_GROUP_LEN);
        assert_eq!(
            got.iter().map(|p| p.seq).collect::<Vec<_>>(),
            (base..base + crate::fec::AUDIO_GROUP_LEN as u32).collect::<Vec<_>>()
        );
    }
}
