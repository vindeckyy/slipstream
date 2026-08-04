//! Persisted host configuration (`<config>/host-config.json`) plus a dual-write to `host.env`
//! so systemd / the Windows service pick up the same knobs on the next start.
//!
//! The management console edits this file via `GET/PUT /api/v1/host/config`. Runtime
//! `ss_host_config::HostConfig` still parses the process environment once at startup, so
//! most changes report `requires_restart: true` until the host is restarted.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use utoipa::ToSchema;

fn one() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ClipboardPolicy {
    #[default]
    Off,
    TextOnly,
    On,
}

impl ClipboardPolicy {
    fn env_value(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::TextOnly => "text-only",
            Self::On => "on",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PerformanceProfile {
    #[default]
    Balanced,
    LowLatency,
}

impl PerformanceProfile {
    fn env_value(self) -> &'static str {
        match self {
            Self::Balanced => "off",
            Self::LowLatency => "low_latency",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LatencyProfile {
    #[default]
    Balanced,
    LowLatency,
}

impl LatencyProfile {
    fn env_value(self) -> &'static str {
        match self {
            Self::Balanced => "balanced",
            Self::LowLatency => "low_latency",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPolicy {
    #[default]
    Auto,
    Lan,
    Wan,
}

impl NetworkPolicy {
    fn env_value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Lan => "lan",
            Self::Wan => "wan",
        }
    }
}

/// Sunshine-shaped host settings the console can toggle without editing env files by hand.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct HostConfigFile {
    #[serde(default = "one")]
    pub version: u32,
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub input: InputConfig,
    #[serde(default)]
    pub audio_video: AudioVideoConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub encoders: EncoderConfig,
    /// Host clipboard policy. The client must also enable clipboard sharing for a session.
    #[serde(default)]
    pub clipboard: ClipboardPolicy,
    /// Named worker scheduling profile (`SLIPSTREAM_PERFORMANCE_PROFILE`).
    #[serde(default)]
    pub performance_profile: PerformanceProfile,
    /// Named encoder latency profile (`SLIPSTREAM_LATENCY_PROFILE`).
    #[serde(default)]
    pub latency_profile: LatencyProfile,
    /// Named transport starting policy (`SLIPSTREAM_NETWORK_POLICY`).
    #[serde(default)]
    pub network_policy: NetworkPolicy,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct GeneralConfig {
    /// Display name for Moonlight / mDNS (`SLIPSTREAM_HOST_NAME`).
    pub host_name: Option<String>,
    /// Verbose perf logging (`SLIPSTREAM_PERF`).
    #[serde(default)]
    pub perf: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct InputConfig {
    /// Gamepad backend preference (`SLIPSTREAM_GAMEPAD`).
    pub gamepad: Option<String>,
    /// Advertise full-fidelity pen/stylus input (`SLIPSTREAM_PEN`). Default on.
    #[serde(default = "default_true")]
    pub pen: bool,
    /// Gamescope: grab cursor into the nested session.
    /// Default off (matches runtime `SLIPSTREAM_GAMESCOPE_GRAB_CURSOR`).
    #[serde(default)]
    pub gamescope_grab_cursor: bool,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            gamepad: None,
            pen: true,
            gamescope_grab_cursor: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct AudioVideoConfig {
    /// `virtual` | `portal` (`SLIPSTREAM_VIDEO_SOURCE`).
    pub video_source: Option<String>,
    /// Desktop capture backend (`SLIPSTREAM_CAPTURE_METHOD`):
    /// `auto` | `portal` | `kwin` | `wlr` | `kms` | `x11` | `nvfbc` (no hermes-kms).
    pub capture_method: Option<String>,
    /// Virtual-display compositor preference (`SLIPSTREAM_COMPOSITOR`):
    /// `kwin` | `mutter` | `wlroots` | `hyprland` | `gamescope`.
    pub compositor: Option<String>,
    /// Headless session spawner (`SLIPSTREAM_HEADLESS_COMPOSITOR`):
    /// `off` | `auto` | `labwc` | `krfb` | `gamescope`.
    pub headless_compositor: Option<String>,
    /// Cap encode FPS (`SLIPSTREAM_MAX_FPS`).
    pub max_fps: Option<u32>,
    /// Prefer 10-bit encode when the client asks (`SLIPSTREAM_10BIT`). Default on.
    #[serde(default = "default_true")]
    pub ten_bit: bool,
    /// Prefer 4:4:4 when supported (`SLIPSTREAM_444`). Default on.
    #[serde(default = "default_true")]
    pub four_four_four: bool,
    /// Gamescope HDR. Default on.
    #[serde(default = "default_true")]
    pub gamescope_hdr: bool,
    /// Requested PipeWire video-node latency in milliseconds. This is a scheduling hint, not a
    /// guarantee from the compositor.
    #[serde(default)]
    #[schema(minimum = 1, maximum = 40)]
    pub pipewire_latency_ms: Option<u32>,
    /// Capture-frame age threshold used by the Linux latency diagnostics.
    #[serde(default)]
    #[schema(minimum = 1, maximum = 500)]
    pub capture_max_age_ms: Option<u32>,
    /// Audio FEC over the native plane (`SLIPSTREAM_AUDIO_FEC`). Default on: RS parity over
    /// groups of 5 ms Opus frames so a lost packet is rebuilt instead of clicking. Off only
    /// as an escape hatch.
    #[serde(default = "default_true")]
    pub audio_fec: bool,
    /// Linear audio gain applied to captured samples (`SLIPSTREAM_AUDIO_GAIN`, default 1.0).
    /// For quiet sources; 1.0 = unchanged, 0.5 = half, 2.0 = double.
    #[serde(default)]
    #[schema(minimum = 0.0, maximum = 4.0)]
    pub audio_gain: Option<f32>,
    /// Linux audio capture source (`SLIPSTREAM_STREAM_SINK`): `stream-sink` (default, a
    /// host-owned sink apps play into) or `monitor` (record the default sink).
    #[serde(default)]
    pub audio_capture: Option<String>,
    /// Keep a bare Gamescope session painting during application startup.
    /// Default on because a blank Gamescope session produces no capture buffers.
    #[serde(default = "default_true")]
    pub gamescope_splash: bool,
    /// Virtual-display refresh multiplier (`SLIPSTREAM_VDISPLAY_HZ_MULT`), from 1x to 4x.
    #[serde(default = "one")]
    pub vdisplay_hz_mult: u32,
    /// SDR luminance inside an HDR Gamescope session (`SLIPSTREAM_GAMESCOPE_SDR_NITS`).
    #[serde(default)]
    #[schema(minimum = 1, maximum = 10000)]
    pub gamescope_sdr_nits: Option<u32>,
}

impl Default for AudioVideoConfig {
    fn default() -> Self {
        Self {
            video_source: None,
            capture_method: None,
            compositor: None,
            headless_compositor: None,
            max_fps: None,
            ten_bit: true,
            four_four_four: true,
            gamescope_hdr: true,
            pipewire_latency_ms: None,
            capture_max_age_ms: None,
            audio_fec: true,
            audio_gain: None,
            audio_capture: None,
            gamescope_splash: true,
            vdisplay_hz_mult: 1,
            gamescope_sdr_nits: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct NetworkConfig {
    /// Prefer ChaCha20-Poly1305 for soft-AES clients (`SLIPSTREAM_CHACHA20`). Default on.
    #[serde(default = "default_true")]
    pub chacha20: bool,
    /// Run and advertise the GameStream/Moonlight compatibility plane on the next host start.
    #[serde(default)]
    pub gamestream: Option<bool>,
    /// Advertise over mDNS (`SLIPSTREAM_MDNS`). Default on.
    #[serde(default = "default_true")]
    pub mdns: bool,
    /// FEC percentage for the native plane (`SLIPSTREAM_FEC_PCT`), when set.
    pub fec_pct: Option<u32>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            chacha20: true,
            gamestream: None,
            mdns: true,
            fec_pct: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct EncoderConfig {
    /// `auto` | `nvenc` | `amf` | `qsv` | `vaapi` | `software` (`SLIPSTREAM_ENCODER`).
    #[serde(default = "default_encoder")]
    pub encoder: String,
    /// Substring pin for the render adapter (`SLIPSTREAM_RENDER_ADAPTER`).
    pub render_adapter: Option<String>,
    /// Tri-state zero-copy override (`SLIPSTREAM_ZEROCOPY`). `null` = vendor default.
    pub zerocopy: Option<bool>,
}

fn default_encoder() -> String {
    "auto".into()
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            encoder: default_encoder(),
            render_adapter: None,
            zerocopy: None,
        }
    }
}

impl Default for HostConfigFile {
    fn default() -> Self {
        Self {
            version: 1,
            general: GeneralConfig::default(),
            input: InputConfig::default(),
            audio_video: AudioVideoConfig::default(),
            network: NetworkConfig::default(),
            encoders: EncoderConfig::default(),
            clipboard: ClipboardPolicy::default(),
            performance_profile: PerformanceProfile::default(),
            latency_profile: LatencyProfile::default(),
            network_policy: NetworkPolicy::default(),
        }
    }
}

impl HostConfigFile {
    /// Validate operator input before it is normalized and persisted.
    ///
    /// Loading an older file still uses [`Self::sanitized`] for compatibility, but API writes
    /// should report the offending fields instead of silently changing the requested settings.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.version != 1 {
            errors.push(format!("version must be 1 (got {})", self.version));
        }
        if let Some(name) = self.general.host_name.as_deref() {
            if name.chars().count() > 128 {
                errors.push("general.host_name must be at most 128 characters".into());
            }
            if name.chars().any(char::is_control) {
                errors.push("general.host_name must not contain control characters".into());
            }
        }
        if let Some(value) = self.audio_video.video_source.as_deref() {
            if !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "virtual" | "portal"
            ) {
                errors.push("audio_video.video_source must be virtual or portal".into());
            }
        }
        if let Some(value) = self.audio_video.capture_method.as_deref() {
            if !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "auto" | "portal" | "kwin" | "wlr" | "kms" | "x11" | "nvfbc"
            ) {
                errors.push("audio_video.capture_method is not a supported backend".into());
            }
        }
        if let Some(value) = self.audio_video.compositor.as_deref() {
            if !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "kwin"
                    | "kde"
                    | "plasma"
                    | "mutter"
                    | "gnome"
                    | "wlroots"
                    | "wlr"
                    | "sway"
                    | "river"
                    | "hyprland"
                    | "hypr"
                    | "gamescope"
            ) {
                errors.push("audio_video.compositor is not a supported backend".into());
            }
        }
        if let Some(value) = self.audio_video.headless_compositor.as_deref() {
            if !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "off" | "auto" | "labwc" | "krfb" | "gamescope"
            ) {
                errors.push("audio_video.headless_compositor is not a supported backend".into());
            }
        }
        if let Some(value) = self.audio_video.max_fps {
            if !(15..=480).contains(&value) {
                errors.push("audio_video.max_fps must be between 15 and 480".into());
            }
        }
        if let Some(value) = self.audio_video.pipewire_latency_ms {
            if !(1..=40).contains(&value) {
                errors.push("audio_video.pipewire_latency_ms must be between 1 and 40".into());
            }
        }
        if let Some(value) = self.audio_video.capture_max_age_ms {
            if !(1..=500).contains(&value) {
                errors.push("audio_video.capture_max_age_ms must be between 1 and 500".into());
            }
        }
        if !(1..=4).contains(&self.audio_video.vdisplay_hz_mult) {
            errors.push("audio_video.vdisplay_hz_mult must be between 1 and 4".into());
        }
        if let Some(value) = self.audio_video.gamescope_sdr_nits {
            if !(1..=10_000).contains(&value) {
                errors.push("audio_video.gamescope_sdr_nits must be between 1 and 10000".into());
            }
        }
        if let Some(value) = self.audio_video.audio_gain {
            if !(0.0..=4.0).contains(&value) || !value.is_finite() {
                errors.push("audio_video.audio_gain must be between 0 and 4".into());
            }
        }
        if let Some(value) = self.audio_video.audio_capture.as_deref() {
            if !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "stream-sink" | "monitor"
            ) {
                errors.push("audio_video.audio_capture must be stream-sink or monitor".into());
            }
        }
        if let Some(value) = self.network.fec_pct {
            if value > 90 {
                errors.push("network.fec_pct must be between 0 and 90".into());
            }
        }
        let encoder = self.encoders.encoder.trim().to_ascii_lowercase();
        if !matches!(
            encoder.as_str(),
            "auto" | "nvenc" | "amf" | "qsv" | "vaapi" | "software"
        ) {
            errors.push("encoders.encoder is not a supported encoder".into());
        }
        if self
            .encoders
            .render_adapter
            .as_deref()
            .is_some_and(|value| value.chars().any(char::is_control))
        {
            errors.push("encoders.render_adapter must not contain control characters".into());
        }
        errors
    }

    pub fn sanitized(mut self) -> Self {
        self.version = 1;
        if self.encoders.encoder.trim().is_empty() {
            self.encoders.encoder = default_encoder();
        } else {
            self.encoders.encoder = self.encoders.encoder.trim().to_ascii_lowercase();
        }
        if let Some(fps) = self.audio_video.max_fps {
            self.audio_video.max_fps = Some(fps.clamp(15, 480));
        }
        if let Some(value) = self.audio_video.pipewire_latency_ms {
            self.audio_video.pipewire_latency_ms = Some(value.clamp(1, 40));
        }
        if let Some(value) = self.audio_video.capture_max_age_ms {
            self.audio_video.capture_max_age_ms = Some(value.clamp(1, 500));
        }
        self.audio_video.vdisplay_hz_mult = self.audio_video.vdisplay_hz_mult.clamp(1, 4);
        if let Some(value) = self.audio_video.gamescope_sdr_nits {
            self.audio_video.gamescope_sdr_nits = Some(value.clamp(1, 10_000));
        }
        if let Some(pct) = self.network.fec_pct {
            self.network.fec_pct = Some(pct.min(90));
        }
        if let Some(name) = self.general.host_name.as_mut() {
            let trimmed = name.trim().to_string();
            if trimmed.is_empty() {
                self.general.host_name = None;
            } else {
                *name = trimmed;
            }
        }
        self
    }

    /// KEY=VALUE lines for `host.env` (systemd EnvironmentFile / Windows service loader).
    pub fn to_host_env(&self) -> String {
        let mut out = String::from(
            "# Generated by slipstream-host from host-config.json. Edit via the web console.\n",
        );
        fn set(buf: &mut String, key: &str, val: &str) {
            let _ = writeln!(buf, "{key}={val}");
        }
        fn set_bool(buf: &mut String, key: &str, on: bool) {
            set(buf, key, if on { "1" } else { "0" });
        }
        if let Some(name) = &self.general.host_name {
            set(&mut out, "SLIPSTREAM_HOST_NAME", name);
        }
        set_bool(&mut out, "SLIPSTREAM_PERF", self.general.perf);
        if let Some(gp) = &self.input.gamepad {
            if !gp.trim().is_empty() {
                set(&mut out, "SLIPSTREAM_GAMEPAD", gp.trim());
            }
        }
        set_bool(&mut out, "SLIPSTREAM_PEN", self.input.pen);
        set_bool(
            &mut out,
            "SLIPSTREAM_GAMESCOPE_GRAB_CURSOR",
            self.input.gamescope_grab_cursor,
        );
        if let Some(src) = &self.audio_video.video_source {
            if !src.trim().is_empty() {
                set(&mut out, "SLIPSTREAM_VIDEO_SOURCE", src.trim());
            }
        }
        if let Some(m) = &self.audio_video.capture_method {
            let m = m.trim().to_ascii_lowercase();
            if !m.is_empty() {
                set(&mut out, "SLIPSTREAM_CAPTURE_METHOD", &m);
            }
        }
        if let Some(c) = &self.audio_video.compositor {
            let c = c.trim().to_ascii_lowercase();
            if !c.is_empty() {
                set(&mut out, "SLIPSTREAM_COMPOSITOR", &c);
            }
        }
        if let Some(h) = &self.audio_video.headless_compositor {
            let h = h.trim().to_ascii_lowercase();
            if !h.is_empty() && h != "off" {
                set(&mut out, "SLIPSTREAM_HEADLESS_COMPOSITOR", &h);
            }
        }
        if let Some(fps) = self.audio_video.max_fps {
            set(&mut out, "SLIPSTREAM_MAX_FPS", &fps.to_string());
        }
        set_bool(&mut out, "SLIPSTREAM_10BIT", self.audio_video.ten_bit);
        set_bool(&mut out, "SLIPSTREAM_444", self.audio_video.four_four_four);
        set_bool(
            &mut out,
            "SLIPSTREAM_GAMESCOPE_HDR",
            self.audio_video.gamescope_hdr,
        );
        if let Some(value) = self.audio_video.pipewire_latency_ms {
            set(
                &mut out,
                "SLIPSTREAM_PIPEWIRE_LATENCY_MS",
                &value.to_string(),
            );
        }
        if let Some(value) = self.audio_video.capture_max_age_ms {
            set(
                &mut out,
                "SLIPSTREAM_CAPTURE_MAX_AGE_MS",
                &value.to_string(),
            );
        }
        set_bool(&mut out, "SLIPSTREAM_AUDIO_FEC", self.audio_video.audio_fec);
        if let Some(gain) = self.audio_video.audio_gain {
            set(&mut out, "SLIPSTREAM_AUDIO_GAIN", &gain.to_string());
        }
        if let Some(src) = &self.audio_video.audio_capture {
            let src = src.trim().to_ascii_lowercase();
            if !src.is_empty() {
                // stream-sink is the default; only the monitor fallback needs the env var.
                if src == "monitor" {
                    set(&mut out, "SLIPSTREAM_STREAM_SINK", "0");
                } else {
                    set(&mut out, "SLIPSTREAM_STREAM_SINK", "1");
                }
            }
        }
        set_bool(
            &mut out,
            "SLIPSTREAM_GAMESCOPE_SPLASH",
            self.audio_video.gamescope_splash,
        );
        set(
            &mut out,
            "SLIPSTREAM_VDISPLAY_HZ_MULT",
            &self.audio_video.vdisplay_hz_mult.to_string(),
        );
        if let Some(value) = self.audio_video.gamescope_sdr_nits {
            set(
                &mut out,
                "SLIPSTREAM_GAMESCOPE_SDR_NITS",
                &value.to_string(),
            );
        }
        set_bool(&mut out, "SLIPSTREAM_CHACHA20", self.network.chacha20);
        if let Some(gamestream) = self.network.gamestream {
            set_bool(&mut out, "SLIPSTREAM_GAMESTREAM", gamestream);
        }
        set_bool(&mut out, "SLIPSTREAM_MDNS", self.network.mdns);
        if let Some(pct) = self.network.fec_pct {
            set(&mut out, "SLIPSTREAM_FEC_PCT", &pct.to_string());
        }
        set(&mut out, "SLIPSTREAM_ENCODER", &self.encoders.encoder);
        if let Some(adapter) = &self.encoders.render_adapter {
            if !adapter.trim().is_empty() {
                set(&mut out, "SLIPSTREAM_RENDER_ADAPTER", adapter.trim());
            }
        }
        match self.encoders.zerocopy {
            Some(true) => set(&mut out, "SLIPSTREAM_ZEROCOPY", "1"),
            Some(false) => set(&mut out, "SLIPSTREAM_ZEROCOPY", "0"),
            None => {}
        }
        set(&mut out, "SLIPSTREAM_CLIPBOARD", self.clipboard.env_value());
        set(
            &mut out,
            "SLIPSTREAM_PERFORMANCE_PROFILE",
            self.performance_profile.env_value(),
        );
        set(
            &mut out,
            "SLIPSTREAM_LATENCY_PROFILE",
            self.latency_profile.env_value(),
        );
        set(
            &mut out,
            "SLIPSTREAM_NETWORK_POLICY",
            self.network_policy.env_value(),
        );
        out
    }
}

pub struct HostConfigStore {
    path: PathBuf,
    env_path: PathBuf,
    cur: Mutex<Option<HostConfigFile>>,
}

impl HostConfigStore {
    pub fn load_from(path: PathBuf, env_path: PathBuf) -> Self {
        let cur = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<HostConfigFile>(&bytes) {
                Ok(s) => Some(s.sanitized()),
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        "host-config.json unreadable — using built-in defaults: {e}"
                    );
                    None
                }
            },
            Err(_) => None,
        };
        Self {
            path,
            env_path,
            cur: Mutex::new(cur),
        }
    }

    pub fn get(&self) -> HostConfigFile {
        self.cur
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .unwrap_or_default()
    }

    pub fn configured(&self) -> bool {
        self.cur.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    pub fn env_path(&self) -> &std::path::Path {
        &self.env_path
    }

    pub fn set(&self, mut settings: HostConfigFile) -> Result<()> {
        // GameStream ownership lives on the Host page. Preserve the current value when a
        // configuration draft from another route is saved.
        settings.network.gamestream = self
            .cur
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(|current| current.network.gamestream);
        self.persist(settings)
    }

    pub fn set_moonlight(&self, enabled: bool) -> Result<()> {
        let mut settings = self.get();
        settings.network.gamestream = Some(enabled);
        if enabled {
            settings.network.mdns = true;
        }
        self.persist(settings)
    }

    fn persist(&self, settings: HostConfigFile) -> Result<()> {
        let errors = settings.validate();
        if !errors.is_empty() {
            anyhow::bail!("invalid host configuration: {}", errors.join("; "));
        }
        let settings = settings.sanitized();
        if let Some(dir) = self.path.parent() {
            ss_paths::create_private_dir(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        ss_paths::write_secret_file(&tmp, &serde_json::to_vec_pretty(&settings)?)?;
        std::fs::rename(&tmp, &self.path)?;

        let env_tmp = self.env_path.with_extension("env.tmp");
        ss_paths::write_secret_file(&env_tmp, settings.to_host_env().as_bytes())?;
        std::fs::rename(&env_tmp, &self.env_path)?;

        *self.cur.lock().unwrap_or_else(|e| e.into_inner()) = Some(settings);
        Ok(())
    }
}

pub fn store() -> &'static HostConfigStore {
    static STORE: OnceLock<HostConfigStore> = OnceLock::new();
    STORE.get_or_init(|| {
        let dir = ss_paths::config_dir();
        HostConfigStore::load_from(dir.join("host-config.json"), dir.join("host.env"))
    })
}

pub fn get() -> HostConfigFile {
    store().get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_env_round_trip_keys() {
        let mut cfg = HostConfigFile::default();
        cfg.general.host_name = Some("Living Room".into());
        cfg.general.perf = true;
        cfg.encoders.encoder = "nvenc".into();
        cfg.encoders.zerocopy = Some(false);
        cfg.network.gamestream = Some(true);
        cfg.network.fec_pct = Some(30);
        let env = cfg.to_host_env();
        assert!(env.contains("SLIPSTREAM_HOST_NAME=Living Room"));
        assert!(env.contains("SLIPSTREAM_PERF=1"));
        assert!(env.contains("SLIPSTREAM_ENCODER=nvenc"));
        assert!(env.contains("SLIPSTREAM_ZEROCOPY=0"));
        assert!(env.contains("SLIPSTREAM_GAMESTREAM=1"));
        assert!(env.contains("SLIPSTREAM_FEC_PCT=30"));
        assert!(env.contains("SLIPSTREAM_CLIPBOARD=off"));
        assert!(env.contains("SLIPSTREAM_PEN=1"));
        assert!(env.contains("SLIPSTREAM_GAMESCOPE_SPLASH=1"));
        assert!(env.contains("SLIPSTREAM_VDISPLAY_HZ_MULT=1"));
        assert!(env.contains("SLIPSTREAM_PERFORMANCE_PROFILE=off"));
        assert!(env.contains("SLIPSTREAM_LATENCY_PROFILE=balanced"));
        assert!(env.contains("SLIPSTREAM_NETWORK_POLICY=auto"));
    }

    #[test]
    fn durable_options_round_trip_through_json_and_host_env() {
        let mut cfg = HostConfigFile::default();
        cfg.input.pen = false;
        cfg.audio_video.gamescope_splash = false;
        cfg.audio_video.vdisplay_hz_mult = 4;
        cfg.audio_video.gamescope_sdr_nits = Some(275);
        cfg.clipboard = ClipboardPolicy::TextOnly;
        cfg.performance_profile = PerformanceProfile::LowLatency;
        cfg.latency_profile = LatencyProfile::LowLatency;
        cfg.network_policy = NetworkPolicy::Wan;

        let json = serde_json::to_string(&cfg).unwrap();
        let decoded: HostConfigFile = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, cfg);

        let env = cfg.to_host_env();
        assert!(env.contains("SLIPSTREAM_PEN=0"));
        assert!(env.contains("SLIPSTREAM_GAMESCOPE_SPLASH=0"));
        assert!(env.contains("SLIPSTREAM_VDISPLAY_HZ_MULT=4"));
        assert!(env.contains("SLIPSTREAM_GAMESCOPE_SDR_NITS=275"));
        assert!(env.contains("SLIPSTREAM_CLIPBOARD=text-only"));
        assert!(env.contains("SLIPSTREAM_PERFORMANCE_PROFILE=low_latency"));
        assert!(env.contains("SLIPSTREAM_LATENCY_PROFILE=low_latency"));
        assert!(env.contains("SLIPSTREAM_NETWORK_POLICY=wan"));
    }

    #[test]
    fn host_env_writes_slipstream_10bit_not_ten_bit() {
        let mut cfg = HostConfigFile::default();
        cfg.audio_video.ten_bit = true;
        let env = cfg.to_host_env();
        assert!(
            env.contains("SLIPSTREAM_10BIT=1"),
            "runtime reads SLIPSTREAM_10BIT; got:\n{env}"
        );
        assert!(
            !env.contains("SLIPSTREAM_TEN_BIT="),
            "stale TEN_BIT key must not be written; got:\n{env}"
        );

        cfg.audio_video.ten_bit = false;
        let env = cfg.to_host_env();
        assert!(
            env.contains("SLIPSTREAM_10BIT=0"),
            "explicit off must write 0; got:\n{env}"
        );
        assert!(
            !env.contains("SLIPSTREAM_TEN_BIT="),
            "stale TEN_BIT key must not be written; got:\n{env}"
        );
    }

    #[test]
    fn default_host_config_matches_runtime_policy_gates() {
        let cfg = HostConfigFile::default();
        // Runtime HostConfig defaults (ss-host-config): 10-bit / 4:4:4 / ChaCha20 /
        // Gamescope HDR on; Gamescope cursor grab off; audio FEC + mDNS on.
        assert!(cfg.audio_video.ten_bit);
        assert!(cfg.audio_video.four_four_four);
        assert!(cfg.audio_video.gamescope_hdr);
        assert!(cfg.audio_video.audio_fec);
        assert!(cfg.network.chacha20);
        assert!(cfg.network.mdns);
        assert!(!cfg.input.gamescope_grab_cursor);
        assert!(cfg.input.pen);
        assert!(cfg.audio_video.gamescope_splash);
        assert_eq!(cfg.audio_video.vdisplay_hz_mult, 1);
        assert_eq!(cfg.clipboard, ClipboardPolicy::Off);
        assert_eq!(cfg.performance_profile, PerformanceProfile::Balanced);
        assert_eq!(cfg.latency_profile, LatencyProfile::Balanced);
        assert_eq!(cfg.network_policy, NetworkPolicy::Auto);
    }

    #[test]
    fn omitted_json_fields_keep_aligned_defaults() {
        let cfg: HostConfigFile = serde_json::from_str(r#"{"version":1}"#).unwrap();
        assert!(cfg.audio_video.ten_bit);
        assert!(cfg.audio_video.four_four_four);
        assert!(cfg.audio_video.gamescope_hdr);
        assert!(cfg.audio_video.audio_fec);
        assert!(cfg.network.chacha20);
        assert!(cfg.network.mdns);
        assert!(!cfg.input.gamescope_grab_cursor);
    }

    #[test]
    fn explicit_json_values_override_serde_defaults() {
        let cfg: HostConfigFile = serde_json::from_str(
            r#"{
                "version": 1,
				"input": { "pen": false, "gamescope_grab_cursor": true },
                "audio_video": {
                    "ten_bit": false,
                    "four_four_four": false,
                    "gamescope_hdr": false,
                    "audio_fec": false
                },
                "network": {
                    "chacha20": false,
                    "mdns": false
                }
            }"#,
        )
        .unwrap();
        assert!(!cfg.audio_video.ten_bit);
        assert!(!cfg.audio_video.four_four_four);
        assert!(!cfg.audio_video.gamescope_hdr);
        assert!(!cfg.audio_video.audio_fec);
        assert!(!cfg.network.chacha20);
        assert!(!cfg.network.mdns);
        assert!(cfg.input.gamescope_grab_cursor);
        assert!(!cfg.input.pen);
    }

    #[test]
    fn sanitize_clamps_fps() {
        let mut cfg = HostConfigFile::default();
        cfg.audio_video.max_fps = Some(9999);
        let s = cfg.sanitized();
        assert_eq!(s.audio_video.max_fps, Some(480));
    }

    #[test]
    fn sanitize_clamps_latency_diagnostics() {
        let mut cfg = HostConfigFile::default();
        cfg.audio_video.pipewire_latency_ms = Some(9999);
        cfg.audio_video.capture_max_age_ms = Some(9999);
        let s = cfg.sanitized();
        assert_eq!(s.audio_video.pipewire_latency_ms, Some(40));
        assert_eq!(s.audio_video.capture_max_age_ms, Some(500));
    }

    #[test]
    fn validate_reports_field_errors() {
        let mut cfg = HostConfigFile::default();
        cfg.audio_video.capture_method = Some("broken".into());
        cfg.audio_video.max_fps = Some(1);
        cfg.network.fec_pct = Some(91);
        let errors = cfg.validate();
        assert_eq!(errors.len(), 3);
        assert!(errors.iter().any(|e| e.contains("capture_method")));
        assert!(errors.iter().any(|e| e.contains("max_fps")));
        assert!(errors.iter().any(|e| e.contains("fec_pct")));
    }
}
