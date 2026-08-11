import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@unom/ui/button";
import { toast } from "@unom/ui/toast";
import type { FC } from "react";
import {
	getListGpusQueryKey,
	useListGpus,
	useSetGpuPreference,
} from "@/api/gen/gpu/gpu";
import type { GpuState } from "@/api/gen/model";
import { HelpTip, RecommendedMark, SettingEffectBadge } from "@/components/option-help";
import { QueryState } from "@/components/query-state";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { apiErrorMessage } from "@/lib/errors";
import type { Loadable } from "@/lib/query";
import { m } from "@/paraglide/messages";

/**
 * Container: the host's GPU inventory + selection. Polls (a stream starting/stopping moves the
 * "In use" badge; an eGPU can appear) and applies auto/preferred choices via the mgmt API. A
 * preference applies to the NEXT session — the help text says so.
 */
export const GpuSection: FC = () => {
	const qc = useQueryClient();
	// GPU state only moves when a session starts or ends, which the event stream reports — so this
	// is a slow safety net rather than a 5 s poll of a device enumeration.
	const gpus = useListGpus({ query: { refetchInterval: 20_000 } });
	const setPref = useSetGpuPreference();

	// A refused GPU preference used to vanish: nothing read `setPref.error`, so the card simply
	// stayed on the old selection as though the click had missed.
	const apply = (mode: "auto" | "manual", gpuId?: string) =>
		setPref.mutate(
			{ data: { mode, gpu_id: gpuId ?? null } },
			{
				onSuccess: () =>
					qc.invalidateQueries({ queryKey: getListGpusQueryKey() }),
				onError: (e) => toast.error(apiErrorMessage(e) ?? m.gpu_apply_failed()),
			},
		);

	return <GpuCard state={gpus} onApply={apply} busy={setPref.isPending} />;
};

const fmtVram = (mb: number) =>
	mb >= 1024 ? `${Math.round(mb / 1024)} GiB` : `${mb} MiB`;

/**
 * The vendor an explicit `SLIPSTREAM_ENCODER` pin can open on (display name) — the console mirror
 * of the host's backend→vendor table. Vendor-agnostic pins (software) and unknown/multi-vendor
 * spellings (vaapi, vulkan, pyrowave) map to nothing: no conflict to warn about.
 */
const encoderPinVendor: Record<string, string> = {
	nvenc: "NVIDIA",
	nvidia: "NVIDIA",
	cuda: "NVIDIA",
	hw: "NVIDIA",
	amf: "AMD",
	amd: "AMD",
	qsv: "Intel",
	intel: "Intel",
};

/**
 * The host.env encoder pin, surfaced so a conflicting GPU choice doesn't just look broken: amber
 * when the pin's vendor contradicts the next session's GPU (the host overrides the pin at session
 * open — the stale pin should be removed), a muted note otherwise.
 */
const EncoderPinNote: FC<{ state: GpuState; pin: string }> = ({
	state,
	pin,
}) => {
	const vendor = encoderPinVendor[pin];
	const conflicting =
		vendor && state.selected && state.selected.vendor !== vendor.toLowerCase();
	return conflicting && state.selected ? (
		<p className="rounded-md border border-warning/40 bg-warning/10 px-3 py-2 text-sm text-[var(--warning)]">
			{m.gpu_encoder_pin_warning({
				value: pin,
				vendor,
				name: state.selected.name,
			})}
		</p>
	) : (
		<p className="text-xs text-muted-foreground">
			{m.gpu_encoder_pin_note({ value: pin })}
		</p>
	);
};

/**
 * GPU list in the compositors-card style: per-GPU badges for the manual pick ("Preferred"), what
 * the next session will use ("Next session"), and what live sessions encode on right now
 * ("In use · NVENC"), plus an Automatic/Prefer control pair.
 */
