import { X } from "lucide-react";
import type { FC } from "react";
import type { Capture } from "@/api/gen/model/capture";
import { useStatsRecordingGet } from "@/api/gen/stats/stats";
import { QueryState } from "@/components/query-state";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { Loadable } from "@/lib/query";
import { m } from "@/paraglide/messages";
import { HealthChart, LatencyChart, ThroughputChart } from "./charts";
import { ChartBlock } from "./helpers";

/** Container: the full graph set for the selected recording — fetched by id. */
export const DetailSection: FC<{ id: string; onClose: () => void }> = ({
	id,
	onClose,
}) => {
	const detail = useStatsRecordingGet(id, { query: { enabled: !!id } });
	return <DetailCard detail={detail} onClose={onClose} />;
};

/** Full graph set for one selected recording: latency (p99 toggle) + throughput + health. */
export const DetailCard: FC<{
	detail: Loadable<Capture>;
	onClose: () => void;
}> = ({ detail, onClose }) => {
	const cap = detail.data;
	const samples = cap?.samples ?? [];
	return (
		<Card>
			<CardHeader className="flex-row items-start justify-between gap-3 space-y-0">
				<div className="min-w-0 space-y-1">
					<CardTitle className="text-base tracking-tight">
						{m.stats_detail_title()}
					</CardTitle>
					{cap && (
						// Encoder + GPU ride along with the mode: the stage split below can't be
						// read without knowing which backend produced it (a 10 ms `submit` means
						// very different things on NVENC and on Vulkan). Older recordings predate
						// the fields and simply omit them.
						<p className="truncate text-xs text-muted-foreground sm:text-sm">
							{cap.meta.width}×{cap.meta.height}@{cap.meta.fps} ·{" "}
							{cap.meta.codec.toUpperCase()}
							{cap.meta.encoder_backend && ` · ${cap.meta.encoder_backend}`}
							{cap.meta.gpu && ` · ${cap.meta.gpu}`}
						</p>
					)}
				</div>
				<Button
					variant="ghost"
					size="icon"
					className="shrink-0"
					aria-label={m.stats_close()}
					onClick={onClose}
				>
					<X className="size-4" />
				</Button>
			</CardHeader>
			<CardContent>
				<QueryState
					isLoading={detail.isLoading}
					error={detail.error}
					refetch={detail.refetch}
				>
					{samples.length === 0 ? (
						<p className="rounded-lg border border-dashed border-border/80 bg-muted/20 px-4 py-10 text-center text-sm text-muted-foreground">
							{m.stats_no_samples()}
						</p>
					) : (
						<div className="space-y-6">
							<ChartBlock
								title={m.stats_latency_title()}
								desc={m.stats_latency_desc()}
							>
								<LatencyChart samples={samples} toggle />
							</ChartBlock>
							<ChartBlock title={m.stats_throughput_title()}>
								<ThroughputChart samples={samples} />
							</ChartBlock>
							<ChartBlock title={m.stats_health_title()}>
								<HealthChart samples={samples} kind={cap?.meta.kind} />
							</ChartBlock>
						</div>
					)}
				</QueryState>
			</CardContent>
		</Card>
	);
};
