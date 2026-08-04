# slipstream-host - RPM (Bazzite / Fedora Atomic)

Build RPMs locally with this tree (or attach them to a
[GitHub Release](https://github.com/vindeckyy/slipstream/releases)). There is no public RPM
registry. CI (`.github/workflows/rpm.yml`) can still produce canary/stable builds when you wire
publishing to your own feed or to GitHub Releases - keep those channels separate (see
[Release Channels](../../docs-site/content/docs/channels.md)). The RPM is built in the Fedora image
(`ci/fedora-rpm.Dockerfile`) so its auto-generated library Requires (`libavcodec.so.NN`, ...) match
the target sonames; the NVIDIA driver lib (`libcuda.so.1`) is excluded - NVENC/EGL come from
whatever NVIDIA stack the host runs (a weak Recommends).

This is the same package as the [COPR](../copr/README.md) / [bootc](../bootc/Containerfile)
paths - same spec (`slipstream.spec`).

## Install on a Bazzite host (one-time)

```sh
# Build (see "Build an RPM locally" below), then layer host + web console and reboot.
# (slipstream Recommends slipstream-web; list it explicitly so it's pulled regardless of weak-dep
# settings.)
rpm-ostree install ./dist/slipstream-*.rpm ./dist/slipstream-web-*.rpm
systemctl reboot
```

If you publish your own dnf/rpm-ostree feed, point a `.repo` file at it with `gpgcheck=1` as in the
signing section below.

## Per-package signing (`gpgcheck=1`, active)

CI can GPG-sign every RPM: `packaging/rpm/sign-rpms.sh` (run from `rpm.yml` between build and
publish) signs with a dedicated EdDSA key (historical uid `packages@unom.io`, fingerprint
`AF245C506F4E4763` when using the committed public key) and self-verifies with
`rpmkeys --checksig` before publishing. The public key is committed at
`packaging/rpm/RPM-GPG-KEY-slipstream`. (This is a GPG/OpenPGP key - a `step-ca`/X.509 cert can't
sign RPMs.)

> Store `RPM_GPG_PRIVATE_KEY` as a CI secret on whatever forge you use. Verify end to end with
> `rpmkeys --checksig` on a built RPM (`NOKEY` until you import the public key still means *signed*).

On a `v*` tag build, a missing key **fails** the build: `sign-rpms.sh` will not publish unsigned
RPMs into a repo whose own instructions say `gpgcheck=1`. Non-release builds still fall through
unsigned so forks and local builds work.

How to generate (and rotate) a signing key:

```sh
# 1. Generate a DEDICATED, passphrase-less signing key.
gpg --batch --gen-key <<EOF
%no-protection
Key-Type: eddsa
Key-Curve: ed25519
Name-Real: slipstream packages
Name-Email: packages@example.invalid
Expire-Date: 0
%commit
EOF
gpg --armor --export-secret-keys packages@example.invalid   # -> RPM_GPG_PRIVATE_KEY CI secret
gpg --armor --export             packages@example.invalid > packaging/rpm/RPM-GPG-KEY-slipstream
```

Commit the public half next to the packaging scripts, and serve it from your own feed if you
publish one (so `gpgkey=` in a `.repo` file resolves).

**This key also signs the Bazzite sysext feed**, and a third copy of its public half is baked into
`packaging/bazzite/slipstream-sysext.sh` (`FEED_KEY=`) - that script is bootstrapped by `curl` on
machines that have nothing installed yet, so it can't fetch the key from the thing it's
authenticating. A rotation must update **all three**: the CI secret, this directory's
`RPM-GPG-KEY-slipstream`, and `FEED_KEY`. `publish-sysext-feed.sh` compares its signing key's
fingerprint against `FEED_KEY` and refuses to sign on a mismatch.

After reboot, as the desktop user:

```sh
ujust add-user-to-input-group           # virtual gamepads need /dev/uinput (re-login).
                                        # Bazzite is atomic - use ujust, NOT `usermod -aG input`.
mkdir -p ~/.config/slipstream
cp /usr/share/slipstream/host.env.bazzite ~/.config/slipstream/host.env   # gamescope defaults
systemctl --user enable --now slipstream-host
# Web console - enable it, then choose a login password in the browser:
systemctl --user enable --now slipstream-web
# open https://<host-ip>:47992
```

(See [`../bazzite/README.md`](../bazzite/README.md) for the full appliance walkthrough -
udev/group, `host.env`, the Steam session unit, firewall, verify.)

## Updates

```sh
rpm-ostree upgrade            # pulls the newest slipstream with the system update
systemctl reboot             # rpm-ostree changes apply on reboot
```

Layered packages are re-resolved against their repos on every `rpm-ostree upgrade`, so the box
tracks new builds automatically (Bazzite's auto-update timer does this for you). To pin or stop
tracking: `rpm-ostree override` / `rpm-ostree uninstall slipstream`.

## Build an RPM locally

```sh
PF_VERSION=0.0.1 bash packaging/rpm/build-rpm.sh                # host + client
PF_VERSION=0.0.1 PF_WITH_WEB=1 bash packaging/rpm/build-rpm.sh  # + slipstream-web (needs bun on PATH)
# -> dist/slipstream-0.0.1-1.fcNN.x86_64.rpm  (+ slipstream-web-0.0.1-1.fcNN.x86_64.rpm with PF_WITH_WEB=1;
#    the web subpackage vendors a bun binary, so it's arch-specific, not noarch)
```

Run it inside the Fedora 43 builder image so the deps resolve and match Bazzite:

```sh
docker build -f ci/fedora-rpm.Dockerfile -t slipstream-fedora-rpm ci
docker run --rm -v "$PWD:/src" -w /src slipstream-fedora-rpm \
  bash -lc 'git config --global --add safe.directory /src && PF_VERSION=0.0.1 bash packaging/rpm/build-rpm.sh'
```

A plain `rpmbuild`/COPR build with no `ss_version`/`ss_release` defines produces `0.3.0-1` (the
spec defaults).

### aarch64 - the client RPM

The **client** builds for aarch64; the **host** does not (its encode stack is NVENC/QSV/AMF, all
x86). `PF_WITHOUT_HOST=1` drops the host binary, the tray, the headless-session data, the
firewalld services and the main package's `%files`, leaving exactly one RPM: `slipstream-client`.
Omitting the main `%files` is what keeps rpm from emitting an empty `slipstream` next to it.

This is **not** a cross-compile - `%build` runs cargo for the host architecture, so run it on an
arm64 machine (or an emulated arm64 container, which is very slow):

```sh
docker build --platform linux/arm64 -f ci/fedora-rpm.Dockerfile -t slipstream-fedora-rpm-arm64 ci
docker run --rm --platform linux/arm64 -v "$PWD:/src" -w /src slipstream-fedora-rpm-arm64 \
  bash -lc 'git config --global --add safe.directory /src && \
            PF_VERSION=0.0.1 PF_WITHOUT_HOST=1 bash packaging/rpm/build-rpm.sh'
# -> dist/slipstream-client-0.0.1-1.fcNN.aarch64.rpm
```

`PF_WITHOUT_HOST=1` works on x86_64 too, if you only want the client RPM. The flag is orthogonal
to the architecture; it is just that aarch64 has no other option.
