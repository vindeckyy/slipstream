# Security Policy

Slipstream can control a Linux host through the network. Keep the host on a trusted LAN or private
VPN, complete pairing only on a trusted link, and do not expose the management or streaming ports
through public port forwarding.

## Supported versions

The supported release is the newest stable version in the 0.23.x line and the current build from
main. Please reproduce a report on one of those versions before reporting it.

## Reporting a vulnerability

Use the [private security advisory form](https://github.com/vindeckyy/slipstream/security/advisories/new).
Do not post exploit details in a public issue, pull request, chat, or forum thread.

Include the affected component and version, the attacker position, impact, reproduction steps, logs or
a proof of concept, and any mitigation you have identified. Remove personal data and credentials from
attachments.

We acknowledge reports within three business days, provide an initial assessment within seven days,
and coordinate a disclosure date with the reporter. We credit reporters in the advisory unless they
request anonymity.

## Scope

In scope:

- The Linux host, including slipstream-host, slipstream-core, capture, encode, display, input, and
  protocol code.
- The native clients for iPhone, Android, and Steam Deck.
- The web management console and management API.
- The optional GameStream compatibility path.

The following are documented limits:

- An administrator or system service that already controls the host is outside Slipstream's trust
  boundary.
- A virtual display is a real monitor. Other processes in the same desktop session can capture it
  through normal operating-system APIs.
- GameStream compatibility uses legacy encryption. Use it only on a trusted LAN.
- Public-internet exposure is unsupported. Keep the host behind a trusted LAN or private VPN.
- Slipstream cannot safely share GameStream ports, discovery, or virtual-display drivers with
  Sunshine or another Moonlight-compatible host while that host is active.

## Verifying releases

Release artifacts published by this repository include SHA-256 checksums. Verify a downloaded file
before installation:

    sha256sum -c slipstream-<version>-<artifact>.sha256

A checksum detects transfer corruption. It does not establish who produced an artifact, so treat a
failed checksum or an unexpected release signature as a security report.
