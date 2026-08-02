import type { FC } from "react";
import type { StatsSample } from "@/api/gen/model/statsSample";
import { HelpTip } from "@/components/option-help";
import { Badge } from "@/components/ui/badge";
import { Stat } from "./helpers";

function lastDefined<T>(samples: StatsSample[], read: (sample: StatsSample) => T | undefined) {
	for (let i = samples.length - 1; i >= 0; i -= 1) {
		const sample = samples[i];
		if (!sample) continue;
		const value = read(sample);
		if (value !== undefined) return value;
	}
	return undefined;
}

function formatAge(ageUs: number | undefined): string {
	if (ageUs === undefined) return "-";
	if (ageUs < 1_000) return `${Math.round(ageUs)} us`;
	return `${(ageUs / 1_000).toFixed(1)} ms`;
}

function formatCount(value: number | undefined): string {
	return value === undefined ? "-" : value.toLocaleString();
}

function formatModifier(value: number | undefined): string {
	if (value === undefined) return "-";
	if (value === 0) return "linear / unknown";
	return `0x${Math.trunc(value).toString(16)}`;
}

export const CaptureDiagnostics: FC<{ samples: StatsSample[] }> = ({ samples }) => {
	const hasTelemetry = samples.some(
		(sample) =>
			sample.capture_backend !== undefined ||
			sample.capture_age_us !== undefined ||
			sample.capture_frames_published !== undefined ||
			sample.capture_width !== undefined,
	);
	if (!hasTelemetry) return null;

	const latestAge = lastDefined(samples, (sample) => sample.capture_age_us);
	const peakAge = samples.reduce(
		(max, sample) =>
			sample.capture_age_us === undefined
				? max
				: Math.max(max, sample.capture_age_us),
		0,
	);
	const backend = lastDefined(samples, (sample) => sample.capture_backend);
	const width = lastDefined(samples, (sample) =>
		sample.capture_width && sample.capture_width > 0 ? sample.capture_width : undefined,
	);
	const height = lastDefined(samples, (sample) =>
		sample.capture_height && sample.capture_height > 0 ? sample.capture_height : undefined,
	);
	const ageOverLimit = samples.some((sample) => sample.capture_age_over_limit === true);
	const published = lastDefined(samples, (sample) => sample.capture_frames_published);
	const overwritten = lastDefined(samples, (sample) => sample.capture_frames_overwritten);
	const drained = lastDefined(samples, (sample) => sample.capture_buffers_drained);
	const modifier = lastDefined(samples, (sample) => sample.capture_modifier);
	const hasSampledAge = samples.some((sample) => sample.capture_age_us !== undefined);

	return (
		<div className="space-y-3 rounded-lg border border-border/70 bg-muted/15 p-3 sm:p-4">
			<div className="flex flex-wrap items-center justify-between gap-2">
				<div className="flex items-center gap-1.5">
					<h3 className="text-sm font-medium tracking-tight">Capture diagnostics</h3>
					<HelpTip
						label="Capture diagnostics"
						text="Source-side measurements recorded at the statistics boundary. Capture age ends before encoding, network transfer, decode, and display."
					/>
				</div>
				<Badge variant={ageOverLimit ? "destructive" : "secondary"}>
					{ageOverLimit ? "Age over threshold" : "Age within threshold"}
				</Badge>
			</div>
			<dl className="grid grid-cols-2 gap-x-4 gap-y-3 sm:grid-cols-3 lg:grid-cols-4">
				<Stat label="Backend" value={backend || "-"} />
				<Stat label="Newest frame age" value={formatAge(latestAge)} />
				<Stat label="Peak sampled age" value={formatAge(hasSampledAge ? peakAge : undefined)} />
				<Stat
					label="Source size"
					value={width !== undefined && height !== undefined ? `${width}x${height}` : "-"}
				/>
				<Stat label="Frames published" value={formatCount(published)} />
				<Stat label="Frames overwritten" value={formatCount(overwritten)} />
				<Stat label="Buffers drained" value={formatCount(drained)} />
				<Stat label="Modifier" value={formatModifier(modifier)} />
			</dl>
		</div>
	);
};
