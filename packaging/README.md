# Packaging slipstream for Fedora / Bazzite

The slipstream host links system FFmpeg (NVENC on NVIDIA, VAAPI on AMD/Intel, with a GPU-less
software-H.264 fallback), PipeWire and Opus. This page covers packaging it for the
**Fedora Atomic / Bazzite** world (rpm-ostree + bootc), where most of those deps are already
present; the NVIDIA-specific notes below apply to the NVENC path.

> 👉 **Ubuntu/Debian hosts** build `.deb`s with [`debian/README.md`](debian/README.md) (or install
> release assets from GitHub Releases when attached).

> 👉 **End-to-end Bazzite setup walkthrough** (install → udev/group → `host.env` → service →
> firewall → verify → troubleshooting): [`bazzite/README.md`](bazzite/README.md). This file is the
> higher-level packaging rationale.

```
packaging/
  rpm/slipstream.spec      # the RPM (builds slipstream-host from source with cargo)
  bazzite/host.env        # gamescope-default config for a Bazzite appliance
  bazzite/README.md       # step-by-step Bazzite setup guide
  bazzite/*sysext*.sh     # the no-layering path: build/install/publish the systemd-sysext
  bootc/Containerfile     # bake slipstream into a Bazzite-based atomic image
  copr/                   # COPR build-from-SCM settings
```

The other packaging targets have their own READMEs: [`debian/`](debian/README.md) (apt),
[`arch/`](arch/README.md) (pacman binary repo + PKGBUILD + SteamOS sysext),
[`flatpak/`](flatpak/README.md) (the client), plus `kde/` and `linux/` helpers. **NixOS / Nix** users
get a flake (`flake.nix` at the
repo root) with reproducible host + client packages and a `services.slipstream` NixOS module -
see [`nix/README.md`](nix/README.md).

## What's needed beyond base Fedora

| Dependency | Where it comes from |
|---|---|
| `ffmpeg-libs` with **NVENC** | **RPM Fusion nonfree** (`ffmpeg`, not `ffmpeg-free`) |
| NVIDIA driver (`libnvidia-encode`, `libEGL_nvidia`) | Bazzite **-nvidia** images ship it; plain Fedora: `akmod-nvidia` + `xorg-x11-drv-nvidia-cuda` |
| gamescope, PipeWire, wireplumber | **Bazzite ships these**; plain Fedora: `dnf install gamescope pipewire wireplumber` |
| `opus`, `libei` | Fedora base / updates |

On **Bazzite** the only genuinely new runtime bits are `ffmpeg-libs` (RPM Fusion) + `opus` +
`libei` - the rest of the stack is already there. The default backend is **gamescope**
(`packaging/bazzite/host.env`), which the host spawns headless per session - no desktop login.

## Option A - systemd-sysext (recommended; no layering, no reboot)

On Bazzite / Fedora Atomic the recommended install is the **systemd-sysext** image - rpm-ostree
layering is a last resort per the Bazzite docs (it slows every OS update and can block upgrades),
while a sysext overlays `/usr` at runtime, survives OS updates, and updates in one command with
no reboot. CI wraps the same RPMs below into the image, so content and channels are identical.

```sh
curl -fsSLO https://raw.githubusercontent.com/vindeckyy/slipstream/main/packaging/bazzite/slipstream-sysext.sh
sudo bash slipstream-sysext.sh install     # then: sudo slipstream-sysext update | status | remove
```

Full walkthrough (incl. the F43→F44 rebase behavior and migration off layering):
[`bazzite/README.md`](bazzite/README.md).

## Option B - local RPM / rpm-ostree layering

Build the host RPM with [`rpm/README.md`](rpm/README.md) (or take one from GitHub Releases when
attached), then layer it. There is no public RPM registry.

```sh
# After packaging/rpm/build-rpm.sh (or equivalent) produces dist/*.rpm:
rpm-ostree install ./dist/slipstream-*.rpm && systemctl reboot
# updates: install a newer local RPM the same way, then reboot
```

## Option C - COPR (per-host, `rpm-ostree install`)

1. Create a COPR project, enable **build-from-SCM** pointing at this repo, spec path
   `packaging/rpm/slipstream.spec` (see `copr/README.md`). Under *External Repositories* add
   RPM Fusion nonfree so `ffmpeg-devel` resolves at build time.
2. On the Bazzite host:
   ```sh
   # RPM Fusion (for the NVENC ffmpeg) - usually already enabled on Bazzite
   rpm-ostree install \
     https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm \
     https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-$(rpm -E %fedora).noarch.rpm
   # enable the COPR + install slipstream (use the repo URL from your COPR project)
   sudo wget -O /etc/yum.repos.d/_copr_slipstream.repo \
     https://copr.fedorainfracloud.org/coprs/<owner>/<project>/repo/fedora-$(rpm -E %fedora)/
   rpm-ostree install slipstream
   systemctl reboot
   ```

## Option D - bootc (image-based, atomic)

Layer slipstream into a Bazzite image once, then rebase any number of hosts onto it - no
per-host drift. See `bootc/Containerfile`:
```sh
podman build -t ghcr.io/<you>/bazzite-slipstream -f packaging/bootc/Containerfile .
podman push  ghcr.io/<you>/bazzite-slipstream
# on the target:
sudo bootc switch ghcr.io/<you>/bazzite-slipstream && systemctl reboot
```

## First-run setup (all options)

```sh
ujust add-user-to-input-group           # virtual gamepads need /dev/uinput (then re-login).
                                        # On Bazzite use ujust, NOT `usermod -aG input` (atomic OS - it won't stick).
mkdir -p ~/.config/slipstream
cp /usr/share/slipstream/host.env.bazzite ~/.config/slipstream/host.env   # edit (gamescope app, etc.)
systemctl --user enable --now slipstream-host

# Management web console (pairing + status) - pulled in by default (the host RPM Recommends it;
# `--no-install-recommends` / headless-only boxes can skip it). Enable it, then choose a password:
systemctl --user enable --now slipstream-web
# open https://<host-ip>:47992
```

Pair a stock Moonlight client (mDNS-discovered), or connect the native slipstream/1 client - via the
web console at `https://<host-ip>:47992` or directly.

> ⚠️ **COPR caveat:** COPR's mock chroot has no `bun`, so a COPR build produces only
> `slipstream` + `slipstream-client` - **not** `slipstream-web`. For the console on a COPR/bootc host,
> build the RPM with `--with web` (or take a release asset that includes it); the sysext image
> includes the console too.

## Why not Flatpak (for the HOST)?

The host needs unsandboxed access the zero-copy NVENC path, `/dev/uinput`, the PipeWire
graph and the compositor's privileged protocols - a Flatpak sandbox fights all of these.
An RPM (or the bootc layer) installs into the host system where those just work.

> 👉 The **client** is a different story - it IS shipped as a Flatpak (the only viable
> Steam Deck install path: SteamOS `/usr` is read-only and lacks `libadwaita`/`libSDL3`). See
> [`flatpak/README.md`](flatpak/README.md). The client sandbox only needs the GPU render node,
> Wayland, PipeWire audio, the network and hidraw - all expressible as finish-args.

## Building the SRPM/RPM locally (Fedora only)

```sh
git archive --format=tar.gz --prefix=slipstream-0.23.0/ -o ~/rpmbuild/SOURCES/slipstream-0.23.0.tar.gz HEAD
rpmbuild -ba packaging/rpm/slipstream.spec     # needs the BuildRequires from the spec
# The archive prefix and filename must match the spec's default %{ss_version}.
```
(Not buildable on Debian/Ubuntu - use a Fedora toolbox/container or COPR.)
