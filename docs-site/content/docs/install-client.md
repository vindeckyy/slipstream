---
title: Install a Client
description: Install the Slipstream client for Android or Steam Deck, or use Moonlight.
---

This page is the **install path for each named client**. For what each client *is* and which to
pick, see [Clients](/docs/clients); to install the **host**, see [Install the Host](/docs/install).
Whichever client you install, the first connection needs a one-time [pairing](/docs/pairing). If the
app installs but your host doesn't appear in its list, start at [Troubleshooting -> The host isn't
found on the network](/docs/troubleshooting#the-host-isnt-found-on-the-network).

Already installed? Skip to [Keeping a client up to date](#keeping-a-client-up-to-date) or
[Removing a client](#removing-a-client).

> The Android link below is the current preview channel. Host releases and future stable client
> channels are published from version tags. See
> [Release Channels](/docs/channels).

## Pick your device

| Device | Install |
|--------|---------|
| **Android / Android TV** | [Download the preview APK](#android) |
| **Steam Deck** | [Decky plugin](/docs/steam-deck) for Gaming Mode, or [Flatpak in Desktop Mode](#steam-deck) |
| Anything else (browser, old phone, TV) | [Moonlight](/docs/moonlight) |

## Android

The Android client is currently distributed as a preview APK. Sideload the
[Android APK](https://github.com/vindeckyy/slipstream/releases/download/android-preview/slipstream-android.apk)
or build from `clients/android/`. Download the accompanying checksum and verify it before copying
the APK to the tablet, then allow installs from your browser or file manager the first time.

```sh
sha256sum -c slipstream-android.apk.sha256
adb install -r slipstream-android.apk
```

### Fire HD 10 (13th Gen)

The 2023 Fire HD 10 uses model `KFTUWI` and has a 1920 x 1200 panel. Slipstream's safe default on
this tablet is **1680 x 1050 at 60 Hz**, using hardware HEVC when the decoder accepts it. Amazon's
documented hardware decoder limits cover 1080p60, so 1920 x 1200 stays an experimental custom mode;
the client probes the decoder and falls back to 1680 x 1050 or 1920 x 1080 when necessary.

AV1 is not advertised on this model. If HEVC cannot be configured, select H.264 in the host or
client profile. Keep the tablet on a 5 GHz Wi-Fi network for high-bitrate streams.

## Steam Deck

Most Deck users want **Gaming Mode**: install the **[Decky plugin](/docs/steam-deck)** and a
**Slipstream** panel lands in the Quick Access Menu, so you can discover hosts, pair with a PIN, and
stream **without dropping to the desktop**. Follow the **[Steam Deck (Decky) guide](/docs/steam-deck)**
, it walks through Decky Loader, the plugin, and the one-time client install.

> The plugin doesn't decode video itself, it drives the Flatpak `slipstream-client` installed on
> the Deck. If your client isn't one the plugin can update for you (a sysext, a nix profile, a source
> build), the panel shows you the update command instead of an **Update** button. The Gaming Mode
> panel comes from the plugin, so a client on its own won't add it. The Decky guide covers installing
> both, so start there.

For **Desktop Mode** (or to add the client to Game Mode as a non-Steam app yourself), install the
Flatpak; it carries its own libadwaita + SDL3 and survives SteamOS updates:

```sh
# Build locally, or install a .flatpak from GitHub Releases when attached:
# flatpak install --user --bundle /path/to/slipstream-client.flatpak
bash packaging/flatpak/build-flatpak.sh
flatpak install --user --bundle dist/slipstream-client-*.flatpak
```

See [packaging/flatpak](https://github.com/vindeckyy/slipstream/blob/main/packaging/flatpak/README.md).

## Anything else, Moonlight

Any device with a [Moonlight](https://moonlight-stream.org/) client (browser, old phone, smart TV)
connects over GameStream with no slipstream-specific software. See
[Connect with Moonlight](/docs/moonlight).

## Keeping a client up to date

Every platform is released from one tag. A client and a host don't have to be on the same version,
but keeping them close is the least surprising. (Updating the **host** is its own page:
[Updating](/docs/updating).)

| Client | How it updates |
|---|---|
| **Android** | Download the current preview APK from GitHub Releases and install it over the existing app |
| **Steam Deck (Decky)** | the panel's **Update** button, see [Steam Deck -> Updating](/docs/steam-deck#updating) |
| **Steam Deck (Flatpak alone)** | `flatpak update --user io.slipstream.Slipstream`, **without `sudo`** |

## Removing a client

The removal command for every client, and what removal deliberately leaves behind, this device's
identity and its saved hosts, and the pairing on the host, are on
[Uninstalling -> Clients](/docs/uninstall#clients). The same page covers forgetting your saved hosts
[without uninstalling anything](/docs/uninstall#removing-the-pairing-not-the-software), and removing
the **host**.
