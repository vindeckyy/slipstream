---
title: Fedora
description: Build and install the Slipstream host RPM on Fedora.
---

Install a Slipstream host on **Fedora** by building an RPM from this repo, or by installing a
release RPM from [GitHub Releases](https://github.com/vindeckyy/slipstream/releases) when one is
attached. There is no public dnf registry. The host installs as an RPM-managed systemd **`--user`**
service. It works with either **KDE Plasma** or **GNOME**; the desktop-specific setup (which
compositor captures, headless sessions, quirks) lives on the
[desktop configure pages](#5-configure-your-desktop). Host encode is **NVENC on NVIDIA**; on
**AMD/Intel** HEVC and AV1 go through **Vulkan Video**, with **VAAPI** for H.264 and as the fallback
(`SLIPSTREAM_ENCODER=auto` picks per GPU).

> New here? Read [Security & Safe Use](/docs/security) first, a streaming host is remote control of
> the machine, so keep it on a trusted LAN or VPN and require pairing.

Install is two parts: **GPU driver** -> **host RPM**. Then open the firewall and point the host at
your desktop from the [desktop configure pages](#5-configure-your-desktop).

## 1. NVIDIA driver (RPM Fusion akmod)

Enable RPM Fusion (free + nonfree), then install the akmod driver + CUDA. RPM Fusion's nonfree
NVIDIA repo is sometimes pre-enabled on some spins; the full free/nonfree repos below are still
needed (they carry the NVENC ffmpeg in the next step).

```sh
sudo dnf install \
  https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm \
  https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-$(rpm -E %fedora).noarch.rpm
sudo dnf install akmod-nvidia xorg-x11-drv-nvidia-cuda
```

**NVENC ffmpeg.** Fedora ships `ffmpeg-free`, which is built **without** NVENC, the host can't
encode with it. Swap to RPM Fusion's ffmpeg:

```sh
sudo dnf install --allowerasing ffmpeg ffmpeg-libs
ffmpeg -hide_banner -encoders | grep nvenc   # expect hevc_nvenc / av1_nvenc / h264_nvenc
```

**Secure Boot.** If `mokutil --sb-state` says *enabled*, the akmod module is signed with a
locally-generated key that must be enrolled once:

```sh
sudo akmods --force                                              # build + sign the module
sudo mokutil --import /etc/pki/akmods/certs/public_key.der       # set a one-time password
sudo reboot
```

On the next boot a blue **MOK Manager** screen appears **on the machine's console** (not over
SSH): *Enroll MOK -> Continue -> Yes -> (the password) -> Reboot*. Then verify:

```sh
nvidia-smi                              # driver loads
ffmpeg -hide_banner -encoders | grep nvenc
```

(Or disable Secure Boot in firmware to skip the MOK step, fine for a dedicated test box.)

**AMD / Intel.** No akmod needed, the Mesa stack carries both encode paths. HEVC and AV1 go through
**Vulkan Video** by default (the Mesa Vulkan driver, present on any normal Fedora desktop), and
**VAAPI** is the H.264 path and the fallback. Install the freeworld VAAPI drivers for full codec
support (`mesa-va-drivers-freeworld` for AMD from RPM Fusion, `intel-media-driver` for Intel); on a
desktop these are usually already present.

## 2. Install the host (local RPM)

There is no public RPM registry for Slipstream. Build from this repo with
[`packaging/rpm`](https://github.com/vindeckyy/slipstream/blob/main/packaging/rpm/README.md), or
install an RPM from [GitHub Releases](https://github.com/vindeckyy/slipstream/releases) when assets
are attached:

```sh
git clone https://github.com/vindeckyy/slipstream.git && cd slipstream
# Prefer the Fedora container CI uses so sonames match your release:
# docker build --build-arg FEDORA_VERSION=$(rpm -E %fedora) -f ci/fedora-rpm.Dockerfile -t ss-rpm ci
# then run packaging/rpm/build-rpm.sh inside it (see packaging/rpm/README.md).
sudo dnf install ./dist/slipstream-*.rpm
sudo usermod -aG input "$USER"     # /dev/uinput access for virtual gamepads (re-login to apply)
```

Updates later: rebuild or download a newer RPM, install it the same way, then
`systemctl --user restart slipstream-host` so the running host picks up the new binary. The package
ships the systemd user units, the udev rule, the UDP socket-buffer sysctl tuning, and example
configs.

Channel notes for when you publish your own feeds are in [Release Channels](/docs/channels).
Updating in general, including the opt-in one-click button in the web console, is covered in
[Updating the Host](/docs/updating).

> On Fedora 42 or older, or on a release newer than what the packaging tree documents, build with
> the same toolchain CI uses (`docker build --build-arg FEDORA_VERSION=NN -f ci/fedora-rpm.Dockerfile
> -t ss-rpm ci` then `packaging/rpm/build-rpm.sh` inside it), or build from source (appendix below).

## 3. Check it installed

Before moving on, confirm the binary is there and nothing else is competing for the same job:

```sh
slipstream-host --version           # the binary is on PATH
slipstream-host detect-conflicts    # exits 1 if Sunshine/Apollo is also installed
```

If `detect-conflicts` reports another streaming host, remove it before going further, two hosts on
one machine is the most common reason a clean install never streams. See
[Troubleshooting -> another streaming host is installed](/docs/troubleshooting#another-streaming-host-sunshine-apollo--is-installed).

Once you've enabled the service on your desktop page below, these are how you watch it:

```sh
systemctl --user status slipstream-host      # active
journalctl --user -u slipstream-host -f      # watch a client connect
```

## 4. Open the firewall

Fedora runs **firewalld** by default and the package never edits your firewall, so the host stays
unreachable until you allow it. The RPM installs the service definitions, enable them once.

The packaged unit runs native-only `serve`, so a host you enabled with `systemctl --user enable --now
slipstream-host` serves the native `slipstream/1` plane. Add the GameStream service only after
starting the host with `serve --gamestream`:

```sh
sudo firewall-cmd --reload                                            # load the installed definitions
sudo firewall-cmd --permanent --add-service=slipstream-native
sudo firewall-cmd --permanent --add-service=slipstream-gamestream
sudo firewall-cmd --reload
```

`slipstream-native` opens UDP 9777 (QUIC control), UDP 5353 (mDNS discovery) and TCP 47990 (the
mgmt/library API, HTTPS + mTLS, read-only off loopback). `slipstream-gamestream` opens the fixed
Moonlight ports, TCP 47984, 47989 and 48010, UDP 47998-48000, plus the same mDNS. The media
**data plane** uses an ephemeral UDP port the client opens with a hole-punch, so there is nothing
fixed to open for video.

Switched the host to **native-only**, dropped `--gamestream` with a
`systemctl --user edit slipstream-host` drop-in, or you run `slipstream-host serve` by hand? Then add
`slipstream-native` alone and leave `slipstream-gamestream` out. `systemctl --user cat slipstream-host`
shows which one yours is.

And if you want the web console reachable from another device, open **TCP 47992**:

```sh
sudo firewall-cmd --permanent --add-service=slipstream-web && sudo firewall-cmd --reload
```

## 5. Configure your desktop

How the host creates its virtual display and injects input depends on your desktop, not your distro.
Continue on the page for the desktop you run, it covers your `host.env`, any compositor quirks, and
starting the host:

- [KDE Plasma (KWin)](/docs/kde)
- [GNOME (Mutter)](/docs/gnome)
- [Steam / gamescope](/docs/gamescope)
- [Hyprland](/docs/hyprland)
- [Sway / wlroots](/docs/sway)

Enable the browser management console (status, paired devices, arm pairing), see
[Web Console](/docs/web-console).

For a headless KWin appliance that streams at boot with no graphical login, see
[KDE -> Headless session](/docs/kde#headless-session).

Full config reference: [Configuration](/docs/configuration). Service model:
[Running as a Service](/docs/running-as-a-service).

## 6. Connect a client

From any [client](/docs/clients), `--discover` finds the host on the LAN. On first connect, complete
the **PIN pairing**, arm it from the host's [web console](/docs/web-console#arm-pairing), which
displays a 4-digit PIN to type into the client. See [Clients](/docs/clients) and
[Pairing](/docs/pairing).

## Next steps

- **Keep it current**, [Updating the Host](/docs/updating).
- **Remove it again**, [Uninstalling](/docs/uninstall).
- **Something not working?**, [Troubleshooting](/docs/troubleshooting).

## Appendix, build from source

If there's no RPM for your Fedora release and you don't want to build one, compile the host directly
(no clean updates / no packaged units, you wire those up by hand):

```sh
sudo dnf install gcc gcc-c++ make cmake clang clang-devel nasm git pkgconf-pkg-config \
  pipewire-devel wayland-devel wayland-protocols-devel libxkbcommon-devel opus-devel \
  libdrm-devel mesa-libgbm-devel mesa-libGL-devel mesa-libEGL-devel mesa-libGLES-devel libva-devel \
  ffmpeg-devel libei-devel
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone https://github.com/vindeckyy/slipstream.git && cd slipstream
cargo build --release --locked \
  --features slipstream-host/nvenc,slipstream-host/vulkan-encode \
  -p slipstream-host
```

`mesa-libGL-devel` isn't optional, the zero-copy GPU path links `libGL`, and without it the build
fails at the link step with `cannot find -lGL`. The two `--features` are what the packaged builds
use: leave them off and the host has no direct NVENC (NVIDIA) and no Vulkan Video encode
(AMD/Intel), and quietly falls back to the slower libav backends.

Then write `~/.config/slipstream/host.env` (as in `/usr/share/slipstream/host.env.kde`, but the host
binary is `target/release/slipstream-host`) and run it inside your desktop session, for a headless
KWin appliance see [KDE -> Headless session](/docs/kde#headless-session).
