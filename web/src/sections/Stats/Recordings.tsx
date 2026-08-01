import { useQueryClient } from "@tanstack/react-query";
import { toast } from "@unom/ui/toast";
import { Download, Eye, Trash2 } from "lucide-react";
import type { FC } from "react";
import type { CaptureMeta } from "@/api/gen/model/captureMeta";
import {
	getStatsRecordingsListQueryKey,
	statsRecordingGet,
	useStatsRecordingDelete,
	useStatsRecordingsList,
} from "@/api/gen/stats/stats";
import { HelpTip, RecommendedMark } from "@/components/option-help";
import { QueryState } from "@/components/query-state";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import { apiErrorMessage } from "@/lib/errors";
import type { Loadable } from "@/lib/query";
import { m } from "@/paraglide/messages";
import { fmtDuration, fmtTimestamp, kindLabel } from "./helpers";

const RECORDINGS_HELP =
	"Saved captures from Stop & save. View opens graphs on this page; Download exports the full capture as JSON; Delete removes it from the host.";
const RECORDINGS_RECOMMENDED =
	"Keep a short capture of the problem window; delete old runs you no longer need.";
const VIEW_HELP =
	"Open latency, throughput, and health graphs for this recording. Click again to close.";
const DOWNLOAD_HELP =
	"Download the full capture as JSON for offline analysis or bug reports.";
const DELETE_HELP =
	"Permanently delete this recording from the host. This cannot be undone.";

/**
 * Container: the saved recordings. Owns the list query, delete, and the JSON export. Selection is
 * the parent's UI state (it also drives the detail card), passed through here for row highlight +
 * to clear it when the selected recording is deleted.
 */
export const RecordingsSection: FC<{
	selectedId: string | null;
	onSelect: (id: string | null) => void;
}> = ({ selectedId, onSelect }) => {
	const qc = useQueryClient();
	const recordings = useStatsRecordingsList();
	const del = useStatsRecordingDelete();

	const onDelete = (id: string) => {
		if (!confirm(m.stats_delete_confirm())) return;
		del.mutate(
			{ id },
			{
				onSuccess: () => {
					if (selectedId === id) onSelect(null);
					qc.invalidateQueries({ queryKey: getStatsRecordingsListQueryKey() });
				},
				onError: (e) =>
					toast.error(apiErrorMessage(e) ?? m.stats_delete_failed()),
			},
		);
	};

	// Export the full Capture JSON via a one-off GET → blob download.
	const onDownload = async (id: string) => {
		try {
			const cap = await statsRecordingGet(id);
			const blob = new Blob([JSON.stringify(cap, null, 2)], {
				type: "application/json",
			});
			const url = URL.createObjectURL(blob);
			const a = document.createElement("a");
			a.href = url;
			a.download = `${id}.json`;
			document.body.appendChild(a);
			a.click();
			a.remove();
			URL.revokeObjectURL(url);
		} catch (e) {
			// The old comment claimed the detail view surfaces this — it only does so for the SELECTED
			// recording, and Download is offered on every row. Downloading an unselected one that
			// failed produced a button that visibly did nothing.
			toast.error(apiErrorMessage(e) ?? m.stats_download_failed());
		}
	};

	return (
		<RecordingsCard
			recordings={recordings}
			selectedId={selectedId}
			onSelect={onSelect}
			onDownload={onDownload}
			onDelete={onDelete}
			isDeleting={del.isPending}
		/>
	);
};

/** Saved recordings, with View / Download / Delete row actions. */
export const RecordingsCard: FC<{
	recordings: Loadable<CaptureMeta[]>;
	selectedId: string | null;
	onSelect: (id: string | null) => void;
	onDownload: (id: string) => void;
	onDelete: (id: string) => void;
	isDeleting: boolean;
}> = ({
	recordings,
	selectedId,
	onSelect,
	onDownload,
	onDelete,
	isDeleting,
}) => {
	const rows = recordings.data ?? [];
	return (
		<Card>
			<CardHeader className="space-y-1">
				<div className="flex items-center gap-1.5">
					<h2 className="text-base font-semibold tracking-tight">
						{m.stats_recordings_title()}
					</h2>
					<HelpTip label={m.stats_recordings_title()} text={RECORDINGS_HELP} />
				</div>
				<RecommendedMark value={RECORDINGS_RECOMMENDED} />
			</CardHeader>
			<QueryState
				isLoading={recordings.isLoading}
				error={recordings.error}
				refetch={recordings.refetch}
			>
				{rows.length === 0 ? (
					<CardContent className="pt-0">
						<p className="rounded-lg border border-dashed border-border/80 bg-muted/20 px-4 py-10 text-center text-sm text-muted-foreground">
							{m.stats_recordings_empty()}
						</p>
					</CardContent>
				) : (
					<CardContent flush>
						<div className="overflow-x-auto">
							<Table>
								<TableHeader>
									<TableRow className="hover:bg-transparent">
										<TableHead>{m.stats_col_time()}</TableHead>
										<TableHead>{m.stats_col_kind()}</TableHead>
										<TableHead>{m.stats_col_resolution()}</TableHead>
										<TableHead>{m.stats_col_codec()}</TableHead>
										<TableHead className="text-right">
											{m.stats_col_duration()}
										</TableHead>
										<TableHead className="text-right">
											{m.stats_col_samples()}
										</TableHead>
										<TableHead className="w-32" />
									</TableRow>
								</TableHeader>
								<TableBody>
									{rows.map((r) => (
										<TableRow
											key={r.id}
											data-state={selectedId === r.id ? "selected" : undefined}
											className="data-[state=selected]:bg-primary/10"
										>
											<TableCell className="whitespace-nowrap text-sm font-medium">
												{fmtTimestamp(r.started_unix_ms)}
											</TableCell>
											<TableCell>
												<Badge
													variant={
														r.kind === "gamestream" ? "secondary" : "default"
													}
												>
													{kindLabel(r.kind)}
												</Badge>
											</TableCell>
											<TableCell className="tabular-nums text-muted-foreground">
												{r.width}×{r.height}@{r.fps}
											</TableCell>
											<TableCell className="uppercase text-muted-foreground">
												{r.codec}
											</TableCell>
											<TableCell className="text-right tabular-nums">
												{fmtDuration(r.duration_ms)}
											</TableCell>
											<TableCell className="text-right tabular-nums">
												{r.sample_count}
											</TableCell>
											<TableCell>
												<div className="flex justify-end gap-0.5">
													<Button
														variant="ghost"
														size="icon"
														aria-label={m.stats_view()}
														title={VIEW_HELP}
														onClick={() =>
															onSelect(selectedId === r.id ? null : r.id)
														}
													>
														<Eye className="size-4" />
													</Button>
													<Button
														variant="ghost"
														size="icon"
														aria-label={m.stats_download()}
														title={DOWNLOAD_HELP}
														onClick={() => onDownload(r.id)}
													>
														<Download className="size-4" />
													</Button>
													<Button
														variant="ghost"
														size="icon"
														aria-label={m.stats_delete()}
														title={DELETE_HELP}
														disabled={isDeleting}
														onClick={() => onDelete(r.id)}
													>
														<Trash2 className="size-4 text-destructive" />
													</Button>
												</div>
											</TableCell>
										</TableRow>
									))}
								</TableBody>
							</Table>
						</div>
					</CardContent>
				)}
			</QueryState>
		</Card>
	);
};
