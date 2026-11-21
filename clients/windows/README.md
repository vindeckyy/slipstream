# slipstream-windows

The native Windows session client. **Not** StreamLight — that is a third-party Moonlight client
(with an entirely different protocol) and has nothing to do with this crate. This speaks
`slipstream/1` to `slipstream-host`.

## What works today

| Area | State |
|---|---|
| LAN discovery (`_slipstream._udp`, port 9777) | works — `--discover` |
| Manual host entry `ip:port` (port defaults to 9777) | works |
| Identity / pairing persistence (`%APPDATA%\slipstream\client-{cert,key}.pem`) | works, shared with the other clients |
| PIN pairing ceremony (`--pair PIN`) | works (same `NativeClient::pair` the probe uses) |
| QUIC connect, Hello/Welcome negotiation, `Start` | works |
| Video plane receive (every access unit, `--record FILE` writes a playable `.h265/.h264/.av1`) | works |
| Admission / connection state machine (`HostAdvert`, admission table, phases) | implemented and unit-tested |
| Keyboard/mouse capture, window, hardware decode (D3D11VA) | **not yet** — see `PR.md` |

The window and the decode backend attach at the two traits in `src/present.rs` (`Decoder`,
`Presenter`). Until they land, the binary is a full transport client with a headless presenter:
it proves the protocol path on Windows and records the stream.

## Build

Requires Rust 1.96 (pinned by `rust-toolchain.toml` in the repo root) and the MSVC toolchain.

`audiopus_sys` builds libopus through CMake. On this box the default generator (VS 2026) and the
CMake 4 policy floor both need overriding, otherwise the build fails in `audiopus_sys`:

```bat
set "CMAKE_GENERATOR=Visual Studio 17 2022"
set "CMAKE_POLICY_VERSION_MINIMUM=3.5"
cargo build --release -p slipstream-windows
```

Binary: `target\release\slipstream-windows.exe`.

## Run

```bat
:: discover hosts on the LAN, then exit
slipstream-windows --discover
slipstream-windows --discover 8

:: pair once (reads the PIN the host prints when pairing is armed)
slipstream-windows 100.112.143.99:9777 --pair 1234
:: -> PAIRED — host fingerprint <hex>   (use it as --pin from now on)

:: stream for 12 s and record the elementary stream
slipstream-windows 100.112.143.99:9777 --pin <hex> --seconds 12 --record out.h265

:: unpinned: trust-on-first-use, the observed fingerprint is logged for pinning
slipstream-windows 192.0.2.10:9777 --seconds 30
```

Flags: `--connect HOST[:PORT]` (or a bare positional), `--pin HEX`, `--pair PIN`,
`--discover [SECS]`, `--name LABEL`, `--mode WxHxFPS` (default `1920x1080x144`),
`--bitrate KBPS`, `--codec auto|h264|hevc|av1`, `--audio-channels 2|6|8`, `--seconds N`,
`--record FILE`. Exit code `3` means the session was admitted but the host published zero frames.

`RUST_LOG=debug` for the transport log.

## Test

```bat
cargo test -p slipstream-windows
```

30 pure-logic tests cover argument parsing, `host:port`/IPv6 handling, TXT advert decoding
(fail-closed pairing policy), the admission table, and every connection phase transition.

## Layout

- `src/args.rs` — flag parsing, `Target` (host/port), fingerprint hex.
- `src/connect.rs` — `HostAdvert`, `PairingPolicy`, `Admission`, `ConnectOutcome`, `Machine`.
- `src/discovery.rs` — mDNS browse + console rows.
- `src/identity.rs` — cert/key persistence.
- `src/present.rs` — `Decoder`/`Presenter` seam, test pattern, counting presenter.
- `src/main.rs` — dispatch, pair, connect, receive loop.
