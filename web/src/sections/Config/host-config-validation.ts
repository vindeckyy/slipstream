/**
 * Client-side validation for the host configuration draft.
 *
 * Mirrors the authoritative Rust constraints in
 * `crates/slipstream-host/src/host_config_file.rs::field_errors`. The backend
 * stays the write authority; this exists so Save is disabled and errors appear
 * at their field before any request is sent.
 */
import type { HostConfigDraft } from "./host-config-draft";

const ENCODER_VALUES = new Set(["auto", "nvenc", "amf", "qsv", "vaapi", "software"]);
const VIDEO_SOURCE_VALUES = new Set(["virtual", "portal"]);
const CAPTURE_METHOD_VALUES = new Set([
	"auto",
	"portal",
	"kwin",
	"wlr",
	"kms",
	"x11",
	"nvfbc",
]);
const COMPOSITOR_VALUES = new Set([
	"kwin",
	"kde",
	"plasma",
	"mutter",
	"gnome",
	"wlroots",
	"wlr",
	"sway",
	"river",
	"hyprland",
	"hypr",
	"gamescope",
]);
const HEADLESS_VALUES = new Set(["off", "auto", "labwc", "krfb", "gamescope"]);
const AUDIO_CAPTURE_VALUES = new Set(["stream-sink", "monitor"]);

function isIntegerText(value: string): boolean {
	return /^\d+$/.test(value.trim());
}

function isDecimalText(value: string): boolean {
	return /^-?\d+(\.\d+)?$/.test(value.trim());
}

/** Field-keyed issues; blank is valid only for nullable numeric fields. */
export function validateHostConfigDraft(
	draft: HostConfigDraft,
): Record<string, string> {
	const errors: Record<string, string> = {};

	const hasControl = (s: string) => [...s].some((ch) => ch.charCodeAt(0) < 32);

	if (draft.general.host_name.length > 128) {
		errors["general.host_name"] = "At most 128 characters.";
	} else if (hasControl(draft.general.host_name)) {
		errors["general.host_name"] = "No control characters.";
	}

	if (!VIDEO_SOURCE_VALUES.has(draft.audio_video.video_source)) {
		errors["audio_video.video_source"] = "Choose Virtual display or Portal.";
	}
	if (
		draft.audio_video.capture_method &&
		!CAPTURE_METHOD_VALUES.has(draft.audio_video.capture_method)
	) {
		errors["audio_video.capture_method"] = "Unsupported capture backend.";
	}
	if (
		draft.audio_video.compositor &&
		!COMPOSITOR_VALUES.has(draft.audio_video.compositor)
	) {
		errors["audio_video.compositor"] = "Unsupported compositor.";
	}
	if (!HEADLESS_VALUES.has(draft.audio_video.headless_compositor)) {
		errors["audio_video.headless_compositor"] = "Unsupported headless backend.";
	}

	const range = (
		field: string,
		value: string,
		min: number,
		max: number,
		nullable = true,
	) => {
		const t = value.trim();
		if (t === "" && nullable) return;
		if (!isIntegerText(value)) {
			errors[field] = "Enter a whole number.";
			return;
		}
		const n = Number(t);
		if (n < min || n > max) {
			errors[field] = `Between ${min} and ${max}.`;
		}
	};
	range("audio_video.max_fps", draft.audio_video.max_fps, 15, 240);
	range(
		"audio_video.pipewire_latency_ms",
		draft.audio_video.pipewire_latency_ms,
		1,
		40,
	);
	range(
		"audio_video.capture_max_age_ms",
		draft.audio_video.capture_max_age_ms,
		1,
		500,
	);
	range("audio_video.vdisplay_hz_mult", draft.audio_video.vdisplay_hz_mult, 1, 4, false);
	range(
		"audio_video.gamescope_sdr_nits",
		draft.audio_video.gamescope_sdr_nits,
		1,
		10_000,
	);
	range("network.fec_pct", draft.network.fec_pct, 0, 90);

	const gain = draft.audio_video.audio_gain.trim();
	if (gain !== "") {
		if (!isDecimalText(gain)) {
			errors["audio_video.audio_gain"] = "Enter a number.";
		} else {
			const n = Number(gain);
			if (n < 0 || n > 4) {
				errors["audio_video.audio_gain"] = "Between 0 and 4.";
			}
		}
	}

	if (!AUDIO_CAPTURE_VALUES.has(draft.audio_video.audio_capture)) {
		errors["audio_video.audio_capture"] = "Unsupported capture source.";
	}
	if (!ENCODER_VALUES.has(draft.encoders.encoder)) {
		errors["encoders.encoder"] = "Unsupported encoder.";
	}
	if (hasControl(draft.encoders.render_adapter)) {
		errors["encoders.render_adapter"] = "No control characters.";
	}

	return errors;
}
