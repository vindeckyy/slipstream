---
title: Network & VPN
description: Reach a Slipstream host from another network safely — trusted LAN model, Tailscale and WireGuard patterns, discovery, ports, firewalls, and what does not work over a VPN.
---

Slipstream is built for a **trusted private network**: your home LAN, or a VPN that makes the
client look local. It is **not** hardened for direct exposure to the public internet. This page is
the networking companion to [Security & Safe Use](/docs/security).

If you are connecting from an office laptop to a home desktop, read this together with
[Desktop at work](/docs/desktop-at-work).

## The model in one paragraph

Host and client must be able to talk privately. On a home Wi‑Fi that is automatic. From another
site, you put both ends on a **private overlay** (Tailscale, WireGuard, router VPN, corporate VPN
that routes to your LAN) and you **never** open Slipstream's ports on the WAN side of your router.

## Do

- Keep the host on a network you trust.
- Use a VPN when you leave home.
- Prefer **native** Slipstream clients over the public internet path of a VPN.
- Add the host **by IP** when mDNS discovery cannot cross the VPN.
- Open only the ports you need on the **host firewall**, on the private interface.

## Do not

- Port-forward TCP/UDP Slipstream ports to `0.0.0.0` on the public internet.
- Rely on “security through obscurity” with a random high port on the WAN.
- Leave GameStream/Moonlight enabled on a host you only use over a wide VPN if you do not need it —
  that plane pairs over legacy plain HTTP ([Security](/docs/security#gamestream--moonlight-compatibility-is-the-weak-crypto-path)).

## Common VPN patterns

### Tailscale (or similar mesh VPN)

1. Install Tailscale on the **host** and on the **office client**.
2. Sign both into the same tailnet; confirm each has a Tailscale IP (`100.x` / IPv6).
3. On the client, add the host by its **Tailscale IP** if it does not appear automatically.
4. Pair once over the VPN the same way you would on LAN.

**Notes**

- mDNS often **does not** cross Tailscale. Manual IP is normal, not a failure.
- MagicDNS names can work as the host address if your client accepts a hostname when adding a host.
- Tailscale ACLs must allow the ports below between client and host.

### WireGuard (or router site-to-site / road-warrior VPN)

1. Host stays on the home LAN with a stable address (DHCP reservation or static).
2. Office client joins the WireGuard network and can route (or is bridged) to that LAN subnet —
   **or** the host is also a WireGuard peer with its own WG address.
3. From the client, add the host by the address the VPN makes reachable.
4. Pair and stream.

**Notes**

- “Full tunnel” vs “split tunnel” both work; split tunnel is usually enough if it includes the host
  subnet or WG peer IPs.
- Some router VPNs put you on a **different subnet** than the host. That is fine for Slipstream as
  long as routed unicast works; discovery may still need add-by-IP.

### Corporate VPN into a lab or desk machine

Only use this when policy allows a streaming host on that machine. Prefer a **dedicated** workstation
over a laptop full of credentials. Pairing and console passwords still matter; the VPN is not a
substitute for PIN pairing.

## Discovery across a VPN

Discovery uses **mDNS** (`_slipstream._udp` for native, `_nvstream._tcp` when GameStream is on).
Multicast usually **stops at the subnet boundary**. Symptoms:

- Host list is empty over VPN.
- The same host appears instantly on home Wi‑Fi.

**Fix:** in the client, **Add host** and enter the VPN IP (or DNS name that resolves on the VPN).
After pairing, the client remembers the host. You can also confirm reachability with
[`slipstream` / host CLI helpers](/docs/host-cli) where available.

Disable mDNS on the host only if you must (`--no-mdns` / `SLIPSTREAM_MDNS=0`); then every client
must add the host manually.

## Ports you care about

Exact numbers matter for firewalls and VPN ACLs.

### Native Slipstream (`slipstream/1`) — always needed for native clients

| Port | Proto | Role |
|---|---|---|
| **9777** | UDP | QUIC control plane (fixed) |
| **5353** | UDP | mDNS discovery (optional if you always add-by-IP) |
| **47990** | TCP | Management / library API (HTTPS + mTLS for paired clients) |
| *ephemeral* | UDP | Per-session media data plane (negotiated; often needs no static rule) |

Linux packages ship a `slipstream-native` firewall service for the fixed native ports.

### Web console (optional but common)

| Port | Proto | Role |
|---|---|---|
| **47992** | TCP | Browser console HTTPS |

Open this on the private network if you pair or administer from another device. You can keep it
loopback-only if you only ever use the console on the host itself.

### GameStream / Moonlight (only if enabled)

| Ports | Proto | Role |
|---|---|---|
| **47984**, **47989**, **48010** | TCP | nvhttp / RTSP control |
| **47998–48000** | UDP | Video / control / audio |
| **5353** | UDP | `_nvstream` mDNS |

Linux packages: `slipstream-gamestream`. Prefer leaving this **off** for office-only hosts.

## Firewalls

- **Linux:** use the packaged `slipstream-native` (and optionally `slipstream-gamestream`,
  `slipstream-web`) services with `firewalld` or `ufw` — your distro install page has the commands.
- **Windows:** the installer opens streaming and console ports on **Private** and **Domain**
  profiles only. A home LAN mis-marked **Public** looks like “nothing connects.” Set the network to
  Private, or see [Troubleshooting](/docs/troubleshooting#the-host-isnt-found-on-the-network).

VPN interfaces must be treated as private/trusted by the OS firewall the same way your LAN is.

## Latency and bandwidth expectations

Office VPNs vary widely.

- **LAN:** you can push high bitrate, high refresh, 4:4:4, HDR.
- **Good home uplink + light VPN:** 1080p–1440p desktop work is realistic; raise bitrate until text
  looks crisp, then stop.
- **Congested VPN:** drop refresh, resolution, or chroma before you assume the host is broken.
- **Wired LAN only:** [PyroWave](/docs/pyrowave) for ultra-low-latency / 4:4:4 — not a WAN codec.

Use the [stats overlay](/docs/stats) (`Ctrl+Alt+Shift+S`) to see whether you are network-bound or
decode-bound.

## Wake-on-LAN and VPNs

[Wake-on-LAN](/docs/wake-on-lan) is a layer-2 broadcast magic packet. Most VPNs **do not** deliver it
to a sleeping host on another site.

Practical options:

- Leave the workstation on (or sleeping with another wake path your router supports).
- Wake it before you leave home.
- Use a smart plug / BMC / vendor wake feature outside Slipstream, then connect.

## Competing hosts on the same ports

Do not run Sunshine, Apollo, or other GameStream hosts **at the same time** as Slipstream on one
machine — shared ports and often shared virtual-display drivers. Slipstream warns when another host
is **actively running**. Details:
[Troubleshooting](/docs/troubleshooting#another-streaming-host-sunshine-apollo--is-installed).

## Related pages

- [Desktop at work](/docs/desktop-at-work)
- [Security & Safe Use](/docs/security)
- [Requirements](/docs/requirements)
- [Troubleshooting → Office / VPN](/docs/troubleshooting#office--vpn)
- [Host CLI](/docs/host-cli)
