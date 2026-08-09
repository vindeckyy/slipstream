//! Virtual-gamepad backend resolution for the native session (plan §W1 — carved out of the
//! [`super`] module). Maps the client's [`GamepadPref`] (plus the host `SLIPSTREAM_GAMEPAD` env and
//! the live platform) to a backend this host can actually build, applying the runtime UHID /
//! Steam-conflict degrades. Pure selection ([`pick_gamepad`]) is separated from the env/logging
//! shell ([`resolve_gamepad`]) and the per-pad variant ([`resolve_pad_kind`]); the platform degrades
//! (`degrade_if_no_uhid`, `physical_steam_product_present`, `degrade_steam_on_conflict`) are
//! resolved against the live Linux input devices.

use super::*;

/// The per-pad routing decision for one frame ([`Pads::handle`]): given `owner` (the manager
/// holding a live device at this index, if any), the client-`declared` kind, and whether this is a
/// create/update frame (`present`) vs a removal, return `(kind to route to, new owner)`.
///
/// A live device stays in its owning manager even if the declared kind later changes (so a pad is
/// never duplicated across managers); the declared kind takes effect only when no device exists
/// yet; a removal routes to the owner's manager (so it tears the right device down) and clears the
/// owner.
pub(super) fn route_decision(
    owner: Option<GamepadPref>,
    declared: GamepadPref,
    present: bool,
) -> (GamepadPref, Option<GamepadPref>) {
    match (owner, present) {
        (Some(k), true) => (k, Some(k)), // keep the existing device in its manager
        (Some(k), false) => (k, None),   // removal → owner's manager, then clear
        (None, true) => (declared, Some(declared)), // create in the declared kind's manager
        (None, false) => (declared, None), // removal with no device — a harmless no-op
    }
}

/// Resolve one client-declared per-pad kind to a backend this host can actually build (mixed
/// types): the platform map + the runtime UHID / Steam-conflict degrades that [`resolve_gamepad`]
/// applies to the session default, minus the Auto/env session logic (a per-pad declaration is
/// always a concrete kind).
pub(super) fn resolve_pad_kind(kind: GamepadPref) -> GamepadPref {
    let chosen = pick_gamepad(
        kind,
        None,
    );
    degrade_steam_on_conflict(degrade_if_no_uhid(chosen))
}

/// Pure selection of the session's virtual-gamepad backend: the client's explicit `pref` wins,
/// then the host's `SLIPSTREAM_GAMEPAD` env var (under a client `Auto`), then X-Box 360.
///
/// Unsupported requests degrade to X-Box 360, so a session without a rich pad still streams.
fn pick_gamepad(pref: GamepadPref, env: Option<&str>) -> GamepadPref {
    let want = match pref {
        GamepadPref::Auto => env
            .and_then(GamepadPref::from_name)
            .unwrap_or(GamepadPref::Auto),
        explicit => explicit,
    };
    match want {
        GamepadPref::DualSense
        | GamepadPref::DualShock4
        | GamepadPref::XboxOne
        | GamepadPref::SteamDeck
        | GamepadPref::SteamController
        | GamepadPref::DualSenseEdge
        | GamepadPref::SwitchPro
        | GamepadPref::SteamController2
        | GamepadPref::SteamController2Puck => want,
        // New Steam Controller (2026, `28DE:1302`): passed through as-is on Linux — the Triton
        // UHID backend mirrors the client's raw reports under the real identity and Steam on
        // the host drives it over hidraw (no kernel driver binds the PID; Steam Input is the
        // consumer). No Windows backend; folds to Xbox360 there.
        _ => GamepadPref::Xbox360,
    }
}

/// Runtime degrade for the Linux UHID backends (DualSense / DualShock 4 / Steam Deck): if
/// `/dev/uhid` can't be opened for write *now*, fall back to the uinput X-Box 360 pad rather than a
/// dead controller. Cheap, opens and drops the character device, and creates no device.
fn degrade_if_no_uhid(chosen: GamepadPref) -> GamepadPref {
    let needs_uhid = matches!(
        chosen,
        GamepadPref::DualSense
            | GamepadPref::DualSenseEdge
            | GamepadPref::DualShock4
            | GamepadPref::SteamDeck
            | GamepadPref::SteamController
            | GamepadPref::SteamController2
            | GamepadPref::SteamController2Puck
            | GamepadPref::SwitchPro
    );
    if needs_uhid
        && std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/uhid")
            .is_err()
    {
        tracing::warn!(
            wanted = chosen.as_str(),
            "/dev/uhid not writable — falling back to the X-Box 360 pad"
        );
        return GamepadPref::Xbox360;
    }
    chosen
}

/// The Valve product id (`28DE:xxxx`) a virtual Steam backend enumerates as, or `None` for a
/// non-Steam backend. This is the identity the conflict gate compares against the *physical* Valve
/// devices attached to the host: only a genuine duplicate (same VID **and** PID) confuses Steam
/// Input, so the gate keys on the PID, not on the `28DE` vendor alone.
fn steam_backend_product(pref: GamepadPref) -> Option<u16> {
    match pref {
        GamepadPref::SteamDeck => Some(0x1205), // built-in Deck controller identity
        GamepadPref::SteamController => Some(0x1102), // classic wired Steam Controller
        GamepadPref::SteamController2 => Some(0x1302), // Steam Controller 2 (Triton), wired
        GamepadPref::SteamController2Puck => Some(0x1304), // Steam Controller 2 via its Puck dongle
        _ => None,
    }
}

