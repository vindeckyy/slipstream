import { Link } from "@tanstack/react-router";
import Section from "@unom/ui/section";
import {
	ArrowLeft,
	MonitorPlay,
	RefreshCw,
	Video,
	Volume2,
	ZapOff,
} from "lucide-react";
import type { FC } from "react";
import type { ActiveGame } from "@/api/gen/model/activeGame";
import type { GameEntry } from "@/api/gen/model/gameEntry";
import type { RuntimeStatus } from "@/api/gen/model/runtimeStatus";
import { PageHeader, StatusIndicator } from "@/components/observatory";
import { HelpTip } from "@/components/option-help";
import { QueryState } from "@/components/query-state";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { fmtNumber } from "@/lib/format";
import type { Loadable } from "@/lib/query";
import { m } from "@/paraglide/messages";
import { RunningGames } from "@/sections/Dashboard/RunningGames";

export type SessionsViewProps = {
	status: Loadable<RuntimeStatus>;
	library?: GameEntry[];
	onStopSession: () => void;
	onRequestIdr: () => void;
	onEndGame: (game: ActiveGame) => void;
	isStopping: boolean;
	isRequestingIdr: boolean;
	isEndingGame: boolean;
};

export const SessionsView: FC<SessionsViewProps> = ({
	status,
	library,
	onStopSession,
	onRequestIdr,
	onEndGame,
	isStopping,
	isRequestingIdr,
	isEndingGame,
}) => {
	const s = status.data;
	return (
		<Section maxWidth={false}>
			<div className="flex flex-col gap-card">
				<PageHeader
					title="Sessions"
					meta={
						s
							? s.active_sessions > 0
								? m.status_sessions_active({ count: s.active_sessions })
								: m.status_no_session()
							: undefined
					}
					actions={
						<Link
							to="/"
							title="Back to Live status overview on the Dashboard."
							className="inline-flex min-h-9 items-center justify-center gap-2 rounded-md border border-border bg-background/70 px-3 py-2 text-sm font-medium shadow-none transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
						>
							<ArrowLeft className="size-3.5" />
							{m.status_title()}
						</Link>
					}
				/>

				<QueryState
					isLoading={status.isLoading}
					error={status.error}
					refetch={status.refetch}
				>
					{s && (
						<div className="flex flex-col gap-card">
							<Card>
								<CardHeader className="pb-3 sm:pb-3">
									<CardTitle className="flex flex-wrap items-center gap-2">
										<MonitorPlay className="size-4 shrink-0 text-muted-foreground" />
										{m.status_session()}
										<Badge
											variant={s.active_sessions > 0 ? "success" : "outline"}
										>
											{s.active_sessions > 0
												? m.status_sessions_active({
														count: s.active_sessions,
													})
												: m.status_no_session()}
										</Badge>
										<HelpTip
											label={m.status_session()}
											text="Live stream session for connected clients. Use the actions below to request a keyframe or stop every active session."
										/>
									</CardTitle>
								</CardHeader>
								<CardContent flush className="flex flex-col">
									<div className="flex flex-wrap items-center gap-2 border-y border-border bg-muted/40 px-4 py-3 sm:px-6">
										<Button
											variant="outline"
											size="sm"
											disabled={!s.video_streaming || isRequestingIdr}
											title="Ask the encoder for a fresh keyframe (IDR). Use when the picture is corrupt or stuck after a network blip. Needs an active video stream."
											onClick={onRequestIdr}
										>
											<RefreshCw className="size-3.5" />
											{m.action_request_idr()}
										</Button>
										<HelpTip
											label={m.action_request_idr()}
											text="Forces an IDR / keyframe so the client can resync the bitstream. Disabled when video is idle."
										/>
										<Button
											variant={s.session ? "destructive" : "secondary"}
											size="sm"
											disabled={!s.session || isStopping}
											title="End the live streaming session. If several clients are connected, one stop ends all of them."
											onClick={onStopSession}
										>
											<ZapOff className="size-3.5" />
											{m.action_stop_session()}
										</Button>
										<HelpTip
											label={m.action_stop_session()}
											text="Stops the host session. With multiple active sessions the console confirms first, because the host has one stop for all of them."
										/>
									</div>
									<div className="grid gap-4 border-b border-border/60 px-4 py-3 sm:grid-cols-2 sm:px-6">
										<StatusIndicator
											icon={<Video className="size-3.5" />}
											label={m.status_video()}
											status={s.video_streaming ? "healthy" : "unknown"}
											statusLabel={
												s.video_streaming
													? m.status_streaming()
													: m.status_idle()
											}
										/>
										<StatusIndicator
											icon={<Volume2 className="size-3.5" />}
											label={m.status_audio()}
											status={s.audio_streaming ? "healthy" : "unknown"}
											statusLabel={
												s.audio_streaming
													? m.status_streaming()
													: m.status_idle()
											}
										/>
									</div>
									<div className="px-4 py-4 sm:px-6 sm:py-5">
										{s.stream ? (
											<dl className="grid grid-cols-2 gap-x-6 gap-y-4 sm:grid-cols-4">
												<Field
													label={m.stream_codec()}
													value={s.stream.codec.toUpperCase()}
												/>
												<Field
													label={m.stream_resolution()}
													value={`${s.stream.width}×${s.stream.height}`}
												/>
												<Field
													label={m.stream_fps()}
													value={`${s.stream.fps} fps`}
												/>
												<Field
													label={m.stream_bitrate()}
													value={`${fmtNumber(s.stream.bitrate_kbps / 1000, 1)} Mbps`}
												/>
												{s.stream.time_to_first_frame_ms != null && (
													<Field
														label={m.stream_first_frame()}
														value={`${fmtNumber(s.stream.time_to_first_frame_ms)} ms`}
													/>
												)}
												{s.stream.last_resize_ms != null && (
													<Field
														label={m.stream_last_resize()}
														value={`${fmtNumber(s.stream.last_resize_ms)} ms`}
													/>
												)}
												<Field
													label={m.stream_packet_size()}
													value={`${fmtNumber(s.stream.packet_size)} B`}
												/>
												<Field
													label={m.stream_min_fec()}
													value={fmtNumber(s.stream.min_fec)}
												/>
											</dl>
										) : (
											<p className="text-sm text-muted-foreground">
												{m.status_no_session()}
											</p>
										)}
									</div>
								</CardContent>
							</Card>

							<RunningGames
								games={s.games ?? []}
								library={library}
								onEnd={onEndGame}
								isEnding={isEndingGame}
							/>
						</div>
					)}
				</QueryState>
			</div>
		</Section>
	);
};

const Field: FC<{ label: string; value: string }> = ({ label, value }) => (
	<div className="min-w-0">
		<dt className="text-xs font-medium text-muted-foreground">{label}</dt>
		<dd className="mt-1 text-sm font-semibold tabular-nums tracking-tight">
			{value}
		</dd>
	</div>
);
