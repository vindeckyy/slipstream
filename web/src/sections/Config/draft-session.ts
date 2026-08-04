/** Browser-session draft for the Configuration page (sessionStorage only). */

import type { HostConfigFile } from "@/api/host-config";

export const CONFIG_DRAFT_MARKER_KEY = "slipstream.config.draft.marker";
export const CONFIG_DRAFT_PAYLOAD_KEY = "slipstream.config.draft.payload";

function sessionStore(): Storage | null {
	try {
		if (typeof globalThis.sessionStorage === "undefined") return null;
		return globalThis.sessionStorage;
	} catch {
		return null;
	}
}

export function hasConfigDraftMarker(
	store: Pick<Storage, "getItem"> | null = sessionStore(),
): boolean {
	if (!store) return false;
	try {
		return store.getItem(CONFIG_DRAFT_MARKER_KEY) === "1";
	} catch {
		return false;
	}
}

export function readConfigDraft<T>(
	store: Pick<Storage, "getItem"> | null = sessionStore(),
): T | null {
	if (!store || !hasConfigDraftMarker(store)) return null;
	try {
		const raw = store.getItem(CONFIG_DRAFT_PAYLOAD_KEY);
		if (!raw) return null;
		return JSON.parse(raw) as T;
	} catch {
		return null;
	}
}

export function writeConfigDraft(
	draft: unknown,
	store: Pick<Storage, "setItem"> | null = sessionStore(),
): void {
	if (!store) return;
	try {
		store.setItem(CONFIG_DRAFT_MARKER_KEY, "1");
		store.setItem(CONFIG_DRAFT_PAYLOAD_KEY, JSON.stringify(draft));
	} catch {
		// Private mode / quota: keep editing in memory only.
	}
}

export function clearConfigDraft(
	store: Pick<Storage, "removeItem"> | null = sessionStore(),
): void {
	if (!store) return;
	try {
		store.removeItem(CONFIG_DRAFT_MARKER_KEY);
		store.removeItem(CONFIG_DRAFT_PAYLOAD_KEY);
	} catch {
		// Ignore.
	}
}

/** Merge a saved draft onto fresh server settings without dropping new fields or defaults. */
export function restoreConfigDraft(
	baseline: HostConfigFile,
	saved: Partial<HostConfigFile> | null,
): HostConfigFile {
	if (!saved) return structuredClone(baseline);
	return {
		...structuredClone(baseline),
		...saved,
		general: { ...baseline.general, ...saved.general },
		input: { ...baseline.input, ...saved.input },
		audio_video: { ...baseline.audio_video, ...saved.audio_video },
		network: { ...baseline.network, ...saved.network },
		encoders: { ...baseline.encoders, ...saved.encoders },
		clipboard: saved.clipboard ?? baseline.clipboard,
		performance_profile:
			saved.performance_profile ?? baseline.performance_profile,
		latency_profile: saved.latency_profile ?? baseline.latency_profile,
		network_policy: saved.network_policy ?? baseline.network_policy,
	};
}
