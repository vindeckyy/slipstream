//! `slipstream-host plugins ...` manages the Linux plugin runner.
//!
//! Package operations are delegated to the scripting runner. Service operations use the
//! per-user systemd unit so the CLI and management API share the same state transitions.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const UNIT: &str = "slipstream-scripting";

pub fn main(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("add") | Some("remove") | Some("rm") | Some("uninstall") | Some("list")
        | Some("ls") => forward_to_runner(args),
        Some("enable") => enable(),
        Some("disable") => disable(),
        Some("status") => status(),
        Some("-h") | Some("--help") | Some("help") | None => {
            print_usage();
            Ok(())
        }
        Some(other) => bail!("unknown plugins command '{other}' (try `plugins --help`)"),
    }
}

fn print_usage() {
    eprintln!(
        "slipstream-host plugins - install and run host plugins

USAGE:
    slipstream-host plugins add <name...>       install a plugin
    slipstream-host plugins remove <name...>    uninstall a plugin
    slipstream-host plugins list                list installed plugins
    slipstream-host plugins enable              enable and start the plugin runner
    slipstream-host plugins disable             stop and disable the plugin runner
    slipstream-host plugins status              show runner state

Plugins are operator-installed code. Install only plugins you trust.
"
    );
}

fn forward_to_runner(args: &[String]) -> Result<()> {
    if args.first().map(String::as_str) == Some("add") {
        let dir = args
            .iter()
            .position(|arg| arg == "--plugins")
            .and_then(|index| args.get(index + 1))
            .map(PathBuf::from)
            .unwrap_or_else(crate::store::plugins_dir);
        crate::store::ensure_plugin_root(&dir)
            .with_context(|| format!("prepare {}", dir.display()))?;
    }

    let (program, prefix) = runner_command()?;
    let status = Command::new(&program)
        .args(&prefix)
        .args(args)
        .status()
        .with_context(|| format!("failed to run the plugin runner ({})", program.display()))?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

/// Resolve the installed scripting runner as a program plus optional leading arguments.
pub(crate) fn runner_command() -> Result<(PathBuf, Vec<String>)> {
    let wrapper = PathBuf::from("/usr/bin/slipstream-scripting");
    if wrapper.exists() {
        return Ok((wrapper, Vec::new()));
    }

    let bun = PathBuf::from("/usr/lib/slipstream-scripting/bun");
    let runner = PathBuf::from("/usr/share/slipstream-scripting/runner-cli.js");
    if bun.exists() && runner.exists() {
        return Ok((bun, vec![runner.to_string_lossy().into_owned()]));
    }

    if let Ok(home) = std::env::var("HOME") {
        let home = Path::new(&home);
        let wrapper = home.join(".local/bin/slipstream-scripting");
        if wrapper.exists() {
            return Ok((wrapper, Vec::new()));
        }

        let bun = home.join(".local/lib/slipstream-scripting/bun");
        let runner = home.join(".local/share/slipstream-scripting/runner-cli.js");
        if bun.exists() && runner.exists() {
            return Ok((bun, vec![runner.to_string_lossy().into_owned()]));
        }
    }

    bail!(
        "the plugin runner is not installed; install slipstream-scripting or run the SteamOS installer"
    )
}

fn enable() -> Result<()> {
    run_systemctl(&["enable", "--now", UNIT])?;
    println!("Plugin runner enabled and started ({UNIT}).");
    Ok(())
}

fn disable() -> Result<()> {
    run_systemctl(&["disable", "--now", UNIT])?;
    println!("Plugin runner stopped and disabled ({UNIT}).");
    Ok(())
}

fn status() -> Result<()> {
    let state = runtime_status();
    println!(
        "runner:  {}\nstate:   {}\nenabled: {}",
        state.unit,
        if !state.installed {
            "not installed"
        } else if state.running {
            "running"
        } else {
            "stopped"
        },
        state.enabled
    );
    if state.installed && !state.running {
        println!("\nStart it with: slipstream-host plugins enable");
    } else if !state.installed {
        println!("\n{}", state.detail);
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeStatus {
    pub installed: bool,
    pub enabled: bool,
    pub running: bool,
    pub unit: &'static str,
    pub principal: Option<String>,
    pub detail: String,
}

pub(crate) fn runtime_status() -> RuntimeStatus {
    let enabled_raw = systemctl_output(&["is-enabled", UNIT]);
    let active = systemctl_output(&["is-active", UNIT]).unwrap_or_default();
    let unit_known = enabled_raw
        .as_deref()
        .is_some_and(|value| value != "not-found");
    let installed = unit_known || runner_command().is_ok();
    RuntimeStatus {
        installed,
        enabled: enabled_raw.as_deref() == Some("enabled"),
        running: active == "active",
        unit: UNIT,
        principal: None,
        detail: if installed {
            String::new()
        } else {
            "the plugin runner package is not installed; install slipstream-scripting".into()
        },
    }
}

pub(crate) fn set_runtime_enabled(enabled: bool) -> Result<()> {
    if enabled {
        enable()
    } else {
        disable()
    }
}

pub(crate) fn restart_runtime() -> Result<bool> {
    let state = runtime_status();
    if !state.installed || !state.running {
        return Ok(false);
    }
    run_systemctl(&["restart", UNIT])?;
    Ok(true)
}

fn run_systemctl(args: &[&str]) -> Result<()> {
    let status = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .status()
        .context("failed to run systemctl (is systemd available in this session?)")?;
    if !status.success() {
        bail!(
            "systemctl --user {} failed; is the slipstream-scripting package installed?",
            args.join(" ")
        );
    }
    Ok(())
}

fn systemctl_output(args: &[&str]) -> Option<String> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .ok()?;
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}
