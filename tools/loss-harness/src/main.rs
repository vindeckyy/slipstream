//! `loss-harness` — sweep packet loss against the FEC and report recovery (plan §10).
//!
//! Drives access units through the in-process loopback at increasing loss rates, for
//! both FEC schemes, and prints how many frames survive. A pure-software stand-in for
//! `tc netem` that needs no network and runs anywhere `slipstream_core` builds. The real slipstream/1
//! harness adds `tc netem` jitter/reorder on the UDP path.
#![forbid(unsafe_code)]

mod harness;

use slipstream_core::config::FecScheme;

fn main() {
    let frames = 50;
    let frame_len = 100_000; // ~98 shards across 2 FEC blocks
    let periods = [0u32, 32, 16, 8, 6, 4, 3, 2];

    println!("slipstream loss-harness — 25% FEC, {frames} frames of {frame_len} bytes");
    println!("(GF8 = P1/GameStream-compat, GF16 = P2/wall-breaker, strm = streamed-AU wire)\n");
    println!(
        "{:>10}  {:>9}  {:>14}  {:>14}  {:>14}",
        "drop 1/N", "~loss %", "GF8 recovered", "GF16 recovered", "GF16 strm"
    );
    println!("{}", "-".repeat(72));
    for &p in &periods {
        let loss = if p == 0 { 0.0 } else { 100.0 / p as f64 };
        let (g8, n) = harness::run(FecScheme::Gf8, p, frames, frame_len, false);
        let (g16, _) = harness::run(FecScheme::Gf16, p, frames, frame_len, false);
        let (g16s, _) = harness::run(FecScheme::Gf16, p, frames, frame_len, true);
        let label = if p == 0 {
            "none".to_string()
        } else {
            format!("1/{p}")
        };
        println!(
            "{label:>10}  {loss:>8.1}%  {:>11}/{n}  {:>11}/{n}  {:>11}/{n}",
            g8, g16, g16s
        );
    }
    println!("\nNote: recovery drops off once per-block loss exceeds the 25% recovery budget.");
}
