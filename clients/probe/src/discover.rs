//! LAN mDNS discovery (`--discover`).

use anyhow::{Context, Result};

pub(crate) fn discover(secs: u64) -> Result<()> {
    use mdns_sd::{ServiceDaemon, ServiceEvent};
    use std::collections::BTreeMap;
    use std::time::{Duration, Instant};

    let daemon = ServiceDaemon::new().context("create mDNS daemon")?;
    let receiver = daemon
        .browse("_slipstream._udp.local.")
        .context("browse _slipstream._udp")?;
    tracing::info!(
        secs,
        "browsing for native slipstream/1 hosts (_slipstream._udp)…"
    );
    // One row per host, keyed by the stable uniqueid (falls back to the fullname) so the same
    // host re-advertising or answering on several interfaces collapses to a single entry.
    let mut hosts: BTreeMap<String, String> = BTreeMap::new();
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        // Timeout == time left to the deadline: an event returns immediately, otherwise the recv
        // returns Err exactly at the deadline (or if the daemon channel closes) and we stop.
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let props = info.get_properties();
                let val = |k: &str| props.get_property_val_str(k).unwrap_or("").to_string();
                let addr = info
                    .get_addresses()
                    .iter()
                    .next()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "?".into());
                let fp = val("fp");
                let fp_short = fp.get(..16).unwrap_or(fp.as_str());
                let name = info
                    .get_fullname()
                    .split('.')
                    .next()
                    .unwrap_or("?")
                    .to_string();
                let id = val("id");
                let key = if id.is_empty() {
                    info.get_fullname().to_string()
                } else {
                    id
                };
                let row = format!(
                    "  {name:<24} {addr}:{:<6} pair={:<9} fp={fp_short}…",
                    info.get_port(),
                    val("pair"),
                );
                hosts.insert(key, row);
            }
            Ok(_) => {} // SearchStarted / ServiceFound / removals — ignore
            Err(_) => break,
        }
    }
    let _ = daemon.shutdown();
    if hosts.is_empty() {
        println!("no native slipstream/1 hosts found on the LAN ({secs}s)");
    } else {
        println!("native slipstream/1 hosts ({}):", hosts.len());
        for row in hosts.values() {
            println!("{row}");
        }
        println!(
            "\nconnect with: slipstream-probe --connect <addr:port> [--pin <fp> | --pair <PIN>]"
        );
    }
    Ok(())
}

