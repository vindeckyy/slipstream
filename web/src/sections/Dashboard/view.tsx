import { Link } from "@tanstack/react-router";
import Section from "@unom/ui/section";
import {
	ArrowRight,
	KeyRound,
	MonitorPlay,
	Users,
	Video,
	Volume2,
} from "lucide-react";
import type { FC } from "react";
import type { ActivityEntry } from "@/api/events";
import type { ActiveGame } from "@/api/gen/model/activeGame";
import type { GameEntry } from "@/api/gen/model/gameEntry";
import type { RuntimeStatus } from "@/api/gen/model/runtimeStatus";
import { MetricCard, PageHeader } from "@/components/observatory";
import { HelpTip } from "@/components/option-help";
import { QueryState } from "@/components/query-state";
import { Badge } from "@/components/ui/badge";
import {
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { fmtNumber } from "@/lib/format";
import type { Loadable } from "@/lib/query";
import { m } from "@/paraglide/messages";
import { ActivityCard } from "./Activity";
import { RunningGames } from "./RunningGames";

export const DashboardView: FC<{
	status: Loadable<RuntimeStatus>;
	library?: GameEntry[];
	onStopSession: () => void;
	onRequestIdr: () => void;
	onEndGame: (game: ActiveGame) => void;
	isStopping: boolean;
	isRequestingIdr: boolean;
	isEndingGame: boolean;
	/** Optional event data keeps the view easy to compose with deterministic stories. */
	activity?: ActivityEntry[];
}> = ({ status, library, onEndGame, isEndingGame, activity }) => {
	const s = status.data;
	return (
		<Section maxWidth={false}>
			<div className="flex flex-col gap-card">
				<PageHeader
					title={m.status_title()}
					actions={
						<div className="flex items-center gap-1">
							<Link
								to="/sessions"
								title="Open Sessions for keyframe requests, stop session, and full stream details."
								className="inline-flex min-h-9 items-center justify-center gap-2 rounded-md border border-border bg-background/70 px-3 py-2 text-sm font-medium shadow-none transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
							>
								<MonitorPlay className="size-4" />
								{m.status_session()}
								<ArrowRight className="size-3.5" />
							</Link>
							<HelpTip
								label={m.status_session()}
								text="Sessions has the live stream actions (request keyframe, stop session) and a fuller stream readout than this overview."
							/>
						</div>
					}
				/>

				<QueryState
					isLoading={status.isLoading}
					error={status.error}
					refetch={status.refetch}
				>
					{s && (
						<div className="flex flex-col gap-card">
							<div className="grid gap-card xl:grid-cols-[minmax(0,1.15fr)_minmax(21rem,0.85fr)]">
								<OverviewMetrics status={s} />
								<SessionSummaryCard status={s} />
							</div>
							<RunningGames
								games={s.games ?? []}
								library={library}
								onEnd={onEndGame}
								isEnding={isEndingGame}
							/>
							<ActivityCard entries={activity} limit={8} />
						</div>
					)}
				</QueryState>
			</div>
		</Section>
	);
};

const OverviewMetrics: FC<{ status: RuntimeStatus }> = ({ status }) => {
	const paired = status.paired_clients + (status.native_paired_clients ?? 0);
	return (
		<div className="grid content-start gap-card sm:grid-cols-2">
			<MetricCard
				title={m.status_session()}
				value={fmtNumber(status.active_sessions)}
				description={
					status.active_sessions > 0
						? m.status_sessions_active({ count: status.active_sessions })
						: m.status_no_session()
				}
				icon={<MonitorPlay className="size-4" />}
			/>
			<MetricCard
				title={m.status_paired_count()}
				value={fmtNumber(paired)}
				icon={<Users className="size-4" />}
			/>
			<MetricCard
				title={m.status_video()}
				value={status.video_streaming ? m.status_streaming() : m.status_idle()}
				icon={<Video className="size-4" />}
			/>
			<MetricCard
				title={m.status_audio()}
				value={status.audio_streaming ? m.status_streaming() : m.status_idle()}
				icon={<Volume2 className="size-4" />}
			/>
			<MetricCard
				title={m.status_pin_pending()}
				value={
					status.pin_pending ? m.status_pin_waiting() : m.status_pin_none()
				}
				icon={<KeyRound className="size-4" />}
			/>
		</div>
	);
};

const SessionSummaryCard: FC<{ status: RuntimeStatus }> = ({ status }) => (
	<Card className="flex flex-col">
		<CardHeader className="pb-3 sm:pb-3">
			<CardTitle className="flex flex-wrap items-center gap-2">
				<MonitorPlay className="size-4 shrink-0 text-muted-foreground" />
				{m.status_session()}
				<Badge variant={status.active_sessions > 0 ? "success" : "outline"}>
					{status.active_sessions > 0
						? m.status_sessions_active({ count: status.active_sessions })
						: m.status_no_session()}
				</Badge>
			</CardTitle>
		</CardHeader>
		<CardContent className="flex-1">
			{status.stream ? (
				<dl className="grid grid-cols-2 gap-x-6 gap-y-4 sm:grid-cols-4 xl:grid-cols-2">
					<Field
						label={m.stream_codec()}
						value={status.stream.codec.toUpperCase()}
					/>
					<Field
						label={m.stream_resolution()}
						value={`${status.stream.width}×${status.stream.height}`}
					/>
					<Field label={m.stream_fps()} value={`${status.stream.fps} fps`} />
					<Field
						label={m.stream_bitrate()}
						value={`${fmtNumber(status.stream.bitrate_kbps / 1000, 1)} Mbps`}
					/>
				</dl>
			) : (
				<p className="text-sm text-muted-foreground">{m.status_no_session()}</p>
			)}
		</CardContent>
		<CardFooter className="border-t border-border/60 pt-4">
			<div className="flex items-center gap-1">
				<Link
					to="/sessions"
					title="Open Sessions for keyframe requests, stop session, and full stream details."
					className="inline-flex items-center gap-2 text-sm font-medium text-primary underline-offset-4 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
				>
					{m.status_session()}
					<ArrowRight className="size-3.5" />
				</Link>
				<HelpTip
					label={m.status_session()}
					text="Jump to the Sessions page to act on the live stream (keyframe, stop) or read the full stream fields."
				/>
			</div>
		</CardFooter>
	</Card>
);

const Field: FC<{ label: string; value: string }> = ({ label, value }) => (
	<div className="min-w-0">
		<dt className="text-xs font-medium text-muted-foreground">{label}</dt>
		<dd className="mt-1 text-sm font-semibold tabular-nums tracking-tight">
			{value}
		</dd>
	</div>
);
