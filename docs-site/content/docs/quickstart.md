---
title: Quick start
description: Install the host, pair a client, and start a stream.
---

Trusted networks only (LAN or VPN). Do not port-forward Slipstream to the public internet. See
[Network](/docs/network-and-vpn).

## 1. Install the host

Follow [Install](/docs/install) for your distro. Then:

```sh
mkdir -p ~/.config/slipstream
cp /usr/share/slipstream/host.env.example ~/.config/slipstream/host.env
# Ubuntu packages may use /usr/share/slipstream-host/host.env.example instead

sudo usermod -aG input "$USER"   # then log out and back in
systemctl --user enable --now slipstream-host
systemctl --user enable --now slipstream-web
```

Open `https://<host-ip>:47992`, accept the self-signed certificate, and set the console password.

## 2. Install a client

| Device | Path |
|--------|------|
| **Android** | [Download the Android preview APK](https://github.com/vindeckyy/slipstream/releases/download/android-preview/slipstream-android.apk) |
| **Steam Deck** | Decky plugin + Flatpak (see [Clients](/docs/clients#steam-deck)) |
| Other | [Moonlight](https://moonlight-stream.org/) with GameStream enabled on the host |

## 3. Pair

1. In the console, open **Pairing** and arm pairing (2-minute window), or wait for **Waiting for approval**.
2. On a native client, select the host and enter the 4-digit PIN (or approve from the console).
3. For Moonlight: pair in Moonlight, then type Moonlight's PIN into the console **Moonlight (GameStream)** card.

Details: [Clients](/docs/clients#pairing).

## 4. Stream

Start a stream from the client. Defaults are fine for a first LAN session.

| Goal | Mouse | Display preset |
|------|-------|----------------|
| Games | Capture | Shared desktop or Headless box |
| Desktop / remote work | Desktop (absolute) | Workstation or Hot-desk |

More: [Usage](/docs/client-settings), [Displays](/docs/virtual-displays), [Network](/docs/network-and-vpn).
