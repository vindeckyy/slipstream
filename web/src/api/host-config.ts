/** Manual host-config client until orval regenerates from OpenAPI. */

export type HostConfigFile = {
	version: number;
	general: {
		host_name?: string | null;
		perf: boolean;
	};
	input: {
		gamepad?: string | null;
		pen: boolean;
		gamescope_grab_cursor: boolean;
	};
	audio_video: {
		video_source?: string | null;
		capture_method?: string | null;
		compositor?: string | null;
		headless_compositor?: string | null;
		max_fps?: number | null;
		pipewire_latency_ms?: number | null;
		capture_max_age_ms?: number | null;
		ten_bit: boolean;
		four_four_four: boolean;
		gamescope_hdr: boolean;
		audio_fec?: boolean;
		audio_gain?: number | null;
		audio_capture?: string | null;
		gamescope_splash: boolean;
		vdisplay_hz_mult: number;
		gamescope_sdr_nits?: number | null;
	};
	network: {
		chacha20: boolean;
		gamestream?: boolean | null;
		mdns: boolean;
		fec_pct?: number | null;
	};
	encoders: {
		encoder: string;
		render_adapter?: string | null;
		zerocopy?: boolean | null;
	};
	clipboard: ClipboardPolicy;
	performance_profile: PerformanceProfile;
	latency_profile: LatencyProfile;
	network_policy: NetworkPolicy;
};

export type ClipboardPolicy = "off" | "text-only" | "on";
export type PerformanceProfile = "balanced" | "low_latency";
export type LatencyProfile = "balanced" | "low_latency";
export type NetworkPolicy = "auto" | "lan" | "wan";

export const hostConfigQueryKey = ["host-config"] as const;
export const compositorsQueryKey = ["compositors"] as const;
export const captureMethodsQueryKey = ["capture-methods"] as const;
export const headlessCompositorsQueryKey = ["headless-compositors"] as const;

export type HostConfigState = {
	settings: HostConfigFile;
	configured: boolean;
	requires_restart: boolean;
	env_path: string;
};

/** Compositor backend from GET /api/v1/compositors. */
export type AvailableCompositor = {
	id: string;
	label: string;
	available: boolean;
	default: boolean;
};

/** Desktop capture method from GET /api/v1/capture/methods. */
export type AvailableCaptureMethod = {
	id: string;
	label: string;
	available: boolean;
};

/** Headless compositor from GET /api/v1/compositors/headless. */
export type AvailableHeadlessCompositor = {
	id: string;
	label: string;
	available: boolean;
};

async function api<T>(path: string, init?: RequestInit): Promise<T> {
	const res = await fetch(path, {
		credentials: "same-origin",
		headers: {
			Accept: "application/json",
			...(init?.body ? { "Content-Type": "application/json" } : {}),
			...init?.headers,
		},
		...init,
	});
	if (!res.ok) {
		const text = await res.text().catch(() => "");
		throw new Error(text || `HTTP ${res.status}`);
	}
	if (res.status === 204 || res.status === 202) {
		const text = await res.text().catch(() => "");
		if (!text) return undefined as T;
		try {
			return JSON.parse(text) as T;
		} catch {
			return undefined as T;
		}
	}
	return res.json() as Promise<T>;
}

export function getHostConfig(): Promise<HostConfigState> {
	return api("/api/v1/host/config");
}

export function setHostConfig(
	settings: HostConfigFile,
): Promise<HostConfigState> {
	return api("/api/v1/host/config", {
		method: "PUT",
		body: JSON.stringify(settings),
	});
}

export function setMoonlightBroadcast(
	enabled: boolean,
): Promise<HostConfigState> {
	return api("/api/v1/host/moonlight", {
		method: "PUT",
		body: JSON.stringify({ enabled }),
	});
}

export function getCompositors(): Promise<AvailableCompositor[]> {
	return api("/api/v1/compositors");
}

export function getCaptureMethods(): Promise<AvailableCaptureMethod[]> {
	return api("/api/v1/capture/methods");
}

export function getHeadlessCompositors(): Promise<
	AvailableHeadlessCompositor[]
> {
	return api("/api/v1/compositors/headless");
}

/**
 * Schedules a host process restart. Endpoint returns 202 with an empty body.
 * Live sessions drop; the caller must confirm before invoking.
 */
export async function restartHost(): Promise<void> {
	const response = await fetch("/api/v1/host/restart", {
		method: "POST",
		credentials: "same-origin",
		headers: { Accept: "application/json" },
	});
	if (response.status === 202) return;
	const body = (await response.json().catch(() => null)) as {
		error?: string;
	} | null;
	throw new Error(
		body?.error || `Host restart failed (HTTP ${response.status})`,
	);
}