export const GpuCard: FC<{
	state: Loadable<GpuState>;
	onApply: (mode: "auto" | "manual", gpuId?: string) => void;
	busy: boolean;
}> = ({ state, onApply, busy }) => {
	const s = state.data;
	return (
		<Card>
			<CardHeader className="flex flex-col gap-3 space-y-0 sm:flex-row sm:items-start sm:justify-between">
				<div className="space-y-1.5">
					<div className="flex items-center gap-1.5">
						<CardTitle className="tracking-tight">{m.host_gpus()}</CardTitle>
						<HelpTip
							label={m.host_gpus()}
							text="Chooses which GPU capture and encode use. Changes apply to the next session, not one that is already streaming."
						/>
						<SettingEffectBadge effect="next-session" />
					</div>
					<p className="max-w-prose text-sm leading-relaxed text-muted-foreground">
						{m.host_gpus_help()}
					</p>
					<RecommendedMark value="Automatic, unless this PC has multiple GPUs and the wrong one is selected" />
				</div>
				{s && (s.gpus?.length ?? 0) > 0 && (
					<div className="flex shrink-0 items-center gap-1.5 self-start">
						<Button
							size="sm"
							variant={s.mode === "auto" ? "default" : "outline"}
							disabled={busy || s.mode === "auto"}
							onClick={() => onApply("auto")}
							title="Let Slipstream pick the best GPU for capture and encode on each new session"
						>
							{m.gpu_automatic()}
						</Button>
						<HelpTip
							label={m.gpu_automatic()}
							text="Clears any pinned GPU and lets the host pick the best adapter for the next session. Best default for single-GPU PCs."
						/>
					</div>
				)}
			</CardHeader>
			<CardContent className="space-y-4">
				<QueryState
					isLoading={state.isLoading}
					error={state.error}
					refetch={state.refetch}
				>
					{s &&
						((s.gpus?.length ?? 0) === 0 ? (
							<p className="rounded-lg border border-dashed border-border/70 bg-muted/20 px-4 py-8 text-center text-sm text-muted-foreground">
								{m.gpu_none()}
							</p>
						) : (
							<ul className="overflow-hidden rounded-lg border border-border/70 bg-muted/15 divide-y divide-border/60">
								{(s.gpus ?? []).map((g) => {
									const isActive = s.active?.id === g.id;
									const isSelected = s.selected?.id === g.id;
									const isPreferred =
										s.mode === "manual" && s.preferred_id === g.id;
									return (
										<li
											key={g.id}
											className="flex flex-col gap-3 px-3 py-2.5 sm:flex-row sm:items-center sm:justify-between sm:px-4"
										>
											<div className="min-w-0">
												<div className="flex flex-wrap items-center gap-2">
													<span className="font-medium">{g.name}</span>
													{isPreferred && (
														<Badge variant="secondary">
															{m.gpu_preferred()}
														</Badge>
													)}
													{isActive && s.active ? (
														<Badge variant="success">
															{m.gpu_in_use({
																backend: s.active.backend.toUpperCase(),
															})}
														</Badge>
													) : (
														isSelected && (
															<Badge variant="default">
																{m.gpu_next_session()}
															</Badge>
														)
													)}
												</div>
												<code className="text-xs text-muted-foreground">
													{g.vendor}
													{g.vram_mb > 0 ? ` · ${fmtVram(g.vram_mb)}` : ""}
													{` · ${g.id}`}
												</code>
											</div>
											<div className="flex shrink-0 items-center gap-1.5 self-start sm:self-center">
												<Button
													size="sm"
													variant="outline"
													disabled={busy || isPreferred}
													onClick={() => onApply("manual", g.id)}
													title={`Pin capture and encode to ${g.name} for the next session`}
												>
													{m.gpu_prefer()}
												</Button>
												<HelpTip
													label={m.gpu_prefer()}
													text={`Pins capture and encode to ${g.name}. Use this when Automatic lands on the wrong GPU (eGPU, iGPU vs discrete). Applies next session.`}
												/>
											</div>
										</li>
									);
								})}
							</ul>
						))}
					{s?.selected?.source === "preference_missing" && (
						<p className="rounded-md border border-warning/40 bg-warning/10 px-3 py-2 text-sm text-[var(--warning)]">
							{m.gpu_missing_warning({ name: s.preferred_name ?? "?" })}
						</p>
					)}
					{s?.env_override && s.mode === "auto" && (
						<p className="text-xs text-muted-foreground">
							{m.gpu_env_note({ value: s.env_override })}
						</p>
					)}
					{s?.encoder_pin && <EncoderPinNote state={s} pin={s.encoder_pin} />}
				</QueryState>
			</CardContent>
		</Card>
	);
};
