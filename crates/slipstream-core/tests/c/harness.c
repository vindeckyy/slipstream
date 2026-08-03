/*
 * slipstream-core C ABI harness — M1 acceptance.
 *
 * Proves the core links from C and round-trips encoded access units through the full
 * packetize -> FEC -> in-process loopback (with deterministic packet loss) -> FEC
 * recover -> reassemble path, recovering every byte exactly.
 *
 * Build/run: see tests/c/run.sh (also driven by `cargo test --test c_abi`).
 */
#include "slipstream_core.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static SlipstreamConfig make_config(uint32_t role, uint32_t drop_period) {
    SlipstreamConfig c;
    memset(&c, 0, sizeof(c));
    c.struct_size = (uint32_t)sizeof(SlipstreamConfig);
    c.role = role;                 /* 0 = host, 1 = client */
    c.phase = 1;                   /* P1, GameStream-compatible */
    c.fec_scheme = 0;              /* GF(2^8) */
    c.fec_percent = 25;
    c.max_data_per_block = 64;
    c.shard_payload = 1024;
    c.max_frame_bytes = 8 * 1024 * 1024;
    c.encrypt = 0;
    c.loopback_drop_period = drop_period;
    return c;
}

int main(void) {
    printf("slipstream-core C ABI harness (abi_version=%u)\n", slipstream_abi_version());

    const uint32_t DROP_PERIOD = 8;   /* drop 1 of every 8 packets */
    SlipstreamConfig host_cfg = make_config(0, DROP_PERIOD);
    SlipstreamConfig client_cfg = make_config(1, DROP_PERIOD);

    SlipstreamSession *host = NULL;
    SlipstreamSession *client = NULL;
    SlipstreamStatus rc = slipstream_test_loopback_pair(&host_cfg, &client_cfg, &host, &client);
    if (rc != SLIPSTREAM_STATUS_OK || !host || !client) {
        fprintf(stderr, "FAIL: loopback_pair rc=%d\n", (int)rc);
        return 1;
    }

    const size_t FRAME_LEN = 200000;  /* ~196 shards across 4 FEC blocks */
    const int FRAMES = 4;
    uint8_t *buf = (uint8_t *)malloc(FRAME_LEN);
    if (!buf) { fprintf(stderr, "FAIL: oom\n"); return 1; }

    int failures = 0;
    for (int f = 0; f < FRAMES; f++) {
        for (size_t i = 0; i < FRAME_LEN; i++) {
            buf[i] = (uint8_t)((i * 131u) + (unsigned)f * 17u);
        }

        rc = slipstream_host_submit_frame(host, buf, FRAME_LEN, (uint64_t)f * 1000000u, 0);
        if (rc != SLIPSTREAM_STATUS_OK) {
            fprintf(stderr, "FAIL: submit frame %d rc=%d\n", f, (int)rc);
            failures++;
            continue;
        }

        SlipstreamFrame out;
        memset(&out, 0, sizeof(out));
        rc = slipstream_client_poll_frame(client, &out);
        if (rc != SLIPSTREAM_STATUS_OK) {
            fprintf(stderr, "FAIL: poll frame %d rc=%d (expected recovery)\n", f, (int)rc);
            failures++;
            continue;
        }
        if (out.len != FRAME_LEN || memcmp(out.data, buf, FRAME_LEN) != 0) {
            fprintf(stderr, "FAIL: frame %d mismatch (len=%zu want=%zu)\n",
                    f, (size_t)out.len, FRAME_LEN);
            failures++;
            continue;
        }
        if (out.frame_index != (uint32_t)f) {
            fprintf(stderr, "FAIL: frame %d wrong index %u\n", f, out.frame_index);
            failures++;
        }
    }

    SlipstreamStats st;
    memset(&st, 0, sizeof(st));
    slipstream_get_stats(client, &st);
    printf("client stats: completed=%llu recovered_shards=%llu dropped_pkts=%llu rx_pkts=%llu\n",
           (unsigned long long)st.frames_completed,
           (unsigned long long)st.fec_recovered_shards,
           (unsigned long long)st.packets_dropped,
           (unsigned long long)st.packets_received);

    if (st.fec_recovered_shards == 0) {
        fprintf(stderr, "FAIL: expected FEC to recover lost shards, but recovered 0\n");
        failures++;
    }

    /* --- SlipstreamStatsV2: append-only surface (Phase 1b latency telemetry) --- */
    if (slipstream_stats_v2_size() != sizeof(SlipstreamStatsV2)) {
        fprintf(stderr, "FAIL: stats v2 size %zu != C sizeof %zu\n",
                (size_t)slipstream_stats_v2_size(), sizeof(SlipstreamStatsV2));
        failures++;
    }
    if (slipstream_stats_v2_version() != 1) {
        fprintf(stderr, "FAIL: stats v2 version %u != 1\n", slipstream_stats_v2_version());
        failures++;
    }

    /* Small-buffer contract: an embedder with a 16-byte view (header only) still gets
     * struct_size/version filled and NOTHING beyond its out_len. */
    SlipstreamStatsV2 prefix;
    memset(&prefix, 0xAB, sizeof(prefix));
    rc = slipstream_get_stats_v2(client, &prefix, 16);
    if (rc != SLIPSTREAM_STATUS_OK) {
        fprintf(stderr, "FAIL: stats v2 prefix get rc=%d\n", (int)rc);
        failures++;
    }
    if (prefix.struct_size != (uint64_t)slipstream_stats_v2_size()) {
        fprintf(stderr, "FAIL: stats v2 prefix struct_size=%llu want %llu\n",
                (unsigned long long)prefix.struct_size,
                (unsigned long long)slipstream_stats_v2_size());
        failures++;
    }
    if (prefix.version != 1) {
        fprintf(stderr, "FAIL: stats v2 prefix version=%u want 1\n", prefix.version);
        failures++;
    }
    if (prefix.frames_submitted != 0xABABABABABABABABULL) {
        fprintf(stderr, "FAIL: stats v2 wrote past the 16-byte prefix\n");
        failures++;
    }

    /* Full-size read: shared fields must agree with the SlipstreamStats snapshot, and the
     * Phase-1 counters must be readable (default 0 until later phases populate them). */
    SlipstreamStatsV2 st2;
    memset(&st2, 0, sizeof(st2));
    rc = slipstream_get_stats_v2(client, &st2, sizeof(st2));
    if (rc != SLIPSTREAM_STATUS_OK) {
        fprintf(stderr, "FAIL: stats v2 get rc=%d\n", (int)rc);
        failures++;
    }
    if (st2.struct_size != (uint64_t)slipstream_stats_v2_size() || st2.version != 1) {
        fprintf(stderr, "FAIL: stats v2 header struct_size=%llu version=%u\n",
                (unsigned long long)st2.struct_size, st2.version);
        failures++;
    }
    if (st2.frames_completed != st.frames_completed ||
        st2.packets_received != st.packets_received ||
        st2.packets_dropped != st.packets_dropped ||
        st2.fec_recovered_shards != st.fec_recovered_shards ||
        st2.bytes_received != st.bytes_received) {
        fprintf(stderr, "FAIL: stats v2 shared fields diverge from SlipstreamStats\n");
        failures++;
    }
    printf("stats v2: struct_size=%llu version=%u stale=%llu backpressure=%llu rejections=%llu\n",
           (unsigned long long)st2.struct_size, st2.version,
           (unsigned long long)st2.frames_stale_dropped,
           (unsigned long long)st2.frames_backpressure_dropped,
           (unsigned long long)st2.send_rejections);

    /* Guard rails: null out / undersized out_len / null session. */
    rc = slipstream_get_stats_v2(client, NULL, sizeof(st2));
    if (rc != SLIPSTREAM_STATUS_INVALID_ARG) {
        fprintf(stderr, "FAIL: stats v2 null out rc=%d want INVALID_ARG\n", (int)rc);
        failures++;
    }
    rc = slipstream_get_stats_v2(client, &st2, 15);
    if (rc != SLIPSTREAM_STATUS_INVALID_ARG) {
        fprintf(stderr, "FAIL: stats v2 out_len=15 rc=%d want INVALID_ARG\n", (int)rc);
        failures++;
    }
    rc = slipstream_get_stats_v2(NULL, &st2, sizeof(st2));
    if (rc != SLIPSTREAM_STATUS_NULL_POINTER) {
        fprintf(stderr, "FAIL: stats v2 null session rc=%d want NULL_POINTER\n", (int)rc);
        failures++;
    }

    free(buf);
    slipstream_session_free(host);
    slipstream_session_free(client);

    if (failures == 0) {
        printf("PASS: %d frames round-tripped byte-exact through lossy loopback\n", FRAMES);
        return 0;
    }
    fprintf(stderr, "FAILED with %d errors\n", failures);
    return 1;
}
