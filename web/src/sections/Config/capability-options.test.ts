import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { describe, test } from "node:test";
import type { HostConfigFile } from "@/api/host-config";
import {
	buildCaptureMethodOptions,
	buildCompositorOptions,
	buildHeadlessCompositorOptions,
	formatCapabilityOptionLabel,
	type CapabilityCopy,
} from "./capability-options";

const TEST_COPY: CapabilityCopy = {
	savedUnavailable:
		"Saved on this host, but not reported as available right now.",
	detectedDefault: "Detected default on this host.",
	unavailable: "Not available on this host right now.",
	autoDetect: "Auto-detect",
	autoDetectHelp: "Detect the running compositor automatically.",
	headlessOff: "Off",
	headlessOffHelp:
		"Do not spawn a private compositor. Use when a desktop session is already running.",
};
import {
	clearConfigDraft,
	CONFIG_DRAFT_MARKER_KEY,
	CONFIG_DRAFT_PAYLOAD_KEY,
	hasConfigDraftMarker,
	readConfigDraft,
	restoreConfigDraft,
	writeConfigDraft,
} from "./draft-session";

function memoryStore(initial: Record<string, string> = {}) {
	const data = { ...initial };
	return {
		getItem(key: string): string | null {
			return Object.hasOwn(data, key) ? (data[key] ?? null) : null;
		},
		setItem(key: string, value: string) {
			data[key] = value;
		},
		removeItem(key: string) {
			delete data[key];
		},
		dump: () => data,
	};
}

describe("config draft session marker", () => {
	test("treats a missing store as no draft", () => {
		strictEqual(hasConfigDraftMarker(null), false);
		strictEqual(readConfigDraft(null), null);
	});

	test("writes and clears a session draft", () => {
		const store = memoryStore();
		writeConfigDraft({ general: { host_name: "box" } }, store);
		strictEqual(store.dump()[CONFIG_DRAFT_MARKER_KEY], "1");
		deepStrictEqual(readConfigDraft(store), { general: { host_name: "box" } });
		clearConfigDraft(store);
		strictEqual(store.dump()[CONFIG_DRAFT_MARKER_KEY], undefined);
		strictEqual(store.dump()[CONFIG_DRAFT_PAYLOAD_KEY], undefined);
		strictEqual(hasConfigDraftMarker(store), false);
	});

	test("restores a draft onto the latest baseline without dropping fields", () => {
		const baseline: HostConfigFile = {
			version: 1,
			general: { host_name: null, perf: false },
			input: { gamepad: null, pen: true, gamescope_grab_cursor: false },
			audio_video: {
				video_source: "virtual",
				capture_method: "auto",
				compositor: null,
				headless_compositor: null,
				max_fps: null,
				pipewire_latency_ms: null,
				capture_max_age_ms: null,
				ten_bit: true,
				four_four_four: true,
				gamescope_hdr: true,
				audio_fec: true,
				audio_gain: null,
				audio_capture: null,
				gamescope_splash: true,
				vdisplay_hz_mult: 1,
				gamescope_sdr_nits: null,
			},
			network: {
				chacha20: true,
				gamestream: true,
				mdns: true,
				fec_pct: null,
			},
			encoders: { encoder: "auto", render_adapter: null, zerocopy: null },
			clipboard: "off",
			performance_profile: "balanced",
			latency_profile: "balanced",
			network_policy: "auto",
		};
		const saved = {
			...baseline,
			general: { ...baseline.general, host_name: "Draft host" },
			audio_video: { ...baseline.audio_video, gamescope_sdr_nits: 450 },
			network_policy: "wan" as const,
		};

		const restored = restoreConfigDraft(baseline, saved);
		strictEqual(restored.general.host_name, "Draft host");
		strictEqual(restored.audio_video.gamescope_sdr_nits, 450);
		strictEqual(restored.input.pen, true);
		strictEqual(restored.network.mdns, true);
		strictEqual(restored.network_policy, "wan");
		strictEqual(baseline.general.host_name, null);
	});
});

describe("capability options", () => {
	test("marks unavailable capture backends and keeps Auto", () => {
		const options = buildCaptureMethodOptions(
			[
				{ id: "auto", label: "Auto", available: true },
				{ id: "kwin", label: "KWin Screencast", available: false },
				{ id: "wlr", label: "wlroots screencopy", available: true },
			],
			"kwin",
			TEST_COPY,
		);
		const kwin = options.find((o) => o.value === "kwin");
		strictEqual(kwin?.available, false);
		strictEqual(options.find((o) => o.value === "auto")?.available, true);
	});

	test("keeps a selected unavailable compositor value visible", () => {
		const options = buildCompositorOptions(
			[{ id: "kwin", label: "KWin", available: true, default: true }],
			"mutter",
			TEST_COPY,
		);
		const mutter = options.find((o) => o.value === "mutter");
		strictEqual(mutter?.available, false);
		strictEqual(mutter?.value, "mutter");
		strictEqual(options[0]?.value, "");
		strictEqual(options[0]?.available, true);
	});

	test("falls back safely when capability probe fails", () => {
		const capture = buildCaptureMethodOptions(null, "auto", TEST_COPY);
		strictEqual(capture.find((o) => o.value === "auto")?.available, true);
		strictEqual(capture.find((o) => o.value === "nvfbc")?.available, false);

		const compositor = buildCompositorOptions(undefined, "", TEST_COPY);
		strictEqual(compositor[0]?.value, "");
		strictEqual(compositor.find((o) => o.value === "kwin")?.available, false);

		const headless = buildHeadlessCompositorOptions(null, "off", TEST_COPY);
		strictEqual(headless[0]?.value, "off");
		strictEqual(headless[0]?.available, true);
		strictEqual(headless.find((o) => o.value === "labwc")?.available, false);
	});

	test("marks headless Off only when it is selected", () => {
		const options = buildHeadlessCompositorOptions(
			[{ id: "gamescope", label: "Gamescope", available: true }],
			"gamescope",
			TEST_COPY,
		);
		strictEqual(options[0]?.marked, false);
		strictEqual(options.find((o) => o.value === "gamescope")?.available, true);
	});

	test("formats detected and unavailable labels", () => {
		strictEqual(
			formatCapabilityOptionLabel(
				{
					value: "kwin",
					label: "KWin",
					available: false,
					marked: true,
				},
				{ detected: "detected", unavailable: "unavailable" },
			),
			"KWin (detected) (unavailable)",
		);
	});
});
