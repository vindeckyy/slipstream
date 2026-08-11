/**
 * Host configuration draft: one complete, required form shape derived from the
 * generated `HostConfigFile`, with editable numeric fields kept as strings so
 * partial input survives typing and validates before save.
 *
 * The generated OpenAPI models are intentionally all-optional (Serde defaults),
 * so `normalizeHostConfig` fills the exact defaults from the Rust
 * `HostConfigFile::default` and the form edits `HostConfigDraft` (required
 * fields, numeric strings). `serializeHostConfigDraft` maps back to the wire
 * type, omitting blank optional values as `null`.
 */
import type {
	AudioVideoConfig,
	EncoderConfig,
	GeneralConfig,
	HostConfigFile,
	InputConfig,
	NetworkConfig,
} from "@/api/gen/model";

export type HostConfigDraft = {
	version: 1;
	general: {
		host_name: string;
		perf: boolean;
	};
	input: {
		gamepad: string;
		pen: boolean;
		gamescope_grab_cursor: boolean;
		hide_host_cursor: boolean;
	};
	audio_video: {
		video_source: string;
		capture_method: string;
		compositor: string;
		headless_compositor: string;
		max_fps: string;
		pipewire_latency_ms: string;
		capture_max_age_ms: string;
		ten_bit: boolean;
		four_four_four: boolean;
		gamescope_hdr: boolean;
		audio_fec: boolean;
		audio_gain: string;
		audio_capture: string;
		gamescope_splash: boolean;
		vdisplay_hz_mult: string;
		gamescope_sdr_nits: string;
	};
	network: {
		chacha20: boolean;
		gamestream: boolean;
		mdns: boolean;
		fec_pct: string;
	};
	encoders: {
		encoder: string;
		render_adapter: string;
		zerocopy: string;
	};
	clipboard: string;
	performance_profile: string;
	latency_profile: string;
	network_policy: string;
};

function str(value: string | null | undefined): string {
	return value ?? "";
}

function bool(value: boolean | undefined, fallback: boolean): boolean {
	return value ?? fallback;
}

/** Map a generated optional `HostConfigFile` onto the complete editable draft. */
export function normalizeHostConfig(settings: HostConfigFile): HostConfigDraft {
	const g: GeneralConfig = settings.general ?? {};
	const i: InputConfig = settings.input ?? {};
	const av: AudioVideoConfig = settings.audio_video ?? {};
	const n: NetworkConfig = settings.network ?? {};
	const e: EncoderConfig = settings.encoders ?? {};
	return {
		version: 1,
		general: {
			host_name: str(g.host_name),
			perf: bool(g.perf, false),
		},
		input: {
			gamepad: str(i.gamepad),
			pen: bool(i.pen, true),
			gamescope_grab_cursor: bool(i.gamescope_grab_cursor, false),
			hide_host_cursor: bool(i.hide_host_cursor, true),
		},
		audio_video: {
			video_source: str(av.video_source) || "virtual",
			capture_method: str(av.capture_method) || "auto",
			compositor: str(av.compositor),
			headless_compositor: str(av.headless_compositor) || "off",
			max_fps: av.max_fps == null ? "" : String(av.max_fps),
			pipewire_latency_ms:
				av.pipewire_latency_ms == null ? "" : String(av.pipewire_latency_ms),
			capture_max_age_ms:
				av.capture_max_age_ms == null ? "" : String(av.capture_max_age_ms),
			ten_bit: bool(av.ten_bit, true),
			four_four_four: bool(av.four_four_four, true),
			gamescope_hdr: bool(av.gamescope_hdr, true),
			audio_fec: bool(av.audio_fec, true),
			audio_gain: av.audio_gain == null ? "" : String(av.audio_gain),
			audio_capture: str(av.audio_capture) || "stream-sink",
			gamescope_splash: bool(av.gamescope_splash, true),
			vdisplay_hz_mult: String(av.vdisplay_hz_mult ?? 1),
			gamescope_sdr_nits:
				av.gamescope_sdr_nits == null ? "" : String(av.gamescope_sdr_nits),
		},
		network: {
			chacha20: bool(n.chacha20, true),
			gamestream: bool(n.gamestream ?? false, false),
			mdns: bool(n.mdns, true),
			fec_pct: n.fec_pct == null ? "" : String(n.fec_pct),
		},
		encoders: {
			encoder: str(e.encoder) || "auto",
			render_adapter: str(e.render_adapter),
			zerocopy:
				e.zerocopy == null ? "" : e.zerocopy ? "1" : "0",
		},
		clipboard: settings.clipboard ?? "off",
		performance_profile: settings.performance_profile ?? "balanced",
		latency_profile: settings.latency_profile ?? "balanced",
		network_policy: settings.network_policy ?? "auto",
	};
}

function num(value: string): number | null {
	const t = value.trim();
	if (t === "") return null;
	const n = Number(t);
	return Number.isFinite(n) ? n : null;
}

/** Map the validated draft back onto the wire `HostConfigFile`. */
export function serializeHostConfigDraft(
	draft: HostConfigDraft,
): HostConfigFile {
	return {
		version: 1,
		general: {
			host_name: draft.general.host_name.trim() || null,
			perf: draft.general.perf,
		},
		input: {
			gamepad: draft.input.gamepad.trim() || null,
			pen: draft.input.pen,
			gamescope_grab_cursor: draft.input.gamescope_grab_cursor,
			hide_host_cursor: draft.input.hide_host_cursor,
		},
		audio_video: {
			video_source: draft.audio_video.video_source,
			capture_method: draft.audio_video.capture_method || null,
			compositor: draft.audio_video.compositor.trim() || null,
			headless_compositor: draft.audio_video.headless_compositor,
			max_fps: num(draft.audio_video.max_fps),
			pipewire_latency_ms: num(draft.audio_video.pipewire_latency_ms),
			capture_max_age_ms: num(draft.audio_video.capture_max_age_ms),
			ten_bit: draft.audio_video.ten_bit,
			four_four_four: draft.audio_video.four_four_four,
			gamescope_hdr: draft.audio_video.gamescope_hdr,
			audio_fec: draft.audio_video.audio_fec,
			audio_gain: num(draft.audio_video.audio_gain),
			audio_capture: draft.audio_video.audio_capture,
			gamescope_splash: draft.audio_video.gamescope_splash,
			vdisplay_hz_mult: Number(draft.audio_video.vdisplay_hz_mult) || 1,
			gamescope_sdr_nits: num(draft.audio_video.gamescope_sdr_nits),
		},
		network: {
			chacha20: draft.network.chacha20,
			mdns: draft.network.mdns,
			fec_pct: num(draft.network.fec_pct),
		},
		encoders: {
			encoder: draft.encoders.encoder,
			render_adapter: draft.encoders.render_adapter.trim() || null,
			zerocopy:
				draft.encoders.zerocopy === "" ? null : draft.encoders.zerocopy === "1",
		},
		clipboard: draft.clipboard as HostConfigFile["clipboard"],
		performance_profile:
			draft.performance_profile as HostConfigFile["performance_profile"],
		latency_profile: draft.latency_profile as HostConfigFile["latency_profile"],
		network_policy: draft.network_policy as HostConfigFile["network_policy"],
	};
}
