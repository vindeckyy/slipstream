---
title: Network & VPN
description: Reach a Slipstream host from another network safely - trusted LAN model, Tailscale and WireGuard patterns, discovery, ports, firewalls, and what does not work over a VPN.
---

Slipstream is built for a **trusted private network**: your home LAN, or a VPN that makes the
client look local. It is **not** hardened for direct exposure to the public internet. This page is
the networking companion to [Security & Safe Use](/docs/security).

If you are connecting from an office laptop to a home desktop, read this together with
[Desktop at work](/docs/desktop-at-work). For couch / LAN play on the same network, you usually
need only the firewall section - see [Play](/docs/play).

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
- Rely on "security through obscurity" with a random high port on the WAN.
- Leave GameStream/Moonlight enabled on a host you only use over a wide VPN if you do not need it -
  that plane pairs over legacy plain HTTP ([Security](/docs/security#gamestream--moonlight-compatibility-is-the-weak-crypto-path),
  [Moonlight](/docs/moonlight)).

## Common VPN patterns

### Tailscale (or similar mesh VPN)

Step-by-step that matches how people actually get a first office stream:

1. Install Tailscale on the **host** PC and on the **office client**.
2. Sign both into the **same tailnet**. On each machine, confirm a Tailscale IP appears
   (`100.x.y.z` and/or a Tailscale IPv6 address). On Linux: `tailscale ip -4`. On phone clients,
   use the Tailscale app's machine details.
3. From the office client, **ping the host's Tailscale IP** (or use Tailscale's peer status). If
   that fails, fix Tailscale ACLs / key expiry / offline peer before touching Slipstream.
4. On the host firewall, treat the Tailscale interface like a private LAN: the packaged
   `slipstream-native` rules (below) must allow traffic that arrives on that interface. Some
   people temporarily disable the host firewall only to prove the path, then re-enable with the
   correct service rules - do not leave the firewall permanently off on a dual-homed box.
5. Open the Slipstream client. The host list is often **empty** over Tailscale (mDNS does not
   cross). Use **Add host** and enter the **Tailscale IP** (or a MagicDNS name if your client
   accepts a hostname).
6. [Pair once](/docs/pairing) over the VPN the same way you would on LAN - arm a PIN in the
   [web console](/docs/web-console) (reachable at `https://<tailscale-ip>:47992` if you opened the
   console port), or approve a waiting device.
