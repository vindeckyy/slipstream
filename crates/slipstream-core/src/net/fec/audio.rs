//! Audio resilience primitives (design/audio-resilience.md): RS erasure parity over groups of
//! `0xC9` audio datagrams, so a lost 5 ms Opus packet can be rebuilt from its group's parity
//! instead of becoming a click or a PLC gap. Host side generates parity per group; client side
//! rebuilds a missing data packet from the survivors + parity.
//!
//! A "group" is a fixed run of consecutive audio sequence numbers (e.g. [`AUDIO_GROUP_LEN`] =
//! 8 = 40 ms). The host emits each data packet with the tail `[group_id][parity_count][kind]`
//! (see [`crate::quic::encode_audio_datagram`]), then one parity datagram per group carrying
//! all the RS parity shards. Because the parity datagram trails its group's data (the host's
//! send-side reorder window guarantees ordering), a receiver that saw any loss in the group
//! reconstructs the missing frames from the shards.
//!
//! Erasure code: the shared [`ErasureCoder`] (GF(2⁸) classic RS — the same scheme as the
//! GameStream audio FEC path, and unconstrained by the GF(2¹⁶) even-shard-length rule).
//! Audio frames are tiny (stereo 5 ms Opus ≈ 80–160 B), so a whole group fits comfortably
//! in the coder's shard budget.

use crate::fec::{ErasureCoder, FecError, Gf8Coder};

/// Audio packets per FEC group. 8 × 5 ms = 40 ms — large enough to amortize the parity
/// overhead and ride out a wifi burst, small enough that a group's shards stay well under
/// the codec's shard-count ceiling and the parity datagram stays tiny.
pub const AUDIO_GROUP_LEN: usize = 8;

/// The strongest parity a sender emits per group (2 shards → recovers up to 2 lost data
/// packets per 40 ms). The host scales this down to 1 on LAN where loss is near zero.
pub const AUDIO_MAX_PARITY: usize = 2;

/// One audio data packet inside a group, as the encoder/decoder see it: its sequence number
/// and the raw Opus payload (may be empty for DTX).
#[derive(Clone, Debug)]
pub struct AudioFecData {
    pub seq: u32,
    pub data: Vec<u8>,
}

/// Compute the RS parity shards for one audio group.
///
/// `packets` must be at most [`AUDIO_GROUP_LEN`] and must carry contiguous `seq`s (the group's
/// sequence range). Shards are equalized to the group's max payload length (zero-padded), which
/// the receiver's rebuild mirrors, so the coder's equal-length invariant holds.
///
/// Returns `parity_count` shards (≤ [`AUDIO_MAX_PARITY`]), or `Err` if the coder refuses
/// (e.g. a shard longer than the codec ceiling — a pathological 5 ms frame).
pub fn generate_parity(
    coder: &dyn ErasureCoder,
    packets: &[AudioFecData],
    parity_count: usize,
) -> Result<Vec<Vec<u8>>, FecError> {
    if packets.is_empty() || packets.len() > AUDIO_GROUP_LEN {
        return Err(FecError::Config("audio FEC group size out of range"));
    }
    if parity_count > AUDIO_MAX_PARITY {
        return Err(FecError::Config("audio FEC parity out of range"));
    }
    let max_len = packets.iter().map(|p| p.data.len()).max().unwrap_or(0);
    if max_len == 0 {
        return Ok(vec![Vec::new(); parity_count]);
    }
    // The coder requires equal-length shards; zero-pad every payload to the group max (the
    // receiver mirrors this padding when rebuilding).
    let refs: Vec<Vec<u8>> = packets
        .iter()
        .map(|p| {
            let mut buf = vec![0u8; max_len];
            buf[..p.data.len()].copy_from_slice(&p.data);
            buf
        })
        .collect();
    let refs: Vec<&[u8]> = refs.iter().map(|b| b.as_slice()).collect();
    coder.encode(&refs, parity_count)
}

/// Rebuild missing data packets in a group from the received data + parity.
///
/// `packets` holds the group's received data packets (at least one); `missing` lists the
/// group positions whose packet did not arrive, in order. `parity_count` is the group's
/// DECLARED parity strength (from the tail — the M the shards were generated with, which the
/// RS codec's (k, m) math needs even when not all M shards arrived). `parity` is the received
/// parity shards (`(index, bytes)`). On success returns the rebuilt data payloads for each
/// position in `missing` (zero-length for a missing DTX packet). On failure (`Err`) the
/// caller falls back to the existing PLC path — the group was lost beyond what the parity
/// could cover.
pub fn rebuild(
    coder: &dyn ErasureCoder,
    packets: &[AudioFecData],
    missing: &[usize],
    parity_count: usize,
    parity: &[(usize, Vec<u8>)],
) -> Result<Vec<Vec<u8>>, FecError> {
    let total = packets.len() + missing.len();
    if total > AUDIO_GROUP_LEN {
        return Err(FecError::Config("audio FEC group size out of range"));
    }
    let max_len = packets
        .iter()
        .map(|p| p.data.len())
        .chain(parity.iter().map(|(_, p)| p.len()))
        .max()
        .unwrap_or(0);
    if max_len == 0 {
        // Every shard is empty (all-DTX group): every missing packet was silence anyway.
        return Ok(missing.iter().map(|_| Vec::new()).collect());
    }

    // Build the group's K+M slot buffers: present data slots carry their payload (padded to
    // the group max length, the equal-length invariant the coder needs), missing slots are
    // zeroed and get filled in place by `reconstruct_into`.
    let total = packets.len() + missing.len();
    let mut missing_set = vec![false; total];
    for &m in missing {
        if m < total {
            missing_set[m] = true;
        }
    }
    let mut slots: Vec<Vec<u8>> = Vec::with_capacity(total);
    let mut have: Vec<bool> = Vec::with_capacity(total);
    let mut data_idx = 0usize;
    for i in 0..total {
        if missing_set[i] {
            slots.push(vec![0u8; max_len]);
            have.push(false);
        } else {
            let p = &packets[data_idx];
            let mut buf = vec![0u8; max_len];
            let len = p.data.len().min(max_len);
            buf[..len].copy_from_slice(&p.data[..len]);
            slots.push(buf);
            have.push(true);
            data_idx += 1;
        }
    }
    let mut slot_refs: Vec<&mut [u8]> = slots.iter_mut().map(|s| s.as_mut_slice()).collect();
    // Reconstruct only the missing slots (the coder fills them in place). `recovery_count`
    // is the DECLARED parity strength of the block — the M the shards were generated with
    // (from the group's tail), which the RS codec's (k, m) math needs even when not all M
    // shards arrived. Passing a smaller M than the shards were made with would run the
    // wrong Cauchy matrix and produce garbage.
    if parity_count == 0 {
        return Err(FecError::Config("audio FEC parity missing"));
    }
    let parity_refs: Vec<(usize, &[u8])> = parity.iter().map(|(i, p)| (*i, p.as_slice())).collect();
    coder.reconstruct_into(parity_count, &mut slot_refs, &have, &parity_refs)?;

    Ok(missing.iter().map(|&m| slots[m].clone()).collect())
}

