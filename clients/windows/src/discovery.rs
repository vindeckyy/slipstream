//! LAN discovery: browse `_slipstream._udp` (the same service name every other client browses)
//! and map each resolved record through the shared [`HostAdvert`] decoder.

use crate::connect::HostAdvert;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};

/// The TXT keys this client reads. Anything else in the record is ignored, so a newer host
/// advertising extra capability keys does not break an older client.
const TXT_KEYS: [&str; 6] = ["proto", "id", "fp", "pair", "mac", "os"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredHost {
    pub name: String,
    pub addr: SocketAddr,
    pub advert: HostAdvert,
}

impl DiscoveredHost {
    /// The de-duplication key: the stable `id` when advertised, else the service fullname.
    pub fn key(&self, fullname: &str) -> String {
        if self.advert.id.is_empty() {
            fullname.to_string()
        } else {
            self.advert.id.clone()
        }
    }

    /// One console row: name, addr, pairing requirement, fingerprint stub.
    pub fn row(&self) -> String {
        let fp = &self.advert.fp;
        format!(
            "  {:<24} {}:{:<6} pair={:<9} fp={}…",
            self.name,
            self.addr.ip(),
            self.addr.port(),
            if self.advert.allows_tofu() {
                "optional"
            } else {
                "required"
            },
            fp.get(..16).unwrap_or(fp.as_str()),
        )
    }
}

/// Pick the address to connect to: prefer the first IPv4 (the data plane is UDP over v4 on the
/// host today), else the first address at all.
pub fn pick_addr(addrs: &[IpAddr], port: u16) -> Option<SocketAddr> {
    addrs
        .iter()
        .find(|a| a.is_ipv4())
        .or_else(|| addrs.first())
        .map(|ip| SocketAddr::new(*ip, port))
}

/// Browse for `secs` and return every host found, keyed so re-adverts on several interfaces
/// collapse to one entry.
pub fn browse(secs: u64) -> Result<Vec<DiscoveredHost>> {
    use mdns_sd::{ServiceDaemon, ServiceEvent};
    use std::time::{Duration, Instant};

    let daemon = ServiceDaemon::new().context("create mDNS daemon")?;
    let receiver = daemon
        .browse("_slipstream._udp.local.")
        .context("browse _slipstream._udp")?;
    let mut hosts: BTreeMap<String, DiscoveredHost> = BTreeMap::new();
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        // A timeout equal to the time left means `recv` returns Err exactly at the deadline.
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let props = info.get_properties();
                let entries: Vec<(String, String)> = TXT_KEYS
                    .iter()
                    .map(|k| {
                        (
                            k.to_string(),
                            props
                                .get_property_val_str(k)
                                .unwrap_or("")
                                .trim()
                                .to_string(),
                        )
                    })
                    .collect();
                let advert = HostAdvert::from_txt(&entries);
                let addrs: Vec<IpAddr> = info
                    .get_addresses()
                    .iter()
                    .filter_map(|a| a.to_string().parse().ok())
                    .collect();
                let Some(addr) = pick_addr(&addrs, info.get_port()) else {
                    continue;
                };
                let fullname = info.get_fullname().to_string();
                let name = fullname.split('.').next().unwrap_or("?").to_string();
                let host = DiscoveredHost { name, addr, advert };
                hosts.insert(host.key(&fullname), host);
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    let _ = daemon.shutdown();
    Ok(hosts.into_values().collect())
}

/// `--discover`: print what is on the LAN and exit without connecting.
pub fn run(secs: u64) -> Result<()> {
    tracing::info!(
        secs,
        "browsing for native slipstream/1 hosts (_slipstream._udp)…"
    );
    let hosts = browse(secs)?;
    if hosts.is_empty() {
        println!("no native slipstream/1 hosts found on the LAN ({secs}s)");
        return Ok(());
    }
    println!("native slipstream/1 hosts ({}):", hosts.len());
    for h in &hosts {
        println!("{}", h.row());
    }
    println!("\nconnect with: slipstream-windows <addr:port> [--pin <fp> | --pair <PIN>]");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn host(ip: &str, port: u16) -> DiscoveredHost {
        DiscoveredHost {
            name: "box".into(),
            addr: SocketAddr::new(ip.parse().unwrap(), port),
            advert: HostAdvert::from_txt(&[]),
        }
    }

    #[test]
    fn ipv4_is_preferred_over_ipv6() {
        let v6: IpAddr = "fd00::5".parse().unwrap();
        let v4: IpAddr = Ipv4Addr::new(192, 168, 1, 9).into();
        assert_eq!(pick_addr(&[v6, v4], 9777), Some(SocketAddr::new(v4, 9777)));
        assert_eq!(pick_addr(&[], 9777), None);
        assert_eq!(pick_addr(&[v6], 9777), Some(SocketAddr::new(v6, 9777)));
    }

    #[test]
    fn key_falls_back_to_fullname_without_an_id() {
        let h = host("10.0.0.5", 9777);
        assert_eq!(
            h.key("box._slipstream._udp.local."),
            "box._slipstream._udp.local."
        );
        let mut known = h.clone();
        known.advert.id = "8c1f".into();
        assert_eq!(known.key("ignored"), "8c1f");
    }

    #[test]
    fn row_reports_the_pairing_policy_and_a_fp_stub() {
        let row = host("10.0.0.5", 9777).row();
        assert!(row.contains("10.0.0.5:9777"), "{row}");
        assert!(row.contains("pair=required"), "{row}");

        let mut optional = host("10.0.0.6", 9777);
        optional.advert.pair = crate::connect::PairingPolicy::Optional;
        optional.advert.fp = "0123456789abcdefdeadbeef".into();
        let row = optional.row();
        assert!(row.contains("pair=optional"), "{row}");
        assert!(
            row.contains("fp=0123456789abcdef…"),
            "fp is truncated to 16 chars: {row}"
        );
    }
}
