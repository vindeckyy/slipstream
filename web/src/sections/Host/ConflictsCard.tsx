import { AlertTriangle } from "lucide-react";
import type { FC } from "react";
import { useGetLocalSummary } from "@/api/gen/host/host";
import { HelpTip, RecommendedMark } from "@/components/option-help";
import { Card, CardContent } from "@/components/ui/card";
import { m } from "@/paraglide/messages";

/**
 * "Something else is already listening on these ports."
 *
 * The host detects other Moonlight-compatible servers (Sunshine, Apollo, …) whose process is
 * currently running and reports them in `GET /local/summary` as `conflicts`. Nothing surfaced it,
 * even though it is a common reason a slipstream host looks installed and working but no client can
 * reach it: two servers fight over the same ports, with whichever won the bind answering the client.
 *
 * Renders nothing at all when there is no conflict, so a healthy host sees no extra chrome.
 */
export const ConflictsCard: FC = () => {
	// The process can start or stop while the host stays up, so refresh the live conflict summary.
	const summary = useGetLocalSummary({
		query: { refetchInterval: 10_000, staleTime: 5_000 },
	});
	const conflicts = summary.data?.conflicts ?? [];
	if (conflicts.length === 0) return null;
	return (
		<Card className="border-warning/40 bg-warning/5 ring-warning/30">
			<CardContent className="flex items-start gap-3 p-card pt-card sm:pt-card">
				<AlertTriangle className="mt-0.5 size-5 shrink-0 text-[var(--warning)]" />
				<div className="min-w-0 flex-1 space-y-2">
					<div className="flex items-center gap-1.5">
						<p className="text-sm font-medium text-[var(--warning)]">
							{m.host_conflicts_title()}
						</p>
						<HelpTip
							label={m.host_conflicts_title()}
							text="Another Moonlight-compatible host is already bound to the same ports. Clients reach whichever process won the bind, so Slipstream can look fine while nothing connects."
						/>
					</div>
					<p className="max-w-prose text-sm leading-relaxed text-muted-foreground">
						{m.host_conflicts_help()}
					</p>
					<RecommendedMark value="Stop or uninstall the other server, then restart Slipstream" />
					<ul className="flex flex-col gap-1.5">
						{conflicts.map((c) => (
							<li
								key={c}
								className="rounded-md border border-border/60 bg-muted/40 px-3 py-1.5 font-mono text-xs text-muted-foreground"
								title="Detected process still listening on shared game-streaming ports"
							>
								{c}
							</li>
						))}
					</ul>
				</div>
			</CardContent>
		</Card>
	);
};
