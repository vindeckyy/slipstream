//! Title launch: resolve a library id or raw command into a Linux session command.

use super::custom::valid_steam_appid;
#[cfg(target_os = "linux")]
use super::heroic::heroic_command;
use super::*;

/// Everything a session needs about the title it is launching, resolved in one library scan.
pub struct LaunchTarget {
    /// Identity for the status surface and the `game.*` events.
    pub game: crate::gamelease::GameRef,
    /// How to recognize the running game.
    pub detect: DetectSpec,
    /// The resolved shell command run by the Linux host.
    pub command: Option<String>,
}

/// Resolve a store-qualified library id against the host's own library. A client can select an
/// existing title, but cannot inject a command.
pub fn resolve_launch(id: &str) -> Option<LaunchTarget> {
    let entry = all_games().into_iter().find(|g| g.id == id)?;
    let game = crate::gamelease::GameRef {
        id: Some(entry.id.clone()),
        store: Some(entry.store.clone()),
        title: entry.title.clone(),
    };
    let command = entry.launch.as_ref().and_then(command_for)?;
    Some(LaunchTarget {
        game,
        detect: entry.detect,
        command: Some(command),
    })
}

/// Map a resolved [`LaunchSpec`] to its Linux shell command.
fn command_for(spec: &LaunchSpec) -> Option<String> {
    match spec.kind.as_str() {
        "steam_appid" => valid_steam_appid(&spec.value)
            .then(|| format!("steam steam://rungameid/{}", spec.value)),
        "lutris_id" => (!spec.value.is_empty() && spec.value.bytes().all(|b| b.is_ascii_digit()))
            .then(|| format!("lutris lutris:rungameid/{}", spec.value)),
        "heroic" => heroic_command(&spec.value),
        "command" => (!spec.value.trim().is_empty()).then(|| spec.value.clone()),
        _ => None,
    }
}

/// The child a session launch produced.
#[cfg(target_os = "linux")]
pub struct SpawnedLaunch {
    pub child: std::process::Child,
    /// Whether the child leads its own process group.
    pub group_leader: bool,
}

/// Launch a resolved shell command into the live Linux session for the session's compositor.
#[cfg(target_os = "linux")]
pub fn launch_session_command(
    compositor: crate::vdisplay::Compositor,
    cmd: &str,
) -> Result<SpawnedLaunch> {
    use std::os::unix::process::CommandExt;
    let cmd = cmd.trim();
    anyhow::ensure!(!cmd.is_empty(), "empty command");
    let (child, group_leader) = match compositor {
        crate::vdisplay::Compositor::Gamescope => {
            (crate::vdisplay::launch_into_gamescope_session(cmd)?, false)
        }
        _ => (
            std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .process_group(0)
                .spawn()
                .context("spawn launch command")?,
            true,
        ),
    };
    tracing::info!(
        command = %cmd,
        pid = child.id(),
        compositor = compositor.id(),
        "launched app into the live session"
    );
    Ok(SpawnedLaunch {
        child,
        group_leader,
    })
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn launch_command_resolves_and_guards() {
        assert_eq!(
            command_for(&LaunchSpec {
                kind: "steam_appid".into(),
                value: "570".into(),
            })
            .as_deref(),
            Some("steam steam://rungameid/570")
        );
        assert_eq!(
            command_for(&LaunchSpec {
                kind: "steam_appid".into(),
                value: "570; rm -rf ~".into(),
            }),
            None
        );
        assert_eq!(
            command_for(&LaunchSpec {
                kind: "command".into(),
                value: "dolphin-emu --batch".into(),
            })
            .as_deref(),
            Some("dolphin-emu --batch")
        );
        assert_eq!(
            command_for(&LaunchSpec {
                kind: "command".into(),
                value: "  ".into(),
            }),
            None
        );
    }

    #[test]
    fn command_for_linux_store_guards() {
        assert_eq!(
            command_for(&LaunchSpec {
                kind: "lutris_id".into(),
                value: "42".into(),
            })
            .as_deref(),
            Some("lutris lutris:rungameid/42")
        );
        assert_eq!(
            command_for(&LaunchSpec {
                kind: "lutris_id".into(),
                value: "42; rm -rf ~".into(),
            }),
            None
        );
        assert_eq!(heroic_command("badrunner:Quail"), None);
        assert_eq!(heroic_command("legendary:bad name"), None);
        assert_eq!(heroic_command("nile:"), None);
        if let Some(cmd) = heroic_command("legendary:Quail-1.2_x") {
            assert!(cmd.contains("heroic://launch?appName=Quail-1.2_x&runner=legendary"));
            assert!(cmd.contains("--no-gui"));
        }
    }
}