/// The GF(2⁸) coder used by both the host parity path and the client rebuild path.
///
/// Chosen over GF(2¹⁶): audio frames are tiny (a few hundred bytes) with no even-length
/// guarantee, and GF(2⁸)'s 255-shard ceiling is far above the 8-data + 2-parity audio group.
/// It's also the same scheme as the GameStream audio FEC path.
pub fn audio_coder() -> Box<dyn ErasureCoder> {
    Box::new(Gf8Coder::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FecScheme;
    use crate::fec::coder_for;

    fn coder() -> Box<dyn ErasureCoder> {
        coder_for(FecScheme::Gf8)
    }

    fn make_group(len: usize) -> Vec<AudioFecData> {
        (0..len)
            .map(|i| AudioFecData {
                seq: 1000 + i as u32,
                data: (0..(80 + i * 7)).map(|b| (i * 13 + b * 3) as u8).collect(),
            })
            .collect()
    }

    #[test]
    fn parity_roundtrip_recovers_any_single_loss() {
        let c = coder();
        for lose in 0..AUDIO_GROUP_LEN {
            let group = make_group(AUDIO_GROUP_LEN);
            let parity = generate_parity(c.as_ref(), &group, 1).unwrap();
            assert_eq!(parity.len(), 1);
            // Drop one data packet, rebuild it from the survivors + parity.
            let received: Vec<AudioFecData> = group
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != lose)
                .map(|(_, p)| p.clone())
                .collect();
            let rebuilt =
                rebuild(c.as_ref(), &received, &[lose], 1, &[(0, parity[0].clone())]).unwrap();
            assert_eq!(rebuilt.len(), 1);
            // The rebuilt shard is padded to the group max length (the equal-length RS
            // invariant); the meaningful prefix is the original frame.
            assert_eq!(
                &rebuilt[0][..group[lose].data.len()],
                group[lose].data,
                "lost position {lose}"
            );
        }
    }

    #[test]
    fn parity_roundtrip_recovers_two_losses_with_two_parity() {
        let c = coder();
        let group = make_group(AUDIO_GROUP_LEN);
        let parity = generate_parity(c.as_ref(), &group, 2).unwrap();
        let received: Vec<AudioFecData> = group
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 2 && *i != 5)
            .map(|(_, p)| p.clone())
            .collect();
        let rebuilt = rebuild(
            c.as_ref(),
            &received,
            &[2, 5],
            2,
            &[(0, parity[0].clone()), (1, parity[1].clone())],
        )
        .unwrap();
        assert_eq!(rebuilt.len(), 2);
        assert_eq!(&rebuilt[0][..group[2].data.len()], group[2].data);
        assert_eq!(&rebuilt[1][..group[5].data.len()], group[5].data);
    }

    #[test]
    fn dtx_empty_shards_roundtrip() {
        let c = coder();
        let group: Vec<AudioFecData> = (0..AUDIO_GROUP_LEN)
            .map(|i| AudioFecData {
                seq: i as u32,
                data: Vec::new(), // all silence — DTX
            })
            .collect();
        let parity = generate_parity(c.as_ref(), &group, 2).unwrap();
        let received: Vec<AudioFecData> = group
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 3 && *i != 6)
            .map(|(_, p)| p.clone())
            .collect();
        let rebuilt = rebuild(
            c.as_ref(),
            &received,
            &[3, 6],
            2,
            &[(0, parity[0].clone()), (1, parity[1].clone())],
        )
        .unwrap();
        assert_eq!(rebuilt.len(), 2);
        assert!(rebuilt[0].is_empty());
        assert!(rebuilt[1].is_empty());
    }

    #[test]
    fn parity_too_weak_errors_instead_of_garbage() {
        let c = coder();
        let group = make_group(AUDIO_GROUP_LEN);
        let parity = generate_parity(c.as_ref(), &group, 1).unwrap();
        let received: Vec<AudioFecData> = group
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 1 && *i != 4) // 2 losses, only 1 parity
            .map(|(_, p)| p.clone())
            .collect();
        assert!(rebuild(c.as_ref(), &received, &[1, 4], 1, &[(0, parity[0].clone())]).is_err());
    }
}
