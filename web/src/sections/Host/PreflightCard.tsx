import { useQuery } from "@tanstack/react-query";
import {
	AlertTriangle,
	CheckCircle2,
	CircleDashed,
	RefreshCw,
	ShieldCheck,
	XCircle,
} from "lucide-react";
import type { FC } from "react";
import { getPreflight, type PreflightStatus } from "@/api/diagnostics";
import { HelpTip } from "@/components/option-help";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { QueryState } from "@/components/query-state";
import { cn } from "@/lib/utils";

const statusLabel: Record<PreflightStatus, string> = {
	pass: "Ready",
	warn: "Warning",
	fail: "Blocked",
	skip: "Skipped",
};

const statusClasses: Record<PreflightStatus, string> = {
	pass: "text-[var(--success)]",
	warn: "text-[var(--warning)]",
	fail: "text-destructive",
	skip: "text-muted-foreground",
};

function StatusIcon({ status }: { status: PreflightStatus }) {
	if (status === "pass") return <CheckCircle2 className="size-4" aria-hidden />;
	if (status === "warn") return <AlertTriangle className="size-4" aria-hidden />;
	if (status === "fail") return <XCircle className="size-4" aria-hidden />;
	return <CircleDashed className="size-4" aria-hidden />;
}

export const PreflightCard: FC = () => {
	const query = useQuery({
		queryKey: ["diagnostics", "preflight"],
		queryFn: getPreflight,
		refetchInterval: 30_000,
		staleTime: 10_000,
	});
	const report = query.data;

	return (
		<Card className="overflow-hidden">
			<CardHeader className="border-b border-border/60 bg-muted/15">
				<div className="flex items-start justify-between gap-3">
					<div className="space-y-1">
						<CardTitle className="flex items-center gap-2 tracking-tight">
							<ShieldCheck className="size-4 text-primary" aria-hidden />
							Host preflight
							<HelpTip
								label="Host preflight"
								text="Read-only checks for the compositor, capture environment, encoder, configuration, storage, and running host conflicts."
							/>
						</CardTitle>
						<CardDescription>
							Run this before pairing a new client or changing capture settings.
						</CardDescription>
					</div>
					<Button
						variant="ghost"
						size="icon"
						title="Refresh preflight checks"
						aria-label="Refresh preflight checks"
						onClick={() => query.refetch()}
						disabled={query.isFetching}
					>
						<RefreshCw
							className={cn("size-4", query.isFetching && "animate-spin")}
						/>
					</Button>
				</div>
			</CardHeader>
			<CardContent className="pt-4 sm:pt-5">
				<QueryState
					isLoading={query.isLoading}
					error={query.error}
					refetch={query.refetch}
				>
					{report && (
						<div className="space-y-3">
							<div
								className={cn(
									"rounded-md border px-3 py-2 text-sm",
									report.ready
										? "border-success/30 bg-success/5 text-[var(--success)]"
										: "border-destructive/30 bg-destructive/5 text-destructive",
								)}
							>
								{report.ready
									? "The host passed its preflight checks."
									: "The host has a blocked check. Resolve it before starting a stream."}
							</div>
							<ul className="divide-y divide-border/60 rounded-md border border-border/60">
								{report.checks.map((item) => (
									<li key={item.id} className="space-y-1 px-3 py-2.5">
										<div className="flex items-center gap-2 text-sm font-medium">
											<span className={statusClasses[item.status]}>
												<StatusIcon status={item.status} />
											</span>
											<span className="min-w-0 flex-1">{item.label}</span>
											<span
												className={cn(
													"text-xs font-medium",
													statusClasses[item.status],
												)}
											>
												{statusLabel[item.status]}
											</span>
										</div>
										<p className="pl-6 text-xs leading-relaxed text-muted-foreground">
											{item.detail}
										</p>
										{item.remediation && (
											<p className="pl-6 text-xs font-medium text-foreground/80">
												{item.remediation}
											</p>
										)}
									</li>
								))}
							</ul>
						</div>
					)}
				</QueryState>
			</CardContent>
		</Card>
	);
};
