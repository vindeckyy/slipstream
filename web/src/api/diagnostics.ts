import { apiFetch } from "./fetcher";

export type PreflightStatus = "pass" | "warn" | "fail" | "skip";

export interface PreflightCheck {
	id: string;
	label: string;
	status: PreflightStatus;
	detail: string;
	remediation?: string;
}

export interface PreflightReport {
	schema: number;
	generated_unix_ms: number;
	ready: boolean;
	checks: PreflightCheck[];
}

export function getPreflight(): Promise<PreflightReport> {
	return apiFetch<PreflightReport>("/api/v1/diagnostics/preflight");
}
