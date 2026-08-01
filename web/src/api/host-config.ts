/** Manual host-config client until orval regenerates from OpenAPI. */

export type HostConfigFile = {
	version: number;
	general: {
		host_name?: string | null;
		perf: boolean;
	};
	input: {
		gamepad?: string | null;
		gamescope_grab_cursor: boolean;
	};
	audio_video: {
		video_source?: string | null;
		capture_method?: string | null;
		compositor?: string | null;
		headless_compositor?: string | null;
		max_fps?: number | null;
		ten_bit: boolean;
		four_four_four: boolean;
		gamescope_hdr: boolean;
	};
	network: {
		chacha20: boolean;
		mdns: boolean;
		fec_pct?: number | null;
	};
	encoders: {
		encoder: string;
		render_adapter?: string | null;
		zerocopy?: boolean | null;
	};
};

export type HostConfigState = {
	settings: HostConfigFile;
	configured: boolean;
	requires_restart: boolean;
	env_path: string;
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
