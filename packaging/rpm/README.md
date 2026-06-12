# slipstream-host — RPM (Bazzite / Fedora Atomic) via the GitHub registry

`slipstream-host` is published as an RPM to **GitHub's RPM package registry** in the public `unom`
org (group `bazzite`), so Bazzite / Fedora Atomic hosts layer and update it with `rpm-ostree`.
CI (`.github/workflows/rpm.yml`) builds and publishes on every push to `main` (a rolling
`0.0.1-0.ciN.<sha>` build) and on `v*` tags (a clean `X.Y.Z-1`). The RPM is built in the
Fedora 43 image (`ci/fedora-rpm.Dockerfile`) so its auto-generated library Requires
(`libavcodec.so.NN`, …) match Bazzite's sonames; the NVIDIA driver lib (`libcuda.so.1`) is
excluded — NVENC/EGL come from whatever NVIDIA stack the host runs (a weak Recommends).

This is the same package as the [COPR](../copr/README.md) / [bootc](../bootc/Containerfile)
paths — same spec (`slipstream.spec`) — just self-hosted in GitHub instead of COPR, mirroring the
[Debian/apt](../debian/README.md) setup.

## Install on a Bazzite host (one-time)

```sh
# Trust + add the repo (rpm-ostree reads /etc/yum.repos.d). Public registry, no auth.
curl -fsSL https://github.com/vindeckyy/slipstream/api/packages/unom/rpm/bazzite.repo \
  | sudo tee /etc/yum.repos.d/slipstream.repo

# Layer the package, then reboot into the new deployment.
rpm-ostree install slipstream
systemctl reboot
```

After reboot, as the desktop user:

```sh
ujust add-user-to-input-group           # virtual gamepads need /dev/uinput (re-login).
                                        # Bazzite is atomic — use ujust, NOT `usermod -aG input`.
mkdir -p ~/.config/slipstream
cp /usr/share/slipstream/host.env.bazzite ~/.config/slipstream/host.env   # gamescope defaults
systemctl --user enable --now slipstream-host
```

(See [`../bazzite/README.md`](../bazzite/README.md) for the full appliance walkthrough —
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
PF_VERSION=0.0.1 bash packaging/rpm/build-rpm.sh   # -> dist/slipstream-0.0.1-1.fcNN.x86_64.rpm
```

Run it inside the Fedora 43 builder image so the deps resolve and match Bazzite:

```sh
docker build -f ci/fedora-rpm.Dockerfile -t slipstream-fedora-rpm ci
docker run --rm -v "$PWD:/src" -w /src slipstream-fedora-rpm \
  bash -lc 'git config --global --add safe.directory /src && PF_VERSION=0.0.1 bash packaging/rpm/build-rpm.sh'
```

A plain `rpmbuild`/COPR build with no `pf_version`/`pf_release` defines produces `0.0.1-1`.
