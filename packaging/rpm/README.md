# slipstream-host — RPM (Bazzite / Fedora Atomic) via the GitHub registry

`slipstream-host` is published as an RPM to **GitHub's RPM package registry** in the public `unom`
org (group `bazzite`), so Bazzite / Fedora Atomic hosts layer and update it with `rpm-ostree`.
CI (`.github/workflows/rpm.yml`) builds and publishes on every push to `main` (a rolling
`0.2.0-0.ciN.<sha>` build, which outranks the stray `0.1.1` so `rpm-ostree upgrade` always gets the
latest — no version pin needed) and on **host-scoped** `host-v*` tags (a clean `X.Y.Z-1`; the Apple
client's `v*` tags deliberately do **not** publish a host RPM). The RPM is built in the
Fedora 43 image (`ci/fedora-rpm.Dockerfile`) so its auto-generated library Requires
(`libavcodec.so.NN`, …) match Bazzite's sonames; the NVIDIA driver lib (`libcuda.so.1`) is
excluded — NVENC/EGL come from whatever NVIDIA stack the host runs (a weak Recommends).

This is the same package as the [COPR](../copr/README.md) / [bootc](../bootc/Containerfile)
paths — same spec (`slipstream.spec`) — just self-hosted in GitHub instead of COPR, mirroring the
[Debian/apt](../debian/README.md) setup.

## Install on a Bazzite host (one-time)

```sh
# Add the repo. Our RPMs are unsigned, but GitHub GPG-signs the repo METADATA — so verify that
# (repo_gpgcheck=1) and skip the per-package signature check (gpgcheck=0). The signed metadata
# carries each package's SHA256, so authenticity still holds. (Don't just curl GitHub's served
# bazzite.repo — it sets gpgcheck=1, which fails on unsigned packages.)
sudo tee /etc/yum.repos.d/slipstream.repo >/dev/null <<'REPO'
[github-unom-bazzite]
name=slipstream (unom, Bazzite)
baseurl=https://github.com/vindeckyy/slipstream/api/packages/unom/rpm/bazzite
enabled=1
gpgcheck=0
repo_gpgcheck=1
gpgkey=https://github.com/vindeckyy/slipstream/api/packages/unom/rpm/repository.key
REPO

# Layer the host + the web console (pairing/status), then reboot into the new deployment.
# (slipstream Recommends slipstream-web; list it explicitly so it's pulled regardless of weak-dep
# settings. The registry carries slipstream-web because CI builds the spec --with web; COPR can't.)
rpm-ostree install slipstream slipstream-web
systemctl reboot
```

> If `rpm-ostree` can't complete the metadata GPG check non-interactively, set `repo_gpgcheck=0`
> (TLS-only trust to the self-hosted registry).

## Enabling per-package signing (`gpgcheck=1`)

CI is wired to GPG-sign each RPM (`packaging/rpm/sign-rpms.sh`, run from `rpm.yml`), but it's
**dormant** until you provide a signing key — until then packages publish unsigned and the repo
above uses `gpgcheck=0`. This is a self-hosted registry served over HTTPS with GPG-signed metadata
(`repo_gpgcheck=1`), so per-package signing is hardening, not a correctness fix. (Note: this is a
GPG/OpenPGP key — a `step-ca`/X.509 cert can't sign RPMs; step-ca is for the registry/console TLS.)

One-time setup:

```sh
# 1. Generate a DEDICATED, passphrase-less signing key (separate from the GitHub registry key).
gpg --batch --gen-key <<EOF
%no-protection
Key-Type: eddsa
Key-Curve: ed25519
Name-Real: slipstream packages
Name-Email: packages@unom.io
Expire-Date: 0
%commit
EOF
gpg --armor --export-secret-keys packages@unom.io   # -> paste into the CI secret below
gpg --armor --export             packages@unom.io > RPM-GPG-KEY-slipstream   # the PUBLIC key

# 2. In the repo's GitHub Actions secrets, add RPM_GPG_PRIVATE_KEY = the armored PRIVATE key
#    (and RPM_GPG_PASSPHRASE only if the key has one). The next CI run signs + self-verifies.

# 3. Publish RPM-GPG-KEY-slipstream where clients can fetch it, then on each host import it and
#    flip the repo to gpgcheck=1:
sudo rpm --import https://github.com/vindeckyy/slipstream/.../RPM-GPG-KEY-slipstream
sudo sed -i 's/^gpgcheck=0/gpgcheck=1/' /etc/yum.repos.d/slipstream.repo
```

Do **not** flip `gpgcheck=1` before a signed build has published, or installs will fail.

After reboot, as the desktop user:

```sh
ujust add-user-to-input-group           # virtual gamepads need /dev/uinput (re-login).
                                        # Bazzite is atomic — use ujust, NOT `usermod -aG input`.
mkdir -p ~/.config/slipstream
cp /usr/share/slipstream/host.env.bazzite ~/.config/slipstream/host.env   # gamescope defaults
systemctl --user enable --now slipstream-host
# Web console — enable it and read the auto-generated login password (then open http://<host-ip>:3000):
systemctl --user enable --now slipstream-web
journalctl --user -u slipstream-web-init | sed -n 's/.*password generated: //p'
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
PF_VERSION=0.0.1 bash packaging/rpm/build-rpm.sh                # host + client
PF_VERSION=0.0.1 PF_WITH_WEB=1 bash packaging/rpm/build-rpm.sh  # + the noarch slipstream-web (needs bun on PATH)
# -> dist/slipstream-0.0.1-1.fcNN.x86_64.rpm  (+ slipstream-web-0.0.1-1.fcNN.noarch.rpm with PF_WITH_WEB=1)
```

Run it inside the Fedora 43 builder image so the deps resolve and match Bazzite:

```sh
docker build -f ci/fedora-rpm.Dockerfile -t slipstream-fedora-rpm ci
docker run --rm -v "$PWD:/src" -w /src slipstream-fedora-rpm \
  bash -lc 'git config --global --add safe.directory /src && PF_VERSION=0.0.1 bash packaging/rpm/build-rpm.sh'
```

A plain `rpmbuild`/COPR build with no `pf_version`/`pf_release` defines produces `0.0.1-1`.