/// True if a **physical** Valve device with this exact product id (`28DE:{product}`) is already
/// attached. The host's own Steam Input is then managing that identity, and presenting a *second,
/// identical* virtual one makes Steam juggle two copies of the same Deck — confirmed conflict-prone
/// on a Deck-as-host (the physical `28DE:1205` + Steam's `28DE:11FF` XInput output pad are both
/// live).
///
/// Crucially this keys on the PID, not just the `28DE` vendor: a *different* Valve product is not a
/// conflict. A physical Steam Controller 2 (`28DE:1302`) must NOT block a virtual Steam Deck
/// (`28DE:1205`) — Steam Input drives distinct Steam controllers side by side, so the wanted pad
/// has to pass through unharmed (this over-broad vendor match degraded such sessions to DualSense).
///
/// HID device dirs are named `BUS:VID:PID.INST` (uppercase); a UHID virtual device resolves through
/// `/devices/virtual/…`, a real one does not. Slipstream's OWN virtual pads must never count: the
/// usbip/gadget transports present a real USB device (vhci resolves through `vhci_hcd`, NOT
/// `/devices/virtual/`), so a just-ended session's pad still detaching — or a concurrent session's
/// live one — read as "physical" and degraded every back-to-back Deck session to DualSense
/// (observed live on Bazzite 2026-07-04). Ours are recognizable by the `FVPF…` serial
/// ([`steam_proto::deck_serial`]) in `HID_UNIQ`, with the vhci path as belt and braces.
fn physical_steam_product_present(product: u16) -> bool {
    let needle = format!(":28DE:{product:04X}");
    let Ok(entries) = std::fs::read_dir("/sys/bus/hid/devices") else {
        return false;
    };
    entries.flatten().any(|e| {
        if !e.file_name().to_string_lossy().contains(&needle) {
            return false;
        }
        if std::fs::read_to_string(e.path().join("uevent"))
            .is_ok_and(|u| u.lines().any(|l| l.starts_with("HID_UNIQ=FVPF")))
        {
            return false; // one of our own virtual pads
        }
        match std::fs::read_link(e.path()) {
            Ok(target) => {
                let t = target.to_string_lossy();
                !t.contains("/virtual/") && !t.contains("vhci_hcd")
            }
            Err(_) => true,
        }
    })
}

/// Gate a virtual Steam pad off when a physical Steam controller *of the same identity* is attached
/// (§ conflict). Degrade to DualSense (then the uhid ladder), which Steam treats as an ordinary,
/// distinct pad. Override with `SLIPSTREAM_STEAM_FORCE=1` when the host has no competing Steam Input
/// (e.g. a remote-only box).
///
/// Only a same-PID duplicate degrades: a physical Steam Controller 2 no longer blocks a virtual
/// Steam Deck (and vice versa), so a Steam Machine with an SC2 plugged in streams the pad the client
/// actually asked for.
fn degrade_steam_on_conflict(chosen: GamepadPref) -> GamepadPref {
    let Some(product) = steam_backend_product(chosen) else {
        return chosen; // not a virtual Steam (28DE) pad — nothing to gate
    };
    let forced = std::env::var("SLIPSTREAM_STEAM_FORCE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !forced && physical_steam_product_present(product) {
        let conflict = format!("28DE:{product:04X}");
        tracing::warn!(
            wanted = chosen.as_str(),
            conflict = conflict.as_str(),
            "a physical Steam controller of the same identity is attached — the host's Steam Input \
             would manage two identical 28DE devices; falling back to DualSense (set \
             SLIPSTREAM_STEAM_FORCE=1 to override)"
        );
        return degrade_if_no_uhid(GamepadPref::DualSense);
    }
    chosen
}

/// Resolve the client's gamepad-backend preference (the env/logging shell around
/// [`pick_gamepad`]). Always concrete — the `Welcome` reports what the session will drive.
pub(super) fn resolve_gamepad(pref: GamepadPref) -> GamepadPref {
    let env = ss_host_config::config().gamepad.clone();
    let chosen = pick_gamepad(
        pref,
        env.as_deref(),
    );
    // Runtime degrade (separate from the compile-time platform check above): the Linux UHID
    // backends need `/dev/uhid` usable *now*, else creating the device just fails and the controller
    // goes dead — fall back to the always-available uinput X-Box 360 pad instead.
    let chosen = degrade_if_no_uhid(chosen);
    // Conflict gate: don't present a virtual Steam (28DE) pad when the host already has a physical
    // Steam controller — its own Steam Input would then manage two Decks (confirmed conflict-prone on
    // a Deck-as-host). `SLIPSTREAM_STEAM_FORCE=1` overrides.
    let chosen = degrade_steam_on_conflict(chosen);
    match pref {
        GamepadPref::Auto => {
            // The operator's env knob deserves a diagnostic when it didn't drive the
            // choice — a typo, or a DualSense wish on a non-UHID host, would otherwise
            // degrade silently.
            if let Some(env) = env.as_deref() {
                if GamepadPref::from_name(env) != Some(chosen) {
                    tracing::warn!(
                        env,
                        chosen = chosen.as_str(),
                        "SLIPSTREAM_GAMEPAD unrecognized or unavailable — falling back"
                    );
                }
            }
            tracing::info!(gamepad = chosen.as_str(), "gamepad backend (client: auto)")
        }
        want if want == chosen => {
            tracing::info!(gamepad = chosen.as_str(), "honoring client gamepad request")
        }
        want => tracing::warn!(
            requested = want.as_str(),
            chosen = chosen.as_str(),
            "client-requested gamepad backend unavailable — falling back"
        ),
    }
    chosen
}

