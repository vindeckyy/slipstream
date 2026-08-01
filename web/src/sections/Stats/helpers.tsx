import type { FC, ReactNode } from "react";
import { HelpTip, RecommendedMark } from "@/components/option-help";
import { fmtDateTime } from "@/lib/format";
import { m } from "@/paraglide/messages";

/** ms → `m:ss`. */
export function fmtDuration(ms: number): string {
	const s = Math.max(0, Math.floor(ms / 1000));
	return `${Math.floor(s / 60)}:${(s % 60).toString().padStart(2, "0")}`;
}

/** Locale-aware (see lib/format.ts) — a bare `toLocaleString` follows the BROWSER, not the app. */
export function fmtTimestamp(unixMs: number): string {
	return fmtDateTime(unixMs);
}

export function kindLabel(kind: string): string {
	if (kind === "gamestream") return m.stats_kind_gamestream();
	if (kind === "native") return m.stats_kind_native();
	return kind;
}

export const Stat: FC<{ label: string; value: string }> = ({
	label,
	value,
}) => (
	<div className="flex min-w-[5.5rem] flex-col gap-0.5">
		<dt className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
			{label}
		</dt>
		<dd className="text-sm font-semibold tabular-nums tracking-tight">
			{value}
		</dd>
	</div>
);

/** Shared hover copy for chart blocks (live + detail). */
export const LATENCY_CHART_HELP =
	"Stacked per-stage pipeline time in microseconds. Tall bands show where frames spend time.";
export const THROUGHPUT_CHART_HELP =
	"New frames vs repeated frames (fps), plus measured goodput against the configured bitrate target.";
export const HEALTH_CHART_HELP =
	"Drops and FEC recoveries per sample window. On GameStream, only frame drops are instrumented.";

export const ChartBlock: FC<{
	title: string;
	desc?: string;
	help?: string;
	recommended?: ReactNode;
	children: ReactNode;
}> = ({ title, desc, help, recommended, children }) => (
	<div className="space-y-3">
		<div className="space-y-1">
			<div className="flex items-center gap-1.5">
				<h3 className="text-sm font-medium tracking-tight">{title}</h3>
				{help ? <HelpTip label={title} text={help} /> : null}
			</div>
			{desc ? (
				<p className="text-xs text-muted-foreground">{desc}</p>
			) : null}
			{recommended ? <RecommendedMark value={recommended} /> : null}
		</div>
		<div className="overflow-hidden rounded-lg border border-border/70 bg-muted/25 p-3 sm:p-4">
			{children}
		</div>
	</div>
);
