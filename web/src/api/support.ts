import { apiFetch } from "./fetcher";

export interface SupportBundle {
	schema: number;
	id: string;
	generated_unix_ms: number;
	host: {
		version: string;
		abi_version: number;
		os: string;
		os_name: string;
		gamestream: boolean;
	};
}

/** Create a redacted, local-only diagnostics bundle through the authenticated BFF. */
export function createSupportBundle(): Promise<SupportBundle> {
	return apiFetch<SupportBundle>("/api/v1/support-bundles", { method: "POST" });
}