#[cfg(test)]
mod tests {
    use super::{pick_gamepad, route_decision};
    use slipstream_core::config::GamepadPref;

    #[test]
    fn per_pad_route_decision() {
        use GamepadPref::{DualSense, Xbox360};
        // First frame with no device: create in the declared kind's manager, record ownership.
        assert_eq!(
            route_decision(None, DualSense, true),
            (DualSense, Some(DualSense))
        );
        // Subsequent frame: stays in the owning manager even if the declared kind now differs
        // (the arrival-after-first-frame reorder) — never a second device in another manager.
        assert_eq!(
            route_decision(Some(DualSense), Xbox360, true),
            (DualSense, Some(DualSense))
        );
        // Removal (cleared bit): routes to the owner so the RIGHT device is torn down, then clears.
        assert_eq!(
            route_decision(Some(DualSense), Xbox360, false),
            (DualSense, None)
        );
        // Removal with no device is a harmless no-op route (owner stays cleared).
        assert_eq!(route_decision(None, Xbox360, false), (Xbox360, None));
        // A fresh device after a re-plug picks up the newly-declared kind (owner was cleared).
        assert_eq!(
            route_decision(None, Xbox360, true),
            (Xbox360, Some(Xbox360))
        );
    }

    #[test]
    fn gamepad_resolution_precedence() {
        use GamepadPref::*;
        assert_eq!(pick_gamepad(DualSense, Some("xbox360")), DualSense);
        assert_eq!(pick_gamepad(Xbox360, Some("dualsense")), Xbox360);
        assert_eq!(pick_gamepad(Auto, Some("dualsense")), DualSense);
        assert_eq!(pick_gamepad(Auto, Some("xbox360")), Xbox360);
        assert_eq!(pick_gamepad(Auto, None), Xbox360);
        assert_eq!(pick_gamepad(Auto, Some("bogus")), Xbox360);
        assert_eq!(pick_gamepad(DualShock4, None), DualShock4);
        assert_eq!(pick_gamepad(Auto, Some("ps4")), DualShock4);
        assert_eq!(pick_gamepad(XboxOne, None), XboxOne);
        assert_eq!(pick_gamepad(Auto, Some("series")), XboxOne);
        assert_eq!(pick_gamepad(SteamDeck, None), SteamDeck);
        assert_eq!(pick_gamepad(Auto, Some("deck")), SteamDeck);
        assert_eq!(pick_gamepad(SteamController, None), SteamController);
        assert_eq!(pick_gamepad(Auto, Some("steamcontroller")), SteamController);
        assert_eq!(pick_gamepad(DualSenseEdge, None), DualSenseEdge);
        assert_eq!(pick_gamepad(Auto, Some("edge")), DualSenseEdge);
        assert_eq!(pick_gamepad(SwitchPro, None), SwitchPro);
        assert_eq!(pick_gamepad(Auto, Some("switch")), SwitchPro);
        assert_eq!(pick_gamepad(SteamController2, None), SteamController2);
        assert_eq!(pick_gamepad(Auto, Some("sc2")), SteamController2);
        assert_eq!(pick_gamepad(SteamController2Puck, None), SteamController2Puck);
        assert_eq!(pick_gamepad(Auto, Some("sc2puck")), SteamController2Puck);
    }

    // The conflict gate keys on the exact Valve PID, so a physical Steam Controller 2 (`28DE:1302`)
    // never blocks a virtual Steam Deck (`28DE:1205`) — only a same-identity duplicate degrades.
    #[cfg(target_os = "linux")]
    #[test]
    fn steam_backend_product_ids() {
        use super::steam_backend_product;
        use GamepadPref::*;
        assert_eq!(steam_backend_product(SteamDeck), Some(0x1205));
        assert_eq!(steam_backend_product(SteamController), Some(0x1102));
        assert_eq!(steam_backend_product(SteamController2), Some(0x1302));
        assert_eq!(steam_backend_product(SteamController2Puck), Some(0x1304));
        // Non-Steam backends carry no Valve identity, so the gate never fires for them.
        assert_eq!(steam_backend_product(DualSense), None);
        assert_eq!(steam_backend_product(Xbox360), None);
        assert_eq!(steam_backend_product(SwitchPro), None);
    }
}
