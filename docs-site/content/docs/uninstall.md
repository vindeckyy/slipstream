---
title: Uninstalling
description: Remove the Slipstream host or client for every install method, and what each one deliberately leaves behind.
---

Every install method has a clean removal path. This page walks through each one, and, just as
important, says what stays on the machine afterwards: the Linux packages run no removal scripts of
their own.

> **Your configuration always survives.** Removing Slipstream never deletes its config directory,
> `~/.config/slipstream`. It holds the host's identity certificate and key, its management token,
> your paired devices, the web-console login password, `host.env`, the game library, the logs, and
> any installed [plugins](/docs/plugins) and their state. Keeping it is what lets a reinstall pick up
> where you left off, each section below gives the one command that clears it, for when you want a
> clean slate instead.

Jump to what you installed:

- Linux host, [apt](#ubuntu-apt) · [dnf](#fedora-dnf) · [rpm-ostree layer](#fedora-atomic--bazzite-rpm-ostree-layer) · [Bazzite sysext](#bazzite--fedora-atomic-systemd-sysext) · [pacman](#arch--cachyos-pacman) · [SteamOS on-device build](#steamos--steam-deck-host-on-device-build) · [NixOS](#nixos)
- [Clients](#clients), Android, Steam Deck
- [Plugins and the script runner](#plugins-and-the-script-runner)

## Linux hosts

### Stop the services first

The Linux packages ship systemd **user** units, and `systemctl --user enable` writes symlinks into
your home directory that package removal cannot see. Disable them before you remove anything, or
you'll be left with dangling links and a unit that fails at every login:

```sh
systemctl --user disable --now slipstream-host slipstream-web
```

Add `slipstream-scripting` to that line if you enabled the [plugin runner](/docs/plugins), and
`slipstream-kde-session` if you set up the [headless KDE session](/docs/kde#headless-session).

If you turned on linger so the host ran without a login, and nothing else on the box needs it:

```sh
sudo loginctl disable-linger "$USER"
```

### Ubuntu (apt)

```sh
sudo apt purge slipstream-host slipstream-web slipstream-scripting
sudo apt autoremove
```

Name only the packages you actually installed, the others are simply reported as not installed.
Then drop the repository and its key, so `apt update` stops contacting it:

```sh
sudo rm -f /etc/apt/sources.list.d/slipstream.list /etc/apt/keyrings/slipstream.asc
sudo apt update
```

**Left behind:** `~/.config/slipstream`, and the empty `slipstream-update` system group the package
created for [one-click updates](/docs/updating). Clear them with:

```sh
rm -rf ~/.config/slipstream
sudo groupdel slipstream-update
```

Your `input` group membership is harmless to keep (it is a stock Ubuntu group). Drop it with
`sudo gpasswd -d "$USER" input` if you'd rather not have it. If you opened the firewall, close it
again: `sudo ufw delete allow slipstream-native` (and `slipstream-gamestream` / `slipstream-web` if you
allowed those too).

### Fedora (dnf)

The host package is called **`slipstream`** on RPM, not `slipstream-host`:

```sh
sudo dnf remove slipstream slipstream-web slipstream-scripting
sudo rm -f /etc/yum.repos.d/slipstream.repo
```

**Left behind:** `~/.config/slipstream`, the `slipstream-update` group, and the signing key dnf
imported into the rpm keyring when it first installed a Slipstream package. Clear the first two with
`rm -rf ~/.config/slipstream` and `sudo groupdel slipstream-update`. The key is harmless to leave, on
its own it only marks packages from our registry as trusted, and nothing fetches them once the repo
file is gone.

On firewalld, close the ports you opened:

```sh
sudo firewall-cmd --permanent --remove-service=slipstream-native
sudo firewall-cmd --permanent --remove-service=slipstream-gamestream   # if you opened it
sudo firewall-cmd --permanent --remove-service=slipstream-web          # if you opened it
sudo firewall-cmd --reload
```

### Fedora Atomic / Bazzite (rpm-ostree layer)

If you layered the RPMs rather than using the sysext:

```sh
sudo rpm-ostree uninstall slipstream slipstream-web
systemctl reboot
```

The change only takes effect in the new deployment, so the reboot is part of the removal. Remove
`/etc/yum.repos.d/slipstream.repo` as well if you added it. `~/.config/slipstream` is untouched.

### Bazzite / Fedora Atomic (systemd-sysext)

This is the supported Bazzite path and the tidiest one, the whole install is a single image under
`/var/lib/extensions/`. Stop the services **before** you unmerge, because once the image is gone
their binaries are gone and the units just keep failing:

```sh
systemctl --user disable --now slipstream-host slipstream-web
sudo slipstream-sysext remove
```

`remove` deletes the image, its version sidecar, `/etc/slipstream-sysext.conf`, the tray autostart
entry, and the gamescope session drop-in unless you edited it, then prints
`slipstream sysext removed (user config in ~/.config/slipstream is untouched)`.

Three things it created outside `/usr` stay behind:

```sh
sudo rm -f /etc/modules-load.d/slipstream.conf /etc/udev/rules.d/60-slipstream.rules
sudo groupdel slipstream-update
```

And your config, if you want it gone: `rm -rf ~/.config/slipstream`. See
[Bazzite](/docs/bazzite#install) for the same sequence in context.

### Arch / CachyOS (pacman)

Close any firewall services you opened during installation while their named profiles are still
installed. Run only the lines that match your setup:

```sh
# ufw (CachyOS)
sudo ufw delete allow slipstream-native
sudo ufw delete allow slipstream-gamestream
sudo ufw delete allow slipstream-web

# firewalld (EndeavourOS and other Arch spins)
sudo firewall-cmd --permanent --remove-service=slipstream-native
sudo firewall-cmd --permanent --remove-service=slipstream-gamestream
sudo firewall-cmd --permanent --remove-service=slipstream-web
sudo firewall-cmd --reload
```

```sh
sudo pacman -Rns slipstream-host slipstream-web slipstream-gamescope \
  slipstream-scripting
```

Name only what you installed. `-Rns` also takes the dependencies nothing else needs and removes the
packages' own configuration files.

**Left behind:** `~/.config/slipstream` and the `slipstream-update` group. Remove them with
`rm -rf ~/.config/slipstream` and `sudo groupdel slipstream-update`.

### SteamOS / Steam Deck host (on-device build)

**There is no uninstall script for this install method.** The on-device build is spread across your
user session and a handful of root-owned files, so stop the user services first:

```sh
systemctl --user disable --now slipstream-host slipstream-web \
  slipstream-scripting slipstream-rebuild-check
rm -f ~/.config/systemd/user/slipstream-*.service
systemctl --user daemon-reload
```

Then follow [SteamOS (Host) -> Uninstalling](/docs/steamos-host#uninstalling) for the build
container, the files under your home, and the root-owned tuning. Don't skip the last of those: the
atomic-update keep list is what carries those files through every SteamOS update, so left alone they
stay on the device indefinitely.

**Left behind:** `~/.config/slipstream` (`rm -rf ~/.config/slipstream` for a clean slate), your
`input` group membership, and, if the installer seeded it because you had none, the KDE
RemoteDesktop portal grant at `~/.local/share/flatpak/db/kde-authorized`.

### NixOS

There is nothing to uninstall imperatively, remove what you declared:

1. Delete the `services.slipstream.*` options from your configuration.
2. Remove `slipstream.nixosModules.default` from the system's module list and the `slipstream` flake
   input.
3. Rebuild: `sudo nixos-rebuild switch`.

The unit, udev rules, sysctl tuning, firewall ports and `input` group membership all disappear with
the generation. The store paths stay until you garbage-collect, and `~/.config/slipstream`, which
the module never managed, stays regardless.

## Clients

Removing a client does **not** tell the host to forget it. Unpair the device from the host's
[web console](/docs/web-console) (Pairing -> unpair) if you want its pairing gone as well.

### Android / Android TV

Uninstall the APK from Settings -> Apps. The preview APK does not create a Slipstream account or
remote subscription.

### Steam Deck, Decky plugin

Uninstall **Slipstream** from Decky's own plugin list (Quick Access Menu -> the **plug** icon (Decky)
-> the **gear** (Settings), where the installed plugins are listed). Decky's uninstall hook does
nothing beyond that, the two Steam shortcuts it created, the Steam Input template, the client it
launched and `~/.config/slipstream` all survive. The step-by-step is on
[Steam Deck -> Uninstalling](/docs/steam-deck#uninstalling).

## Plugins and the script runner

Plugins are installed into the host's config directory, so they survive host removal. Take them off
before you uninstall the host, or delete the directories afterwards.

```sh
slipstream-host plugins list             # what's installed
slipstream-host plugins remove <name>    # uninstall one
slipstream-host plugins disable          # stop and disable the runner
```

`plugins remove` takes the plugin's code out of `plugins/`, but nothing else. These stay in the
config directory, `~/.config/slipstream`, whether you removed the plugins first or uninstalled the
host with them still installed:

- `plugins/`, the plugin code itself, for any plugin you did **not** `plugins remove`.
- `plugin-state/<plugin>/`, each plugin's own config and cache, including any API keys you put in
  a plugin's `config.json`. `plugins remove` does not touch this.
- `plugin-token`, the runner's scoped credential for the management API.

Deleting the whole config directory removes all three. The runner package itself
(`slipstream-scripting`) comes off with your package manager, as in the sections above.

## Removing the pairing, not the software

If you only want to undo a pairing, you don't need to uninstall anything. The two halves are
separate:

- **On the host**, unpair the device from the [web console](/docs/web-console); it stops being
  trusted immediately.
- **On the client**, forget the saved host from that client's host list (or reset its saved hosts
  and stream settings). The client keeps its identity, so a re-pair doesn't look like a brand-new
  device. Where a headless `slipstream` CLI is available, `slipstream hosts forget <host-ref>` drops
  one host and `slipstream reset` clears all of them plus the stream settings.

See [Pairing](/docs/pairing) for the full model.
