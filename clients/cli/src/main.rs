//! `slipstream` — the headless client CLI (design/client-architecture-split.md §4).
//!
//! One console-subsystem binary over the same brain the GUI shells use, so a script gets the
//! same behaviour a click does — including wake-then-connect, which the Linux shell's old
//! exec-style `--connect` never had. It is a FRONT-END, not the brain: policy lives in
//! `ss_client_core`, and the shells call the same functions in-process rather than shelling
//! out to this. That is the whole point of the split — if the GUI shelled out for connects,
//! trust prompts and wake progress would have to squeeze through an IPC contract.
//!
//! Existing surfaces are a frozen compatibility contract and are NOT replaced by this: the
//! Linux shell keeps its headless flags (Decky invokes them), and `slipstream-probe` stays the
//! diagnostics tool. New integrations should use `slipstream library <host> --json`.
//!
//! Exit codes extend the session binary's: 0 ok, 2 connect failed, 3 trust rejected,
//! 4 renderer failed, 5 could not resolve what was asked for, 6 refused because it needs a
//! human (pairing, an unknown host). A machine consumer can branch on those without parsing
//! prose.

#![forbid(unsafe_code)]

#[cfg(target_os = "linux")]
#[path = "cmd/mod.rs"]
mod cli;

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    // Logs to stderr; stdout is the machine interface (TSV/JSON), exactly like the session
    // binary's contract.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::ExitCode::from(cli::run(args))
}

/// Keeps `cargo build --workspace` green on macOS, where the client is clients/apple.
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("slipstream CLI runs on Linux; the macOS client lives in clients/apple");
    std::process::exit(2);
}
