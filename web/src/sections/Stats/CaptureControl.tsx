import { useQueryClient } from "@tanstack/react-query";
import { toast } from "@unom/ui/toast";
import { Circle, Square } from "lucide-react";
import type { FC } from "react";
import type { StatsStatus } from "@/api/gen/model/statsStatus";
import {
	getStatsCaptureStatusQueryKey,
	getStatsRecordingsListQueryKey,
	useStatsCaptureStart,
	useStatsCaptureStatus,
	useStatsCaptureStop,
} from "@/api/gen/stats/stats";
import { QueryState } from "@/components/query-state";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { apiErrorMessage } from "@/lib/errors";
import type { Loadable } from "@/lib/query";
import { m } from "@/paraglide/messages";
import { fmtDuration, kindLabel, Stat } from "./helpers";

/**
 * Container: arm/disarm the capture. Owns the polled status query plus start/stop; stopping also
 * refreshes the recordings list (owned by the Recordings subsection — invalidated here by key).
 */
export const CaptureControlSection: FC = () => {
	const qc = useQueryClient();
	const status = useStatsCaptureStatus({ query: { refetchInterval: 2_000 } });
	const start = useStatsCaptureStart();
	const stop = useStatsCaptureStop();

	const refreshStatus = () =>
		qc.invalidateQueries({ queryKey: getStatsCaptureStatusQueryKey() });
	// Both paths report failure. A failed STOP is the one that matters: it is "stop & save", so
	// swallowing the error let a capture the operator had been recording for minutes disappear with
	// no recording written and nothing on screen to say so.
	const onStart = () =>
		start.mutate(undefined, {
			onSuccess: refreshStatus,
			onError: (e) => toast.error(apiErrorMessage(e) ?? m.stats_start_failed()),
		});
	const onStop = () =>
		stop.mutate(undefined, {
			onSuccess: () => {
				refreshStatus();
				qc.invalidateQueries({ queryKey: getStatsRecordingsListQueryKey() });
			},
			onError: (e) => toast.error(apiErrorMessage(e) ?? m.stats_stop_failed()),
		});

	return (
		<CaptureControlCard
			status={status}
			onStart={onStart}
			onStop={onStop}
			isStarting={start.isPending}
			isStopping={stop.isPending}
		/>
	);
};

/** Start/Stop + a Recording/Idle pill with elapsed + sample count. */
export const CaptureControlCard: FC<{
	status: Loadable<StatsStatus>;
	onStart: () => void;
	onStop: () => void;
	isStarting: boolean;
	isStopping: boolean;
}> = ({ status, onStart, onStop, isStarting, isStopping }) => {
	const s = status.data;
	const armed = s?.armed ?? false;
	// Host-measured elapsed (monotonic) — not `Date.now() - started_unix_ms`, which mixes the
	// browser's clock with the host's and reads wrong (or clamps to 0:00) under any skew.
	const elapsed = armed && s ? s.elapsed_ms : 0;
	return (
		<QueryState
			isLoading={status.isLoading}
			error={status.error}
			refetch={status.refetch}
		>
			<Card>
				<CardHeader className="flex-row items-start justify-between gap-3 space-y-0">
					<div className="min-w-0 space-y-1">
						<CardTitle className="text-base tracking-tight">
							{m.stats_capture_title()}
						</CardTitle>
						<p className="text-sm text-muted-foreground">
							{m.stats_capture_desc()}
						</p>
					</div>
					{armed ? (
						<Badge variant="destructive" className="shrink-0 gap-1.5">
							<Circle className="size-2.5 animate-pulse fill-current" />
							{m.stats_recording()}
						</Badge>
					) : (
						<Badge variant="outline" className="shrink-0">
							{m.stats_idle()}
						</Badge>
					)}
				</CardHeader>
				<CardContent className="space-y-4">
					{armed && s && (
						<dl className="flex flex-wrap gap-x-6 gap-y-3 rounded-lg border border-border/70 bg-muted/30 px-3 py-3 sm:px-4">
							<Stat label={m.stats_elapsed()} value={fmtDuration(elapsed)} />
							<Stat label={m.stats_samples()} value={String(s.sample_count)} />
							{s.kind && (
								<Stat label={m.stats_kind()} value={kindLabel(s.kind)} />
							)}
						</dl>
					)}
					<div className="flex flex-wrap gap-2">
						{armed ? (
							<Button
								variant="destructive"
								disabled={isStopping}
								onClick={onStop}
							>
								<Square className="size-4" />
								{m.stats_stop()}
							</Button>
						) : (
							<Button disabled={isStarting} onClick={onStart}>
								<Circle className="size-4 fill-current" />
								{m.stats_start()}
							</Button>
						)}
					</div>
				</CardContent>
			</Card>
		</QueryState>
	);
};
