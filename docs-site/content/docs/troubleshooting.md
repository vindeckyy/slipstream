---
title: Troubleshooting
description: Common problems setting up or using a Slipstream host, and how to fix them.
---

## Another streaming host (Sunshine, Apollo, ...) is installed

Slipstream is a Moonlight-compatible host. So are **Sunshine** and its forks (**Apollo**,
**Vibeshine**, **Vibepollo**, **LuminalShine**, ...). Running one of them **at the same time** as
Slipstream is **not supported**: they bind the *same* GameStream ports (47984/47989 and
47998-48010, plus a web UI on 47990 that collides with Slipstream's management API), advertise the
*same* `_nvstream` mDNS name, and often install a *conflicting virtual-display driver*. The result
is `address already in use` errors, pairing that silently fails, the wrong host answering a client,
and capture/display glitches.

- Slipstream detects this automatically. It warns in the host's startup log, so it's on the web
  console's **Logs** page, and carries the finding in the status summary the management API
  serves. The tray icon doesn't flag it at all. To check on demand, run:

  ```sh
  slipstream-host detect-conflicts
  ```

  It lists any conflicting host found (installed or running) and exits non-zero if there is one.
- **Fix:** stop and uninstall the other host, then start Slipstream, e.g. stop the service
  (`sudo systemctl disable --now sunshine`) and uninstall it.
  If you only want to try Slipstream without removing the other host, at least make sure the other
  host is fully **stopped** first (they cannot both run at once).

## The host isn't found on the network

- Make sure the host is actually running: `systemctl --user status slipstream-host` (or you
  see it listening in the terminal).
- **On an Android phone or TV, check the app's local-network permission.** On Android 17 and newer,
  Android blocks Slipstream from touching anything on your LAN, discovery, the connect itself,
  Wake-on-LAN, the game library, until you allow it. The app asks when you open the host list, and
  a denial looks exactly like a host that isn't there. Tap **Allow...** under
  *Local network access is off* at the top of the host list, or enable **Nearby devices** for
  Slipstream in Android's app settings.
- Host and client must be on the **same network/subnet**. Discovery uses mDNS, which doesn't cross
  routed subnets or most VPNs-without-multicast. As a fallback, add the host by **IP address** in your
  client.
- A firewall on the host can block it. The native protocol needs **two** fixed UDP ports open:
  **9777** (the QUIC control plane) and **5353** (mDNS, this is the one discovery itself runs on).
  The packages ship a ready-made `slipstream-native` rule that opens both, plus TCP 47990 for
  the library API:

  ```sh
  sudo ufw allow slipstream-native                              # ufw (CachyOS, Ubuntu)
  sudo firewall-cmd --permanent --add-service=slipstream-native \
    && sudo firewall-cmd --reload                              # firewalld (Fedora, some Arch spins)
  ```

  The per-session **data plane** rides a *separate, random* UDP port and usually needs **no** firewall
  rule (see [Video is slow to start, or fails across
  subnets](#video-is-slow-to-start-or-fails-across-subnets) for why, and the one case where opening it
  helps). GameStream/Moonlight (only with `--gamestream`) uses TCP **47984/47989/48010** + UDP
  **47998/47999/48000** (video/FEC 47998, ENet control 47999, audio 48000) + mDNS UDP **5353**,
  that's the packages' `slipstream-gamestream` rule.

## The Linux host service won't start

`systemctl --user status slipstream-host` shows it failed instead of running. Two common causes:

- **There's no `host.env` yet.** The packaged unit reads `~/.config/slipstream/host.env` and won't
  start until that file exists, no package creates it, they only ship a template to copy:

  ```sh
  mkdir -p ~/.config/slipstream
  # /usr/share/slipstream/ on Fedora/Arch/Bazzite, /usr/share/slipstream-host/ on Ubuntu
  cp /usr/share/slipstream/host.env.example ~/.config/slipstream/host.env
  systemctl --user restart slipstream-host
  ```

  On Bazzite copy `host.env.bazzite` instead of `host.env.example`.
- **`status=203/EXEC`** instead means the unit that ran points at a binary that isn't there,
  usually an old hand-copied unit in `~/.config/systemd/user/` shadowing the packaged one and still
  aimed at a source checkout. Remove it, run `systemctl --user daemon-reload`, and start the
  packaged unit, see [Running as a
  Service](/docs/running-as-a-service#a-a-desktop-you-log-into).

## The host is asleep and won't wake

Clients wake a saved host by themselves, auto-wake is on by default, but only once they have seen
it awake, which is how they learn its MAC address, and only if the machine is armed to answer a magic
packet. The arming is what's usually missing, and the host tells you outright: search the web
console's **Logs** page for `Wake-on-LAN`, and the line either confirms the card is armed or names
the interface and the exact command to arm it. Then go to the BIOS/UEFI and network-card steps in
[Arming the machine](/docs/wake-on-lan#arming-the-machine).

## Video is slow to start, or fails across subnets

The native **data plane** (the raw UDP that carries video, separate from the 9777 control plane) uses
a **random, per-session UDP port**, the host binds `0.0.0.0:0`, then tells the client which port it
got during the connect handshake. There is no fixed data port.

Video flows host -> client, but the **client sends the first packet**: a small *hole-punch* datagram to
that port. This is deliberate. It lets the host learn the client's real (possibly NAT-translated)
source address and stream back to it, so a session can cross a NAT or a stateful inter-VLAN firewall
**without** a forwarded data port. What it means for a host firewall:

- **Same LAN, no host firewall (or the port allowed):** the punch arrives immediately and video starts
  at once. Nothing to configure.
- **Same LAN, host firewall that denies inbound** (ufw/nftables/firewalld default): the punch is
  dropped, so the host waits **~2.5 s**, then falls back to the address the client reported and streams
  anyway, a stateful firewall admits the return traffic because the host sent first. **Net effect: it
  works, but each session takes ~2.5 s longer to start.** That slow start is the symptom of a
  data-plane rule you're missing.
- **Across subnets / NAT:** the same punch-then-fallback applies, as long as the host's outbound video
  can reach the client (the path's stateful firewall then admits the return). If the host itself is
  behind NAT reached only via a forwarded control port, the data path may not establish, this is the
  case a fixed, forwardable data port would solve.

To remove the ~2.5 s fallback delay, **pin the data port** in [`host.env`](/docs/configuration) and
open exactly that one port. The host then binds that fixed port, skips the punch-wait, and streams
straight to the client, no timeout to pay:

```ini
# ~/.config/slipstream/host.env
SLIPSTREAM_DATA_PORT=9778
```

```sh
systemctl --user restart slipstream-host
sudo ufw allow 9778/udp                    # open exactly that one port
```

Running `serve` by hand instead? Pass `--data-port 9778` on that command line, but don't start one
alongside the service, which already holds these ports.

Two caveats. A fixed data port serves **one session at a time**; a second concurrent session finds it
busy and transparently falls back to a random port + hole-punch (logged). And `--data-port` streams
to the client's *reported* address, so use it only where that address is reachable, a flat LAN, or a
port-forward that doesn't remap the client's source. Leave it **off** (the default) to keep the
NAT-crossing hole-punch. On a normal single-LAN setup you can also just leave the data port closed and
accept the one-time ~2.5 s punch-timeout, or not run a host firewall on a trusted LAN at all.

## `nvidia-smi` says it can't communicate with the driver

- The NVIDIA kernel module didn't load. With **Secure Boot** enabled, enrol the module's signing key:
  `sudo mokutil --import /var/lib/shim-signed/mok/MOK.der`, reboot, **Enrol MOK** at the blue screen
  (or disable Secure Boot). On Fedora, follow RPM Fusion's Secure Boot steps.
- After a kernel update the module may need a rebuild, reinstall the driver package.

## The desktop won't start, or "GPU ... not supported by EGL"

The NVIDIA **GL/EGL userspace** is missing, the base driver package doesn't always include it.

- **Ubuntu:** `sudo apt install libnvidia-gl-<version>` (matching your driver).
- Confirm `/usr/share/glvnd/egl_vendor.d/10_nvidia.json` exists and `nvidia-drm modeset` is `Y`.

See [GNOME](/docs/gnome) for the GL/EGL userspace details.

## Black screen / no picture, but the client connects

- You must be on a **Wayland** session, not X11 (check the login-screen session picker).
- KWin must be **≥ 6.5.6** (`kwin_wayland --version`) for the *headless* appliance session
  (`kwin_wayland --virtual`); a normal Plasma 6 login needs no particular version, only the screencast
  grant. GNOME **≥ 48**; gamescope **≥ 3.16.22**. See [KDE](/docs/kde) for the KWin/Wayland
  requirement and [gamescope](/docs/gamescope) for the gamescope one.
- If [`host.env`](/docs/configuration) sets `SLIPSTREAM_COMPOSITOR`, **remove it**, the host
  auto-detects the live compositor, and the pin points it at one backend even when a different
  session is live (it also disables Gaming ↔ Desktop following).

## The screen stays black after switching to Game Mode (Nobara)

On distros whose Game Mode is display-manager autologin under **plasmalogin** (Nobara), a managed
takeover from a host **0.19.1 or older** could kill the display manager: it trips systemd's start
limit and the box stays black until someone restarts it. Recover from a VT (Ctrl+Alt+F3) or SSH:

```sh
systemctl --user unmask --runtime 'gamescope-session-plus@*.service'
sudo systemctl reset-failed plasmalogin && sudo systemctl restart plasmalogin
```

Current hosts detect the display-manager flavor and never mask the session unit there, see
[gamescope -> autologin display managers](/docs/gamescope) for the polkit rule that enables the full
managed takeover on these boxes (without it the host mirrors Game Mode instead).

## Session fails right after editing host.env

- Keys are **case-sensitive**: `slipstream_gamescope_attach=1` sets nothing, use the exact
  uppercase names.
- Hardcoded session anchors with the wrong uid (`XDG_RUNTIME_DIR=/run/user/1000` when `id -u`
  isn't 1000) point the host at another user's PipeWire/D-Bus: audio errors like
  `pw audio connect ... Creation failed`, no capture, and clients reporting the host as
  unreachable or asleep. **Delete both anchor lines**, a `systemctl --user` service doesn't need
  them, or fix the uid.
- `SLIPSTREAM_COMPOSITOR` pins the backend and disables Gaming ↔ Desktop following, remove it on
  any box that switches sessions.
- The env file is read at service start: `systemctl --user restart slipstream-host` after edits.

## Capture fails: "Session creation inhibited" (GNOME)

A **locked** GNOME session blocks screen capture. On an always-on/headless host, disable the lock:

```sh
gsettings set org.gnome.desktop.screensaver lock-enabled false
gsettings set org.gnome.desktop.session idle-delay 0
```

See [GNOME -> Headless session](/docs/gnome#headless-session) and
[Running as a Service](/docs/running-as-a-service).

## My mouse and keyboard are stuck in the stream

Nothing is broken, the stream captures them on purpose, from the moment it starts and again
whenever you click into it, so your keys and pointer go to the host instead of your own device.
**Ctrl+Alt+Shift+Q** hands them back where the client supports that chord; with a pad in your hands
**L1+R1+Start+Select** does the same on Steam Deck (and other clients that honour that chord).
Whatever you were holding down is released on the host, so nothing sticks. The rest of the in-stream
chords, switch mouse mode, disconnect, fullscreen, are in
[Getting your input back](/docs/input#getting-your-input-back).

## A controller is detected but games don't see it

The host user needs to be in the `input` group. On Bazzite:

```sh
ujust add-user-to-input-group
```

Then log out and back in. On other distros this is `sudo usermod -aG input $USER` + re-login. See
[Bazzite](/docs/bazzite).

## Copy and paste between host and client does nothing

The shared clipboard needs **two** separate switches on, and turning on only one looks exactly like
the feature not existing: the host operator has to allow it with `SLIPSTREAM_CLIPBOARD` in `host.env`
and restart the host, and you have to turn it on for that one host in your client's **Edit...** sheet
(where the client offers one). Work through
[Why the toggle does nothing](/docs/clipboard#why-the-toggle-does-nothing-or-is-greyed-out), it also
names the clients and host sessions where nothing crosses no matter what you set.

## Pairing is rejected / the client can't connect

- The host **requires pairing** by default. Arm pairing from the web console, then enter the PIN on
  the client. See [Pairing & Trust](/docs/pairing).
- If you re-installed the host, its identity changed, re-pair the client.

## Office / VPN

Connecting from an office laptop to a home host is the same product path as LAN streaming, with a
few networking twists. The full guides are [Desktop at work](/docs/desktop-at-work) and
[Network & VPN](/docs/network-and-vpn). Quick checks:

- **Host list is empty over VPN, but works on home Wi‑Fi.** mDNS usually does not cross a VPN.
  Add the host by its VPN IP (Tailscale / WireGuard / LAN address reachable through the tunnel).
- **Connect starts, then video stalls or never appears.** Confirm the VPN allows UDP to the host
  (native control **9777**, plus the negotiated media port). Overly strict ACLs that allow only TCP
  will break the stream. See [Video is slow to start, or fails across
  subnets](#video-is-slow-to-start-or-fails-across-subnets).
- **Mouse feels wrong for desk work.** Prefer **Desktop (absolute)** mouse where the client offers
  it (`Ctrl+Alt+Shift+M` where supported). gamescope / Gaming Mode hosts cannot take absolute mouse;
  use a full desktop session for office work ([Input](/docs/input#mouse-modes)).
- **Clipboard does nothing.** Both host `SLIPSTREAM_CLIPBOARD` and the per-host client switch must
  be on ([Clipboard](/docs/clipboard)).
- **Wake-on-LAN from the office does nothing.** WoL is a LAN broadcast; most VPNs will not deliver
  it. Leave the machine on, wake it before you leave, or use another wake path
  ([Wake-on-LAN](/docs/wake-on-lan)).
- **Do not port-forward** Slipstream to the public internet. Use the VPN
  ([Security](/docs/security)).

## Stutter, drops, or high latency

- Lower the **bitrate**. On a busy or Wi-Fi link, the requested bitrate may be too high, the native
  clients' [speed test](/docs/configuration#bitrate) picks a safe value; with Moonlight, set it
  manually.
- Prefer a **wired** connection or 5 GHz Wi-Fi between host and client.
- Streaming to **many devices at once** shares the GPU encoder. The host serves several
  concurrent native sessions (up to 4 by default); heavy load is usually bitrate-bound, so
  lower the bitrate first.

If the stream is *wrong* rather than late, a codec you didn't pick, 8-bit where you expected HDR,
4:2:0 where you asked for full chroma, the answer is usually that the host declined the request and
told your client so. [When the client and the host
disagree](/docs/client-settings#when-the-client-and-the-host-disagree) lists what it does with each
one.

## Still stuck?

Read the host's log around the failed connect or capture.

1. Open the web console's **Logs** page. It always holds the host's recent output at *debug* detail,
   whatever the log level is set to, there's nothing to switch on and no restart needed.
2. Filter it down to the level or the text you're after.
3. Use **Download logs** to save exactly what you're filtering on as a timestamped `.log` file you
   can attach to a bug report. The button beside it hands the same text to your phone or tablet's
   share sheet, or copies it to the clipboard on a desktop.

The same output also lands outside the console in the journal
(`journalctl --user -u slipstream-host`). Those *do* follow the log level: raise it with
`RUST_LOG=debug` in [`host.env`](/docs/configuration) and restart the host. `RUST_LOG=info` is
already the default, so setting it changes nothing.

For a performance problem rather than a failure, attach a **recording** instead of a log: see
[Recording a capture for a bug report](/docs/stats#recording-a-capture-for-a-bug-report).