7. Connect. If control connects but video stalls, your ACLs are probably allowing TCP-ish paths
   but not **UDP** to **9777** and the negotiated media port - see
   [Office / VPN](/docs/troubleshooting#office--vpn).

**Notes**

- mDNS often **does not** cross Tailscale. Manual IP is normal, not a failure.
- MagicDNS names can work as the host address if your client accepts a hostname when adding a host.
- Tailscale **ACLs** must allow the ports below between client and host (UDP **9777** at minimum
  for native control; TCP **47990** if paired clients browse the library API; TCP **47992** if you
  administer the console remotely; GameStream ports only if that plane is on).
- Exit nodes and subnet routers are optional. A plain peer-to-peer Tailscale link between office
  laptop and home host is enough.

### WireGuard (or router site-to-site / road-warrior VPN)

1. Give the host a **stable address** on the home LAN (DHCP reservation or static) *or* make the
   host itself a WireGuard peer with a fixed WG address.
2. Configure the office client as a road-warrior peer (or join a site-to-site tunnel that routes
   to the home LAN subnet). Confirm `AllowedIPs` (or the router's equivalent) includes the host's
   address or subnet.
3. Bring the tunnel up. From the client, **ping** the host address the VPN makes reachable. If
   ping fails, Slipstream will not magically route around a broken tunnel.
4. Open host firewall ports on the interface that receives VPN traffic (LAN NIC for site-to-site,
   or the WG interface when the host is a peer).
5. In the Slipstream client, **Add host** by that reachable IP. Pair and stream.

**Notes**

- "Full tunnel" vs "split tunnel" both work; split tunnel is usually enough if it includes the host
  subnet or WG peer IPs - and it avoids sending all office browsing through your home uplink.
- Some router VPNs put you on a **different subnet** than the host. That is fine for Slipstream as
  long as routed unicast works; discovery may still need add-by-IP.
- Keep MTU sensible. Pathologically low or fragmented UDP paths show up as high latency or stalled
  video; check the [stats overlay](/docs/stats) before retuning codecs.

### Corporate VPN into a lab or desk machine

Only use this when policy allows a streaming host on that machine. Prefer a **dedicated** workstation
over a laptop full of credentials. Pairing and console passwords still matter; the VPN is not a
substitute for PIN pairing.

Extra caveats that bite people on corp tunnels:

- **UDP is often restricted.** Native Slipstream needs UDP for control (**9777**) and media. A
  "VPN that only forwards HTTPS" will not carry a stream. Test with a native client after ping
  works; TCP-only tools lying green does not prove the path.
- **Split DNS / captive portal helpers** can make the host hostname resolve to a wrong address.
  Prefer the literal VPN or LAN IP when adding the host.
- **Mandatory proxies** on the client OS usually do not apply to Slipstream's UDP path - but they
  can break fetching the web console in a browser. Use the host's VPN IP with HTTPS on **47992**,
  or administer from a machine that is actually on the host LAN.
- **Do not** enable GameStream/Moonlight on a Work host that sits on a shared corporate segment
  unless you fully trust that LAN - see [Moonlight](/docs/moonlight) and
  [Desktop at work](/docs/desktop-at-work).
- IT policy may forbid always-on remote-control hosts. That is an org decision Slipstream cannot
  override; keep the blast radius small ([Security → which machine](/docs/security#choosing-which-machine-to-host-on)).

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

## Host not found - decision tree

Work top to bottom. Most "VPN is broken" reports stop at step 3 or 5.

1. **Is the host process running?**  
   `systemctl --user status slipstream-host`. If it is down, fix that first
   ([Troubleshooting](/docs/troubleshooting#the-linux-host-service-wont-start)).

2. **Are you on the same path you think you are?**  
   On LAN: same Wi‑Fi/subnet. On VPN: client shows connected, and you can ping the host's VPN or
   LAN IP. No ping → fix the overlay, not Slipstream.

3. **Empty list only over VPN?**  
   Expected when mDNS does not cross. **Add host by IP.** Saved hosts from a prior pair still
   work even when discovery is empty.

4. **Android client?**  
   Local-network / Nearby devices permission denied looks exactly like "no hosts." Allow it -
   [Troubleshooting](/docs/troubleshooting#the-host-isnt-found-on-the-network).

5. **Host firewall open for native ports?**  
   UDP **9777** + **5353**, TCP **47990** via `slipstream-native` (commands below). Without
   **9777**, nothing connects even if you typed the IP correctly.

6. **Competing GameStream host running?**  
   Sunshine / Apollo / forks binding the same ports -
   `slipstream-host detect-conflicts`, then stop the other host.

7. **Still stuck?**  
   Full checklist: [The host isn't found on the network](/docs/troubleshooting#the-host-isnt-found-on-the-network).
   Office-specific: [Office / VPN](/docs/troubleshooting#office--vpn).

## Ports you care about

Exact numbers matter for firewalls and VPN ACLs.

### Native Slipstream (`slipstream/1`) - always needed for native clients

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
| **47998-48000** | UDP | Video / control / audio |
| **5353** | UDP | `_nvstream` mDNS |

Linux packages: `slipstream-gamestream`. Prefer leaving this **off** for office-only hosts.

### Data plane ports

The native **media** path is separate from UDP **9777**. By default the host binds a **random
per-session UDP port** and tells the client during connect. The client sends a small hole-punch
first so video can cross NATs and stateful firewalls without a forwarded data port.

What you will see in practice:

- **Same LAN, no host firewall (or port allowed):** video starts immediately.
- **Host firewall denies inbound to the random port:** session still works after ~**2.5 s**
  (punch timeout, then fallback). Slow start is the symptom of a missing data-plane allow.
- **Across a VPN / subnet:** same punch-then-fallback, as long as host → client UDP can flow.
  Strict ACLs that allow only TCP break the picture even when "connect" seemed to start.

To remove the punch wait on a controlled path, pin a port in `host.env` and open **exactly** that
UDP port:

```ini
SLIPSTREAM_DATA_PORT=9778
```

```sh
systemctl --user restart slipstream-host
sudo ufw allow 9778/udp   # or the firewalld equivalent for that port
```

Caveats (do not skip): a fixed data port serves **one session at a time** (a second concurrent
session falls back to random + punch); leave the pin **off** when you rely on NAT-crossing
hole-punch. Full write-up:
[Troubleshooting → Video is slow to start](/docs/troubleshooting#video-is-slow-to-start-or-fails-across-subnets).

**Never** port-forward that data port (or 9777) to the public WAN - use a VPN
([Security](/docs/security)).

## Firewalls - copy-paste

Open only what you use. A Work host that never serves Moonlight can skip `slipstream-gamestream`.

### ufw (Ubuntu, CachyOS, many Arch setups)

```sh
sudo ufw allow slipstream-native
# optional - browser console from another device on the private network:
sudo ufw allow slipstream-web
# only if the host runs with --gamestream / Moonlight clients:
sudo ufw allow slipstream-gamestream
sudo ufw status
```

### firewalld (Fedora, some Arch spins, Bazzite-style hosts)

```sh
sudo firewall-cmd --reload   # load packaged service definitions if freshly installed
sudo firewall-cmd --permanent --add-service=slipstream-native
sudo firewall-cmd --permanent --add-service=slipstream-web          # optional
sudo firewall-cmd --permanent --add-service=slipstream-gamestream   # only if needed
sudo firewall-cmd --reload
sudo firewall-cmd --list-services
```

`slipstream-native` opens UDP **9777**, UDP **5353**, and TCP **47990**. `slipstream-web` is TCP
**47992**. `slipstream-gamestream` is the Moonlight port set above.

## Latency and bandwidth expectations

Office VPNs vary widely.

- **LAN:** you can push high bitrate, high refresh, 4:4:4, HDR -
  [Picture quality](/docs/picture-quality), [Play](/docs/play).
- **Good home uplink + light VPN:** 1080p-1440p desktop work is realistic; raise bitrate until text
  looks crisp, then stop - [Desktop at work](/docs/desktop-at-work),
  [Picture quality → Soft text](/docs/picture-quality#soft-text-diagnosis).
- **Congested VPN:** drop refresh, resolution, or chroma before you assume the host is broken.
- **Wired LAN only:** [PyroWave](/docs/pyrowave) for ultra-low-latency / 4:4:4 - not a WAN codec.

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
machine - shared ports and often shared virtual-display drivers. Slipstream warns when another host
is **actively running**. Details:
[Troubleshooting](/docs/troubleshooting#another-streaming-host-sunshine-apollo--is-installed).

## Related pages

- [Desktop at work](/docs/desktop-at-work)
- [Play](/docs/play)
- [Picture quality](/docs/picture-quality)
- [Pairing & Trust](/docs/pairing)
- [Connect with Moonlight](/docs/moonlight)
- [Security & Safe Use](/docs/security)
- [Requirements](/docs/requirements)
- [Troubleshooting → Office / VPN](/docs/troubleshooting#office--vpn)
- [Host CLI](/docs/host-cli)
