# slipstream host on a Steam Deck

Run a slipstream **host** on a Steam Deck — stream its Game Mode (or KDE desktop) *to* other devices.
(Streaming *to* a Deck is the client; use the Flatpak + [Decky plugin](../../clients/decky/) instead.)

User-facing guide: **docs-site → "Steam Deck (Host)"** (`docs-site/content/docs/steam-deck-host.md`).
This README is the deep reference for what the scripts do and how to operate them by hand.

## Why build on-device (not a package or prebuilt binary)

SteamOS 3 is an **immutable, read-only Arch** base:

- No `pacman -S` for system libs; `/usr` is read-only and reset on A/B updates.
- A **prebuilt binary is fragile** — it links the system FFmpeg/glibc, and a SteamOS update can bump
  those sonames out from under it (the same class of breakage as the NVIDIA-driver-after-update issue).
- The host needs **unsandboxed** `/dev/uinput` + `/dev/uhid`, PipeWire, the compositor, and VAAPI — so
  Flatpak (the normal Deck app channel) doesn't fit. Flatpak/Decky are for the *client*.

So the host is built **natively inside a Debian-trixie distrobox** (`pf2`), chosen because its
FFmpeg/glibc ABI matches SteamOS's — the resulting binary runs **natively on SteamOS** (the container
is only the build environment; `slipstream-host` is launched directly, not via `distrobox enter`). A
rebuild always matches the running OS. Encode is **VAAPI** on the Deck's AMD GPU (NVENC on NVIDIA),
auto-selected by `SLIPSTREAM_ENCODER=auto`.

The web console is the one part that stays in the container at runtime: it's a Nitro/Node server run
by `bun`, so its service does `distrobox enter pf2 -- … bun run .output/server/index.mjs`.

## Scripts

| Script | What it does |
|--------|--------------|
| `install.sh` | Idempotent installer: ensure the `pf2` distrobox + toolchain → build host (+web) → write config → tune sysctl + `input` group (sudo) → install + start `slipstream-host` / `slipstream-web` systemd **user** services with linger. |
| `update.sh` | Rebuild from the current source and restart the services (config + pairings persist). `--pull` does `git pull` first. |

```sh
git clone https://github.com/vindeckyy/slipstream.git ~/slipstream
bash ~/slipstream/scripts/steamdeck/install.sh            # PIN pairing required (secure default)
bash ~/slipstream/scripts/steamdeck/install.sh --open     # trusted LAN: accept unpaired clients
bash ~/slipstream/scripts/steamdeck/install.sh --no-web   # host only, no web console
bash ~/slipstream/scripts/steamdeck/update.sh             # after pulling new source
```

Env overrides: `SLIPSTREAM_SRC` (source dir, default `~/slipstream`), `SLIPSTREAM_BOX` (container name,
default `pf2`), `SLIPSTREAM_MGMT_PORT` (47990), `SLIPSTREAM_WEB_PORT` (3000).

## What gets installed

- **Binary:** `~/slipstream/target-steamos/release/slipstream-host` (built in `pf2`, run natively).
- **Config:** `~/.config/slipstream/host.env` (encoder/compositor) and `web.env` (generated web login
  password + session secret). Trust material (`cert.pem`, `mgmt-token`, `slipstream1-paired.json`) lives
  here too and persists across updates.
- **Services:** `~/.config/systemd/user/slipstream-host.service` (runs `serve --native --mgmt-bind
  0.0.0.0:47990`, `+ --open` if chosen) and `slipstream-web.service`. Linger is enabled so they run
  without a login session.
- **System tuning (sudo):** `/etc/sysctl.d/99-slipstream-net.conf` (32 MB UDP buffers — the #1
  high-bitrate lever), `/etc/udev/rules.d/60-slipstream.rules`, and `$USER` in the `input` group.

## Operating

```sh
systemctl --user status  slipstream-host slipstream-web
journalctl --user -u slipstream-host -f          # watch sessions / pairing PIN
systemctl --user restart slipstream-host         # after editing host.env
```

Pair from the web console (Devices → arm pairing) or directly from a client with the host's PIN. The
host advertises over mDNS as `_slipstream._udp`, so clients discover it automatically.

## Gotchas

- **distrobox required.** If missing: `curl -sfL https://raw.githubusercontent.com/89luca89/distrobox/main/install | sh -s -- --prefix ~/.local` (then ensure `~/.local/bin` is on PATH).
- **First build is slow** (~10–15 min + ~1 GB toolchain/image). Incremental afterwards.
- **No passwordless sudo** → the installer skips the sysctl/udev/input steps with a warning; high
  bitrates will drop packets until you apply `99-slipstream-net.conf` and join `input` yourself.
- **Game Mode auto-suspend** drops the host off the network on idle — disable it (Settings → Power)
  for a headless host.
- **WiFi tx ceiling** ≈ 250 Mbps goodput (a Deck hardware/driver packet-rate limit, band-independent);
  fine for 1080p/1440p60. A wired dock lifts it.
- **After a major SteamOS update**, if the host won't start, run `update.sh` to rebuild against the new
  base libraries.
