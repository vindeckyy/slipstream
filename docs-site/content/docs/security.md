---
title: Security & Safe Use
description: What a streaming host actually exposes, why to keep it on a trusted network, and how Slipstream protects you.
---

Read this before you put a host on a network you don't fully control. Slipstream is built to be secure
**on a trusted local network**, and that's the setting we support today. This page is upfront about what
a streaming host is, what protects it, and where the honest limits are.

> **The short version**
> - **Keep the host on a network you trust**, your home LAN, or a private VPN that puts host and client
>   on the same subnet. **Do not port-forward it to the public internet.**
> - **A streaming host is remote control of the machine.** Anyone who can stream to it sees the screen
>   and can move the mouse, type, and act as a controller, the same as sitting at the keyboard.
> - **Pairing is the security boundary.** Require pairing (the default), pick a strong console
>   password, and review your paired devices from time to time.
> - **If you installed from a Linux package, the Moonlight/GameStream plane is on**, it pairs over
>   plain HTTP, so it belongs on a trusted LAN only.
>   [Turn it off](#gamestream--moonlight-compatibility-is-the-weak-crypto-path) if you don't use
>   Moonlight.
> - **Be thoughtful about *which* machine you run it on.** Prefer a dedicated or gaming PC over one
>   holding your most sensitive data.

## What a streaming host really is

Low-latency desktop and game streaming means two things travel over the network: **the screen goes
out, and input comes back in.** A paired client doesn't just watch, it drives. Its mouse, keyboard,
and controller are injected into the host's desktop, so **for anything it can reach, a streaming client
is equivalent to a person sitting at that machine.**

That's the feature. It's also the risk to understand:

- The host can capture the **secure desktop**, lock screen and elevation prompts, so a connected
  client can see and interact with those too. (This is what lets you unlock and administer a
  headless box remotely; it's the same capability Sunshine and Apollo provide.)
- Injected input isn't sandboxed to a game. Whoever is streaming can alt-tab, open a terminal, read
  files, or change settings, whatever the logged-in session can do.

This is true of **every** remote-access and game-streaming tool, not just Slipstream. The takeaway isn't
"don't use it", it's "treat access to your host the way you'd treat handing someone your unlocked
keyboard." The rest of this page is about making sure only people you intend can get that access.

## Keep it on a trusted network

**Slipstream assumes a trusted local network. It is not designed, tested, or hardened to be exposed to
the public internet. Do not port-forward it.** There is no WAN-hardening story yet: no rate-limited
public authentication gateway, no DDoS protection, no assumption that hostile traffic is constantly
probing the ports. Exposing the streaming ports directly to the internet puts an interactive
control surface for your machine in front of the entire world.

If you want to stream from outside your home, tunnel in instead of opening up:

- **Use a VPN**, WireGuard, Tailscale, or your router's built-in VPN. This puts your remote client on
  the *same private subnet* as the host, so from Slipstream's point of view it's still a local
  connection, and the tunnel (not Slipstream) handles internet-facing authentication and encryption.
  Discovery, pairing, and streaming then work exactly as they do at home. Step-by-step patterns,
  ports, and discovery notes: [Network & VPN](/docs/network-and-vpn). For office client → home
  desktop, also read [Desktop at work](/docs/desktop-at-work).
- **Don't** map a router port to the host. A port-forward turns "trusted LAN service" into
  "internet-facing service" with none of the protections that implies.

Either way, pairing is what ultimately gates access, but keeping the host off untrusted networks is
the first line. On a portable host the safest habit is still to stop the service when you don't need
it.

## What actually protects you

Slipstream has **no accounts and no cloud**. Trust is established directly, device-to-device, and then
pinned. The layers, from the outside in:

- **Pairing is required by default.** A new device can't stream until it completes a one-time
  **PIN pairing ceremony** (SPAKE2): the host shows a 4-digit PIN, you enter it on the client, and the
  exchange cryptographically binds both identities. An attacker who doesn't know the PIN gets a
  *single online guess*, no offline cracking, no dictionary attack. See
  [Pairing & Trust](/docs/pairing).
- **Identities are pinned.** After pairing, the client remembers the host's certificate fingerprint and
  the host stores the client's. Reconnects are automatic and mutually authenticated; if a host's
  fingerprint ever changes, the client refuses to auto-trust it and forces re-pairing.
- **The host's admin API is loopback-only.** The management API's read-only status is reachable by
  paired clients over the LAN (authenticated by their certificate), but every state-changing action,
  arming pairing, removing devices, session control, is honored **only from the host machine
  itself**.
- **The web console is your LAN-facing admin surface.** It serves on `https://<host-ip>:47992` so you
  can administer the host from another machine, and it performs those local-only actions on your
  behalf, over loopback, once you've logged in. Treat anyone who can reach port 47992 as a
  candidate administrator, and restrict that port at the host firewall if your LAN is shared.
- **The web console has its own password.** New Linux and SteamOS installs ask you to choose one in
  the browser and save it to `~/.config/slipstream/web-password`. Pick a strong one and keep it out
  of shared documents; repeated wrong guesses are rate-limited per IP. To read it back or change it,
  see [Forgot your Password?](/docs/forgot-password).
- **The [shared clipboard](/docs/clipboard) is opt-in at both ends.** The host never advertises it
  until an operator adds a line to `host.env`, and on your side it is a switch per *saved host*
  rather than a global one, because letting a machine read and write what you copy is a decision
  about that machine (Android is the one client where that switch starts on).

### GameStream / Moonlight compatibility is the weak-crypto path

To interoperate with stock [Moonlight](/docs/moonlight) clients, Slipstream can also speak the legacy
GameStream protocol. **That plane does not get the protections above.** Its pairing runs over plain
HTTP, and its legacy control encryption can reuse GCM nonces, so an attacker already on your network
could sit in the middle of a pairing, or recover input from a session. It is safe only on a LAN you
fully trust. The native `slipstream/1` plane is unaffected and always on; GameStream is a second
plane running beside it.

**Check whether your host has it on. The default differs by install path, and most Linux installs
have it on:**

- **Linux packages: on.** The bundled systemd user unit runs `serve --gamestream`, and the deb, rpm,
  Arch and Bazzite sysext packages all install that unit as it ships. If you installed Slipstream from
  a package and enabled the service, GameStream is on.
- **SteamOS / Steam Deck installer: on**, unless you ran it with `--no-gamestream`.
- **NixOS module: on**, `services.slipstream.host.gamestream` defaults to `true`.
- **A host you started yourself** with `slipstream-host serve`: off. Only `serve --gamestream` (or
  `--moonlight`) enables it.

The host doesn't hide it: whenever the compat planes come up, it logs a warning at startup naming
exactly this risk.

**To turn it off**, if you don't use Moonlight clients:

- **Linux**, override the unit's `ExecStart` with a drop-in, so a package upgrade doesn't undo it:
  see [What the unit starts](/docs/running-as-a-service#what-the-unit-starts).
- **SteamOS**, re-run the Deck installer with `--no-gamestream`.
- **NixOS**, set `services.slipstream.host.gamestream = false;` (this also drops the GameStream
  firewall ports).

If you do need Moonlight, keep that host on a network you trust and pair over it there. Don't pair a
GameStream client on a network you share with strangers.

## Choosing which machine to host on

We've put real work into hardening the host, sealed capture and gamepad channels, loopback-gated
admin, pinned trust, and we'll keep at it. But security is also about *blast radius*: if a host is
ever compromised, or you misconfigure trust, what does the attacker get? Pick the machine with that
in mind.

### The Linux host runs as your desktop user

The Linux host runs inside your normal desktop session as your **regular user account**, not root, so a
worst-case compromise is scoped to that user rather than the whole system. The same network guidance
applies: keep it on a trusted LAN or a VPN, require pairing, and don't expose it to the internet.

**Recommendation:** run the host on a **dedicated or gaming PC**, not on a machine that also holds
your most sensitive material (work laptop, financial records, the box with your password vault). A
gaming rig you stream from is a great fit; your primary secrets machine is not.

## Plugins run code on your host

A [plugin](/docs/plugins) is third-party code you choose to install, and it runs alongside the host,
so installing one is a real trust decision, the same kind as installing any other program on that
machine. Slipstream narrows it as far as it can:

- **Catalog entries are version- and hash-pinned.** Every plugin in the catalog names one exact
  version and that version's package hash, and the host re-checks the hash before it downloads
  anything. A package quietly republished under the same version number is refused, not installed.
- **The badges tell you who looked at it.** **Verified** means Slipstream maintainers reviewed
  *that exact package* from the built-in catalog. **External source** (*from &lt;source&gt;*) means a
  catalog **you** added. It is still pinned and hash-checked, but curated by somebody else.
  **Unverified** means
  you installed it by hand from a package spec: nobody reviewed it, nothing pins it, and it stays
  marked that way for as long as it is installed.
- **Adding a catalog source is a one-time trust decision.** Every plugin in that catalog becomes
  installable on this host. You can paste the source's `ed25519:` public key when you add it, and
  the host will then refuse any index from it that isn't correctly signed.
- **The runner is not the host.** Plugins run in a separate scripting runner process holding a
  capability-limited `plugin-token`, not the full-admin `mgmt-token`, so a plugin can't register
  hooks or admit new devices. On Linux it runs as your desktop user, like the host.

Install plugins only from sources you trust, and prefer Verified catalog entries. See
[Plugins](/docs/plugins) for the install flow and the CLI.

## A short hardening checklist

- **Require pairing**, it's the default; don't run `--open` / `--allow-tofu` except on a network you
  fully trust and control.
- **Use a strong console password** and keep it out of shared documents.
- **Restrict who can reach TCP 47992** at the host firewall if your LAN is shared. That port is the
  console, and the console is remote administration.
- **Stay on a trusted network**, LAN or VPN. Never port-forward to the internet.
- **Turn GameStream off** unless you specifically need Moonlight compatibility. Every Linux package
  ships it **on**, so on Linux this is something you do, not something you leave alone. See
  [above](#gamestream--moonlight-compatibility-is-the-weak-crypto-path).
- **Only install plugins you trust**, prefer Verified catalog entries.
- **Review paired devices** in the web console periodically; remove anything you don't recognize.
- **Keep the host updated**, security fixes ship in new builds.
- **On portable hosts**, stop the service when you're on an untrusted network.

## Reporting a vulnerability

Found a security issue? **Email [security@slipstream.com](mailto:security@slipstream.com).** Please
don't open a public issue, pull request, or chat post for a suspected vulnerability. That exposes
other users before a fix is available.

Helpful things to include:

- The component and version, e.g. `slipstream-host 0.22.3`, and which client.
- The impact, and the attacker's position (same LAN, a paired client, a local service account,
  admin, ...).
- Steps to reproduce, a proof-of-concept, or a crash/log if you have one.

We acknowledge reports within **3 business days** and practice coordinated disclosure. We'll keep
you posted, agree a disclosure date, and credit you when the fix ships (unless you'd rather stay
anonymous). The full policy is in
[`SECURITY.md`](https://github.com/vindeckyy/slipstream/blob/main/SECURITY.md).
