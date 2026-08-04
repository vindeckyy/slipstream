//! Shared Exclusive-session monitor hold for Linux: DDC panel power + DRM connector force-off.
//!
//! Call [`arm_before_topology`] while physical heads are still active (DDC), then
//! [`arm_after_topology`] after the compositor Exclusive (or for the standby-TV selector under
//! any topology). Hand the returned [`Hold`] into the group's topology-restore closure via
//! [`restore`] so last-out wakes panels and re-detects connectors.

use crate::policy::prefs;

/// State captured for one display group's hold; restored once on last teardown.
#[derive(Default)]
pub struct Hold {
    /// Whether DDC DPMS-off was attempted (so restore can wake).
    pub ddc_armed: bool,
    /// DRM connectors forced off (sysfs names like `card1-DP-1`).
    pub drm_forced: Vec<String>,
}

/// Before Exclusive topology: darken panels over DDC/CI when the policy axis is on.
pub fn arm_before_topology(exclusive: bool) {
    if exclusive && prefs().ddc_power_off() {
        let _ = crate::ddc::panel_off_except("");
    }
}

/// After topology apply (or on first member for any topology when PnP axis is on): force-off
/// connected external DRM connectors. Merges into `hold`.
pub fn arm_after_topology(hold: &mut Hold, exclusive: bool) {
    if exclusive && prefs().ddc_power_off() {
        hold.ddc_armed = true;
    }
    // Standby-TV selector: any topology, same as Windows `disable_connected_inactive`.
    if prefs().pnp_disable_monitors() {
        let forced = crate::drm_force::force_off_connected_external();
        for n in forced {
            if !hold.drm_forced.contains(&n) {
                hold.drm_forced.push(n);
            }
        }
    }
}

/// Reverse [`arm_before_topology`] / [`arm_after_topology`] on last-out.
pub fn restore(hold: Hold) {
    if !hold.drm_forced.is_empty() {
        crate::drm_force::restore(&hold.drm_forced);
    }
    if hold.ddc_armed {
        let _ = crate::ddc::panel_on_all();
    }
}
