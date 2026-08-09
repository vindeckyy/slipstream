# winget manifests - Windows host

The reviewed source of truth for the `vindeckyy.SlipstreamHost` winget package. Everything except
`PackageVersion` / `InstallerUrl` / `InstallerSha256` / `ReleaseNotesUrl` is edited **here**;
`scripts/ci/winget-manifest.ps1` only substitutes those four per release, so the switches,
agreements and installation notes stay under normal code review.

| File | Purpose |
| --- | --- |
| `vindeckyy.SlipstreamHost.yaml` | Version manifest - ties the other two together. |
| `vindeckyy.SlipstreamHost.installer.yaml` | Installer type, scope, silent switches, `ProductCode`, URL + hash. |
| `vindeckyy.SlipstreamHost.locale.en-US.yaml` | User-facing metadata, `Agreements`, `InstallationNotes`. |

## Why these choices

- **`InstallerType: inno`, `Scope: machine`, `ElevationRequirement: elevatesSelf`.** The host
  registers a SYSTEM service, installs drivers and opens firewall ports; `PrivilegesRequired=admin`
  in the `.iss` means Setup raises its own UAC prompt. There is no per-user scope.
- **`ProductCode: {7C9E6A52-...}_is1`** - Inno's ARP key is `<AppId>_is1`. This is what correlates an
  installed host with the package for `winget list` / `winget upgrade`. **It must track `AppId` in
  `packaging/windows/slipstream-host.iss`** - if that GUID ever changes, change it here too or
  upgrades silently stop being detected.
- **`interactive` is in `InstallModes`.** `winget install vindeckyy.SlipstreamHost --interactive` runs the
  full existing wizard: every task checkbox, the web-console password page, the VB-CABLE notice.
  Nothing about the installer changes to support it.
- **No `/MERGETASKS` in the silent switches.** A silent install deliberately takes the *same* task
  defaults the wizard shows, so the product does not differ by install channel - a per-channel
  default is a support trap ("it works when I install it by hand"). The disclosures the wizard puts
  on screen are carried by `Agreements` instead, which winget shows *before* install and requires
  the user to accept.
- **`UpgradeBehavior: install`** - Inno upgrades in place (`UsePreviousAppDir=yes`). Uninstalling
  first would run the `[UninstallRun]` service + driver teardown between versions.

## Opting out of individual tasks

Inno's `/MERGETASKS` takes `!` prefixes to deselect a default-checked task. Use `--override`
(replaces winget's switches) rather than `--custom` (appends - you would end up with two
`/MERGETASKS` on one command line):

```powershell
winget install vindeckyy.SlipstreamHost --override "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- /MERGETASKS=!gamestream"
```

Task names: `installdriver`, `installgamepad`, `installaudiocable`, `installhdrlayer`,
`gamestream`, `allowpublicfw`, `startservice`, `trayicon`.

## Two installer behaviours that exist for this path

Both are in `packaging/windows/slipstream-host.iss` and both also fix pre-existing bugs on the
plain double-click upgrade path:

- **`InitializeSetup` uses `SuppressibleMsgBox`, not `MsgBox`.** A plain `MsgBox` ignores
  `/SUPPRESSMSGBOXES` and displays even under `/VERYSILENT` - an unattended install on a box that
  already runs Sunshine/Apollo would block on an invisible modal dialog. Suppressed it returns
  `IDNO`, so that install aborts (Setup exits non-zero) rather than proceeding into the unsupported
  dual-host state.
- **`GamestreamParam` is fresh-install-only.** On an upgrade the flag is omitted entirely, which
  `service install` reads as "keep host.env as-is". Passing an explicit on/off would rewrite
  `SLIPSTREAM_HOST_CMD` whenever it still holds either canonical value - so a silent upgrade, where
  no wizard carries the old choice forward, would flip a user's GameStream setting with nothing on
  screen.
- **`PublicFwParam` is fresh-install-only too**, and `--allow-public-network` is now tri-state
  (`=on` / `=off` / absent → keep the recorded choice, resolved from the `fw-allow-public` marker in
  `windows/service.rs`). This task is default-*unchecked*, so without the change a silent upgrade
  would have silently **revoked** a Public-network opt-in the user made once. The bare
  `--allow-public-network` form still means `on` for existing scripts; a malformed value is a hard
  error rather than a fall-through, since a typo'd opt-*out* must never resolve to "keep Public
  open".

## Release flow

`GitHub Actions` runs on stable `v*` tags only, **after** the installer is
attached to the GitHub release - winget validates the URL and hash, so a manifest must never be
published ahead of its artifact:

```powershell
scripts/ci/winget-manifest.ps1 -Version 0.19.2 `
  -InstallerPath C:\t\out\slipstream-host-setup-0.19.2.exe -OutDir C:\t\out\winget
```

The generated trio is attached to the same release. Canary builds are excluded: winget pins one
immutable artifact per version, so the rolling `canary/` alias has nothing it could point at.

## Validating a change

```powershell
winget validate --manifest packaging\winget
winget install --manifest packaging\winget          # local install from the manifest
```

For a throwaway check, `winget-pkgs`' `Tools\SandboxTest.ps1` runs a manifest in Windows Sandbox.
Note the host needs a real GPU and installs drivers, so a Sandbox run exercises the *manifest*
(download, hash, switches, ARP correlation) rather than a working stream.

## Publishing

The checked-in manifests are ready for validation and for a future submission to winget-pkgs. A self-hosted REST source can also serve the catalogue from the server directory, provided it is deployed behind HTTPS and supplied with release data.

~~~powershell
winget install --manifest packaging\winget
winget install vindeckyy.SlipstreamHost
~~~

Before submitting to winget-pkgs, use a publicly trusted code-signing certificate for the installer. The winget client does not sign binaries; it downloads the release asset and applies the same Windows trust checks as a browser download.
