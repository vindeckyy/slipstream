import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { describe, test } from "node:test";
import type { HostConfigFile } from "@/api/gen/model";
import {
	normalizeHostConfig,
	serializeHostConfigDraft,
	type HostConfigDraft,
} from "./host-config-draft";
import { validateHostConfigDraft } from "./host-config-validation";

const EMPTY_WIRE: HostConfigFile = { version: 1 };

describe("host config draft", () => {
	test("normalize fills the documented defaults", () => {
		const d = normalizeHostConfig(EMPTY_WIRE);
		strictEqual(d.general.host_name, "");
		strictEqual(d.input.pen, true);
		strictEqual(d.input.hide_host_cursor, true);
		strictEqual(d.audio_video.ten_bit, true);
		strictEqual(d.audio_video.four_four_four, true);
		strictEqual(d.audio_video.gamescope_hdr, true);
		strictEqual(d.audio_video.audio_fec, true);
		strictEqual(d.audio_video.gamescope_splash, true);
		strictEqual(d.audio_video.vdisplay_hz_mult, "1");
		strictEqual(d.audio_video.video_source, "virtual");
		strictEqual(d.audio_video.audio_capture, "stream-sink");
		strictEqual(d.audio_video.headless_compositor, "off");
		strictEqual(d.network.chacha20, true);
		strictEqual(d.network.mdns, true);
		strictEqual(d.clipboard, "off");
		strictEqual(d.performance_profile, "balanced");
		strictEqual(d.latency_profile, "balanced");
		strictEqual(d.network_policy, "auto");
	});

	test("numeric strings survive and serialize back to null when blank", () => {
		const d = normalizeHostConfig(EMPTY_WIRE);
		strictEqual(d.audio_video.max_fps, "");
		const wire = serializeHostConfigDraft(d);
		strictEqual(wire.audio_video?.max_fps, null);
		strictEqual(wire.audio_video?.audio_gain, null);
		strictEqual(wire.audio_video?.gamescope_sdr_nits, null);
		strictEqual(wire.network?.fec_pct, null);
		strictEqual(wire.encoders?.zerocopy, null);
	});

	test("round-trips an explicit configuration", () => {
		const wire: HostConfigFile = {
			version: 1,
			general: { host_name: "Living Room", perf: true },
			input: { pen: false, hide_host_cursor: false, gamescope_grab_cursor: true },
			audio_video: {
				video_source: "portal",
				capture_method: "kwin",
				max_fps: 240,
				audio_gain: 1.5,
				vdisplay_hz_mult: 2,
				gamescope_sdr_nits: 275,
			},
			network: { chacha20: false, mdns: false, fec_pct: 30 },
			encoders: { encoder: "nvenc", render_adapter: "NVIDIA", zerocopy: true },
			clipboard: "text-only",
			performance_profile: "low_latency",
			latency_profile: "low_latency",
			network_policy: "wan",
		};
		const d = normalizeHostConfig(wire);
		strictEqual(d.general.host_name, "Living Room");
		strictEqual(d.audio_video.max_fps, "240");
		strictEqual(d.audio_video.audio_gain, "1.5");
		strictEqual(d.network.fec_pct, "30");
		strictEqual(d.encoders.zerocopy, "1");
		const back = serializeHostConfigDraft(d);
		deepStrictEqual(back.audio_video?.max_fps, 240);
		deepStrictEqual(back.audio_video?.audio_gain, 1.5);
		deepStrictEqual(back.network?.fec_pct, 30);
		deepStrictEqual(back.encoders?.zerocopy, true);
		deepStrictEqual(back.general?.host_name, "Living Room");
	});

	test("zerocopy tri-state maps both ways", () => {
		const on = normalizeHostConfig({
			version: 1,
			encoders: { encoder: "auto", zerocopy: false },
		});
		strictEqual(on.encoders.zerocopy, "0");
		strictEqual(serializeHostConfigDraft(on).encoders?.zerocopy, false);
	});
});

describe("host config validation", () => {
	const draft: HostConfigDraft = normalizeHostConfig(EMPTY_WIRE);

	test("accepts defaults and blank nullable numerics", () => {
		deepStrictEqual(validateHostConfigDraft(draft), {});
	});

	test("rejects out-of-range and malformed numerics", () => {
		const bad: HostConfigDraft = {
			...draft,
			audio_video: {
				...draft.audio_video,
				max_fps: "241",
				pipewire_latency_ms: "0",
				capture_max_age_ms: "501",
				vdisplay_hz_mult: "5",
				audio_gain: "-0.1",
				gamescope_sdr_nits: "0",
			},
			network: { ...draft.network, fec_pct: "91" },
		};
		const errors = validateHostConfigDraft(bad);
		strictEqual(errors["audio_video.max_fps"], "Between 15 and 240.");
		strictEqual(errors["audio_video.pipewire_latency_ms"], "Between 1 and 40.");
		strictEqual(errors["audio_video.capture_max_age_ms"], "Between 1 and 500.");
		strictEqual(errors["audio_video.vdisplay_hz_mult"], "Between 1 and 4.");
		strictEqual(errors["audio_video.audio_gain"], "Between 0 and 4.");
		strictEqual(errors["audio_video.gamescope_sdr_nits"], "Between 1 and 10000.");
		strictEqual(errors["network.fec_pct"], "Between 0 and 90.");
	});

	test("requires the non-nullable multiplier and rejects non-integers", () => {
		const bad: HostConfigDraft = {
			...draft,
			audio_video: { ...draft.audio_video, vdisplay_hz_mult: "", max_fps: "12.5" },
		};
		const errors = validateHostConfigDraft(bad);
		strictEqual(errors["audio_video.vdisplay_hz_mult"], "Enter a whole number.");
		strictEqual(errors["audio_video.max_fps"], "Enter a whole number.");
	});

	test("rejects hostname length, control chars, and bad enums", () => {
		const bad: HostConfigDraft = {
			...draft,
			general: { ...draft.general, host_name: "a".repeat(129) },
			encoders: { ...draft.encoders, encoder: "broken", render_adapter: "bad\u0000" },
			audio_video: {
				...draft.audio_video,
				video_source: "broken",
				capture_method: "broken",
				audio_capture: "broken",
			},
		};
		const errors = validateHostConfigDraft(bad);
		strictEqual(errors["general.host_name"], "At most 128 characters.");
		strictEqual(errors["audio_video.video_source"], "Choose Virtual display or Portal.");
		strictEqual(errors["audio_video.capture_method"], "Unsupported capture backend.");
		strictEqual(errors["audio_video.audio_capture"], "Unsupported capture source.");
		strictEqual(errors["encoders.encoder"], "Unsupported encoder.");
		strictEqual(errors["encoders.render_adapter"], "No control characters.");
	});
});
