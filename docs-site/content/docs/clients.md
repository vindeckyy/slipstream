---
title: Clients
description: Connect from Android, Steam Deck, or Moonlight.
---

## Named apps

| Client | Install | Protocol |
|--------|---------|----------|
| [**Android / Android TV**](https://github.com/vindeckyy/slipstream/releases/download/v0.23.0-public/app-release.apk) | [Download the Android public APK](https://github.com/vindeckyy/slipstream/releases/download/v0.23.0-public/app-release.apk) | `slipstream/1` |
| **Steam Deck** | Decky plugin + Flatpak `io.slipstream.Slipstream` | `slipstream/1` |

Native apps discover hosts on the LAN, pair with a PIN, reconnect on a pinned identity, and can
browse the host game library.

### Steam Deck

1. Install [Decky Loader](https://decky.xyz/).
2. Install the Slipstream Decky plugin (adds a QAM panel in Gaming Mode).
3. Install the Flatpak client the plugin drives (`io.slipstream.Slipstream`). Desktop Mode can run that Flatpak directly.

Deck controls forward as a Steam Deck pad when Steam Input is set for the client.

## Moonlight

Any [Moonlight](https://moonlight-stream.org/) client works when the host runs with `--gamestream`.
Prefer native apps when available. GameStream uses weaker control-plane crypto; enable it only on a
trusted LAN and keep it off for VPN-only hosts (see [Install](/docs/install)).

## Pairing

No accounts. Trust is between this client and this host.

### Native (`slipstream/1`)

1. Enable the [console](/docs/web-console).
2. Open **Pairing** → arm pairing (2 minutes) **or** connect and **Approve** under Waiting for approval.
3. On the client, select the host and enter the host PIN (native flow: host shows PIN).

### Moonlight

1. Host must be running with GameStream enabled.
2. In Moonlight, start Pair (Moonlight shows the PIN).
3. Enter that PIN in the console **Moonlight (GameStream) pairing** card.

After pairing, reconnects are automatic. To revoke a device, remove it on the Pairing page.

## While streaming

- **Ctrl+Alt+Shift+Q** releases captured mouse/keyboard on clients that support it.
- Mouse modes and picture settings: [Usage](/docs/client-settings).
