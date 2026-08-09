---
title: Network
description: Ports, firewalls, LAN and VPN. Trusted networks only.
---

Slipstream is remote control of the host. Use a **trusted LAN** or a **private VPN**. Do not
port-forward Slipstream ports to the public internet.

## Ports

### Native (`slipstream/1`)

| Port | Proto | Use |
|------|-------|-----|
| **9777** | UDP | Control |
| **5353** | UDP | mDNS discovery |
| **47990** | TCP | Management / library API (HTTPS; paired clients) |
| *(session)* | UDP | Media (negotiated; optional pin below) |

Packages ship a `slipstream-native` firewall helper for the fixed native ports.

To pin the media port (one session at a time) and avoid the hole-punch wait:

```sh
# host.env
SLIPSTREAM_DATA_PORT=9778
```

Open that UDP port on the host firewall.

### Console

| Port | Proto | Use |
|------|-------|-----|
| **47992** | TCP | Browser console HTTPS (`slipstream-web`) |

### GameStream (optional)

| Ports | Proto | Use |
|-------|-------|-----|
| **47984**, **47989**, **48010** | TCP | nvhttp / RTSP |
| **47998-48000** | UDP | Video / control / audio |

Open these only if `--gamestream` is enabled. Prefer leaving GameStream **off** on VPN-only / work hosts.

## Firewall

On the host, allow the ports you use on the private interface (LAN or VPN). Linux packages provide
`slipstream-native`, `slipstream-web`, and `slipstream-gamestream` firewall services for the sets above.

## VPN (Tailscale / WireGuard)

1. Prove the stream on the home LAN first.
2. Put host and client on the same VPN.
3. Add the host by **VPN IP** when mDNS is empty.
4. Allow at least UDP **9777** between client and host. Add TCP **47990** for library, TCP **47992** for console, and GameStream ports only if needed.
5. Prefer the **native** client over Moonlight across a VPN.

Do not rely on WAN port-forwarding or "security through obscurity."

## Conflicts

Another GameStream host (Sunshine, Apollo, ...) on the same machine shares these ports. Run
`slipstream-host detect-conflicts` and stop the other host before debugging connectivity.
