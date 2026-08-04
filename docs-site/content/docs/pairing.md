---
title: Pairing & Trust
description: How a client and host establish trust, PIN pairing once, pinned reconnects after.
---

Slipstream has no accounts and no cloud. Trust is established directly between a client and a host,
on your network, with a one-time pairing, either an **approval click in the host's
[web console](/docs/web-console)** or a **PIN ceremony**. After that, the device reconnects
automatically on a pinned cryptographic identity.

## How it works

- Each host has a stable **identity** (a certificate). Clients remember its fingerprint, so they know
  they're talking to the same host next time.
- The first time a client connects, you **pair** it: with the native protocol the **host** shows a
  short **4-digit PIN** and you type it into the client. (With Moonlight it runs the other way round
, Moonlight shows the PIN and you type it into the host's console.) Either way a secure exchange
  (SPAKE2) binds the two identities, and an attacker who doesn't know the PIN gets a single online
  guess, no offline cracking.
- After pairing, the host stores the client's identity in its allow-list, and the client stores the
  host's fingerprint. Reconnects are automatic, no PIN. Having seen the host on your network while
  it was awake also teaches the client its MAC address, so a later connect can
  [wake it from sleep](/docs/wake-on-lan).

## Native vs GameStream pairing

Two planes, two ceremonies. Do not mix up which UI shows the PIN.

| | **Native (`slipstream/1`)** | **GameStream / Moonlight** |
|---|---|---|
| When available | Always (native plane is always on) | Only if the host runs with `--gamestream` |
| Who shows the PIN | **Host** (console **Pair a device**) | **Moonlight** client |
| Who types the PIN | Native Slipstream client (or CLI) | Host console **Moonlight (GameStream) pairing** card |
| Arming required? | Yes - arm pairing in the console (2‑minute window) | No arming; submit the PIN Moonlight shows |
| Console card visible? | Always (Pairing page) | Only when GameStream plane is running |
| Crypto posture | SPAKE2 binding + pinned identity on the native plane | Legacy GameStream pairing over **plain HTTP**; weaker control encryption - trusted LAN only |
| Office / VPN recommendation | **Prefer this** | Leave **off** on Work-oriented hosts unless you need Moonlight |

Native clients: [Clients](/docs/clients). Moonlight path: [Connect with Moonlight](/docs/moonlight).
Why GameStream is the weak-crypto path:
[Security](/docs/security#gamestream--moonlight-compatibility-is-the-weak-crypto-path).
Work checklist: [Desktop at work](/docs/desktop-at-work).

## Approving a device from the console (no PIN)

The fastest way to admit a new device: just **try to connect** from it. On a pairing-required host,
the attempt shows up in the web console's Pairing page under **Waiting for approval**, with the
device's name and identity fingerprint. Click **Approve** (and optionally give it a label like
"Living Room TV"), and the device is paired on the spot: its next connect goes straight through. No
PIN to read or type.

**Deny** just dismisses the request (the device can knock again later, it's "not now", not a
blocklist). Requests expire on their own after **10 minutes**.

This works because approval happens on the host's authenticated management surface, only someone
with console access can admit a device.

## Pairing with a PIN

PIN pairing is the **default and required** path for any new host: unless the host has explicitly
opted into trust-on-first-use (see below), a client connecting to an unknown host must complete the
PIN ceremony, or be approved from the console, as above, before it can stream. It's the right path
for the *first* device (before the console has admitted anything) or when you're at the client and
the console isn't handy.

Pairing has to be **armed** on the host before a client can pair (so a random device can't pair
itself). On the production host (`serve`), this is done from the **web console**: open the
host's management console, click to arm pairing, and the host displays a 4-digit PIN along with the
list of paired devices. This works on a headless host over the network, there is no command-line flag
to arm pairing on `serve`.

The armed window lasts **2 minutes**, the console counts it down under the PIN and offers a
**Cancel** button. Arm it once you're standing at the device; if it lapses, just arm it again.

Pairing from the console needs the console running. On Linux that's the separate `slipstream-web`
systemd user unit, which you enable once, see [The Web Console](/docs/web-console).

Then, on the client:

- **[Native clients](/docs/clients) (Apple, Linux, Windows, Android):** select the host (or use
  *Pair with PIN...* from its menu) and enter the PIN the host displays.
- **[Steam Deck](/docs/steam-deck) (the Decky plugin):** open Slipstream from the Quick Access menu
  and pick the host, an unpaired one's button reads **Pair & Stream**. Enter the PIN on the
  4-digit pad it opens.
- **[Moonlight](/docs/moonlight):** choose **Pair**; Moonlight shows a 4-digit PIN, and you type
  that PIN into the console's **Moonlight (GameStream) pairing** card and press **Submit PIN**.
  (This direction is the reverse of the native flow, and arming doesn't apply to it.)

A link can't stand in for any of this. A
[`slipstream://` link](/docs/profiles-and-links#what-a-link-can-and-cant-do) starts a stream on a
host this device already trusts; `slipstream://pair/...` is refused outright, and a link naming a host
you've never paired with can at most open the app's own trust prompt.

### Moonlight PIN conditions

GameStream pairing only works when all of these are true:

1. The host was started **with GameStream enabled** (`serve --gamestream`, or the packaged unit /
   installer option that turns it on). Bare `serve` has no Moonlight pairing card.
2. The **web console** is running and reachable - it is the only UI where a Moonlight PIN can be
   entered ([Web console](/docs/web-console)).
3. Moonlight can reach the GameStream ports (firewall `slipstream-gamestream`, and no competing
   Sunshine/Apollo process) - [Moonlight](/docs/moonlight),
   [Network & VPN](/docs/network-and-vpn).
4. You start **Pair** in Moonlight **while** you are ready to type the PIN into the console; there
   is no separate "arm" step, but the waiting-client state is still time-bounded by the Moonlight /
   host handshake.

If the Moonlight card never appears in the console, GameStream is off on that host - that is
expected on stock Windows installs and on Linux hosts you deliberately hardened for
[Desktop at work](/docs/desktop-at-work).

### Pairing from a terminal

On Linux the client package also installs `slipstream`, a headless CLI. Arm pairing in the console,
read the PIN, then run:

```sh
slipstream pair 192.168.1.50 --pin 1234 --name "Living Room"
```

It prints `paired <addr>:<port> fp=<fingerprint>` and saves the host in the same store the desktop
client uses, so later connects are silent. `--name` is the label the host files this device under
(default: this machine's name), and the port defaults to **9777**, write `host:port` to use another
one. Without `--pin` the command asks for one; in a script with no terminal it exits **6** rather
than hanging, and exit **3** means the host refused or the PIN was wrong.

The GTK client can do the same thing without opening a window:

```sh
slipstream-client --connect 192.168.1.50:9777 --pair 1234 --name "Living Room"
```

Over a VPN, substitute the Tailscale / WireGuard IP for `192.168.1.50` - same ceremony
([Network & VPN](/docs/network-and-vpn)).

## Pairing over a VPN

Pairing does not require multicast. As long as the client can reach the host's control port
(native UDP **9777**, or GameStream ports for Moonlight) on the private overlay, the ceremony is
the same as on LAN.

Practical tips:

- **Add the host by VPN IP** first if discovery is empty - normal over Tailscale / WireGuard.
- Open the [web console](/docs/web-console) at `https://<vpn-ip>:47992` to arm native pairing or
  submit a Moonlight PIN (firewall: `slipstream-web`).
- Prefer **native** pairing on office paths; keep GameStream off on Work hosts when you can.
- Pair once from the office laptop you will actually use; a second device (tablet, phone) is a
  separate allow-list entry - see multi-device notes in
  [Desktop at work](/docs/desktop-at-work).
- The VPN is **not** a substitute for PIN / console approval. Anyone who can reach the host still
  needs a successful ceremony unless you deliberately ran `serve --open`.

## Requiring pairing (the default)

By default, the native host **requires** pairing, only devices that have paired can stream. This is
the right setting on a shared network: a device has to complete the PIN ceremony once before it can
connect.

If you're on a fully trusted single-user network and want to skip pairing, run the host open with
`serve --open`, it then advertises `pair=optional` and accepts unpaired clients. Requiring pairing
is strongly recommended.

## Trust-on-first-use (host opt-in)

Trust-on-first-use (TOFU) is **off by default** and is an explicit *host* opt-in for fully trusted
networks. A host enables it by running open, `serve --open`, which makes it advertise
`pair=optional` over mDNS and accept unpaired clients. Only then does a client offer the
TOFU path: connecting to such a host for the first time shows the host's fingerprint and asks you to
confirm it (compare it with the one the host logged at startup), then pins it. The client presents
this clearly as the reduced-security option, alongside **Pair with PIN**.

> **Warning:** TOFU cannot detect an impostor on the first connection, if someone is impersonating
> the host the very first time you connect, you'll pin the attacker's fingerprint. PIN pairing closes
> that gap (the SPAKE2 ceremony binds both identities), which is why it's the default. Use TOFU only
> on a network you fully trust, see [Security & Safe Use](/docs/security).

For every other case, a host advertising `pair=required` (the default), a host you typed in by hand,
or a discovered host whose pair policy is unknown, TOFU is not offered and the client routes straight
to the PIN ceremony.

Once a host is pinned, a fingerprint change is treated as the impostor signal: the client forces
re-pairing through the PIN ceremony rather than offering to re-trust the new identity.

**When TOFU is a bad fit**

- Office / VPN paths where the network is not solely yours.
- Any host that still has GameStream enabled on a segment you do not fully trust.
- Shared houses, labs, or corp VPN egress where another device could answer first.

Prefer PIN or console approval everywhere you would also refuse `serve --open`.

## Revoke and re-pair

### Revoke (remove a device)

The [web console](/docs/web-console) **Pairing** page lists every paired device. Remove one to
revoke its access immediately - that identity is no longer on the allow-list. Use this when:

- You lost a laptop or phone that was paired.
- You reinstalled a client OS and want a clean label.
- You are retiring a living-room device.

Revoke does not block the device's IP forever; it only drops the stored client identity. The same
physical machine can pair again with a new ceremony.

### Re-pair

Re-pairing is the PIN ceremony (or console approval) again:

1. Optionally remove the old entry on the host so labels stay clear.
2. Arm pairing (native) or start Moonlight **Pair** (GameStream).
3. Complete the PIN / approval flow.

**Host identity changed** (reinstall, wiped config, new machine with the same name): clients that
pinned the old fingerprint treat this as the impostor signal and **force** re-pairing - they will
not silently accept the new cert. That is intentional. After you trust the new fingerprint via PIN,
reconnects are automatic again.

If a client can't pair at all, see [Troubleshooting → Pairing is
rejected](/docs/troubleshooting#pairing-is-rejected--the-client-cant-connect).

## Managing paired devices

The [web console](/docs/web-console) lists every paired device and lets you remove one (revoking its
access). Re-pairing is just the PIN ceremony again - see [Revoke and re-pair](#revoke-and-re-pair)
above.

(There is also a developer/measurement host, `slipstream-host slipstream1-host`, a subcommand of the
same binary, not the host you install. It has its own `--allow-tofu` / `--pairing-pin` flags for test
harnesses; nothing on this page applies to it.)

## Related pages

- [The Web Console](/docs/web-console)
- [Connect with Moonlight](/docs/moonlight)
- [Network & VPN](/docs/network-and-vpn)
- [Desktop at work](/docs/desktop-at-work)
- [Security & Safe Use](/docs/security)
- [Clients](/docs/clients)
- [Troubleshooting → Pairing](/docs/troubleshooting#pairing-is-rejected--the-client-cant-connect)
