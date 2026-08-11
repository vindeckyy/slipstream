/**
 * Host configuration API client that has no generated equivalent.
 *
 * The generated `web/src/api/gen/host/host.ts` now owns `getHostConfig`,
 * `setHostConfig`, `setMoonlightBroadcast`, and `restartHost` plus their
 * query/mutation hooks and models. This module keeps only the capability
 * probes and their shared React Query keys.
 */
import type {
	AvailableCaptureMethod,
	AvailableCompositor,
	AvailableHeadlessCompositor,
} from "@/api/gen/model";

export const hostConfigQueryKey = ["host-config"] as const;
export const compositorsQueryKey = ["compositors"] as const;
export const captureMethodsQueryKey = ["capture-methods"] as const;
export const headlessCompositorsQueryKey = ["headless-compositors"] as const;

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
