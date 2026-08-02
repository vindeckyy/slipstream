import Section from "@unom/ui/section";
import { Cpu, Globe2, Network, Radio, Server, ShieldCheck } from "lucide-react";
import type { FC, ReactNode } from "react";
import type { HostInfo } from "@/api/gen/model/hostInfo";
import { HelpTip } from "@/components/option-help";
import { OsIcon } from "@/components/os-icon";
import { QueryState } from "@/components/query-state";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { Loadable } from "@/lib/query";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages";
import { ConnectCard } from "./ConnectCard";

export const HostView: FC<{
	host: Loadable<HostInfo>;
	/** The GPU inventory/selection card (a self-contained container — see `GpuCard.tsx`). */
	gpu?: ReactNode;
	/** The update-check card (a self-contained container — see `UpdateCard.tsx`). */
	update?: ReactNode;
	/** Restart / shut down the host process (see `PowerCard.tsx`). */
	power?: ReactNode;
	/** Warning about other Moonlight-compatible servers on this machine — renders nothing when
	 * there are none (see `ConflictsCard.tsx`). Sits at the top: it explains "nothing can connect". */
	conflicts?: ReactNode;
	/** Read-only capture and runtime checks for the host (see `PreflightCard.tsx`). */
	preflight?: ReactNode;
}> = ({ host, gpu, update, power, conflicts, preflight }) => {
	const h = host.data;
	return (
		<Section maxWidth={false}>
			<div className="flex flex-col gap-5">
				<div className="relative overflow-hidden rounded-xl border border-primary/30 bg-card/90 shadow-sm ring-1 ring-primary/10">
					<div
						aria-hidden
						className="pointer-events-none absolute inset-y-0 right-0 w-2/3 bg-gradient-to-l from-primary/10 via-primary/5 to-transparent"
					/>
					<div className="relative space-y-5 p-4 sm:p-6">
						<div className="flex flex-wrap items-start justify-between gap-4">
							<div className="space-y-2">
								<div className="flex items-center gap-2 text-xs font-medium uppercase tracking-[0.18em] text-primary">
									<Server className="size-3.5" aria-hidden />
									<span>{m.nav_host()}</span>
								</div>
								<h1 className="text-2xl font-semibold tracking-tight sm:text-3xl">
									{h?.hostname || m.nav_host()}
								</h1>
								{h && (
									<div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
										<span>{m.host_uniqueid()}</span>
										<code className="font-mono">{h.uniqueid}</code>
									</div>
								)}
							</div>
							{h && (
								<div className="flex items-center gap-2 rounded-full border border-border/70 bg-muted/40 px-3 py-2">
									<OsIcon os={h.os} className="size-4 shrink-0" />
									<span className="text-sm font-medium">{h.os_name}</span>
								</div>
							)}
						</div>
						{h && (
							<dl className="grid gap-px overflow-hidden rounded-lg border border-border/70 bg-border/50 sm:grid-cols-3">
								<HeroMetric
									label={m.host_local_ip()}
									value={h.local_ip}
									icon={<Globe2 className="size-3.5" />}
									mono
								/>
								<HeroMetric
									label={m.host_version()}
									value={`${h.app_version} (${h.version})`}
									icon={<Cpu className="size-3.5" />}
								/>
								<HeroMetric
									label={m.host_abi()}
									value={String(h.abi_version)}
									icon={<ShieldCheck className="size-3.5" />}
									mono
								/>
							</dl>
						)}
					</div>
				</div>

				{conflicts}
				{preflight}
				{h && <ConnectCard host={h} />}

				<QueryState
					isLoading={host.isLoading}
					error={host.error}
					refetch={host.refetch}
				>
					{h && (
						<div className="grid gap-5 xl:grid-cols-[1.1fr_0.9fr]">
							<Card className="overflow-hidden">
								<CardHeader className="border-b border-border/60 bg-muted/15">
									<CardTitle className="flex items-center gap-2 tracking-tight">
										<Server className="size-4 text-primary" aria-hidden />
										{m.host_identity()}
										<HelpTip
											label={m.host_identity()}
											text="How this host identifies itself to clients: name, OS, local address, build, and unique ID used for pairing and deep links."
										/>
									</CardTitle>
								</CardHeader>
								<CardContent className="pt-4 sm:pt-5">
									<dl className="grid grid-cols-1 gap-px overflow-hidden rounded-lg border border-border/70 bg-border/50 sm:grid-cols-2">
										<Row label={m.host_hostname()} value={h.hostname} />
										{/* The OS mark resolves from the identity chain (h.os), which also
										    serves as the tooltip for the curious; the text is the pretty name. */}
										<Row
											label={m.host_os()}
											value={h.os_name}
											title={h.os}
											icon={<OsIcon os={h.os} className="size-4 shrink-0" />}
										/>
										<Row label={m.host_local_ip()} value={h.local_ip} mono />
										<Row
											label={m.host_version()}
											value={`${h.app_version} (${h.version})`}
										/>
										<Row
											label={m.host_abi()}
											value={String(h.abi_version)}
											mono
										/>
										<Row label={m.host_uniqueid()} value={h.uniqueid} mono />
									</dl>
								</CardContent>
							</Card>
							<div className="flex flex-col gap-5">
								<Card className="overflow-hidden">
									<CardHeader className="border-b border-border/60 bg-muted/15">
										<CardTitle className="flex items-center gap-2 tracking-tight">
											<Radio className="size-4 text-primary" aria-hidden />
											{m.host_codecs()}
											<HelpTip
												label={m.host_codecs()}
												text="Video codecs this host can advertise to clients. The session still negotiates what the GPU and client both support."
											/>
										</CardTitle>
									</CardHeader>
									<CardContent className="flex flex-wrap gap-2 pt-4 sm:pt-5">
										{(h.codecs ?? []).map((c) => (
											<Badge key={c} variant="secondary">
												{c.toUpperCase()}
											</Badge>
										))}
									</CardContent>
								</Card>
								<Card className="overflow-hidden">
									<CardHeader className="border-b border-border/60 bg-muted/15">
										<CardTitle className="flex items-center gap-2 tracking-tight">
											<Network className="size-4 text-primary" aria-hidden />
											{m.host_ports()}
											<HelpTip
												label={m.host_ports()}
												text="Ports clients use to reach this host. Keep them reachable on your LAN or through port forwards when streaming remotely."
											/>
										</CardTitle>
									</CardHeader>
									<CardContent className="pt-4 sm:pt-5">
										<dl className="grid grid-cols-2 gap-px overflow-hidden rounded-lg border border-border/70 bg-border/50 text-sm tabular-nums">
											{Object.entries(h.ports).map(([k, v]) => (
												<div key={k} className="min-w-0 bg-card px-3 py-3">
													<dt className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
														{k}
													</dt>
													<dd className="mt-1 font-mono font-medium">
														{v as number}
													</dd>
												</div>
											))}
										</dl>
									</CardContent>
								</Card>
							</div>
						</div>
					)}
				</QueryState>

				{update}

				{power}

				{gpu}
			</div>
		</Section>
	);
};

const HeroMetric: FC<{
	label: string;
	value: string;
	icon: ReactNode;
	mono?: boolean;
}> = ({ label, value, icon, mono }) => (
	<div className="min-w-0 bg-card/80 px-3 py-3 sm:px-4">
		<dt className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
			{label}
		</dt>
		<dd
			className={cn(
				"mt-1 flex min-w-0 items-center gap-1.5 text-sm font-semibold",
				mono && "font-mono text-xs",
			)}
		>
			<span aria-hidden="true" className="shrink-0 text-primary">
				{icon}
			</span>
			<span className="min-w-0 truncate" title={value}>
				{value}
			</span>
		</dd>
	</div>
);

const Row: FC<{
	label: string;
	value: string;
	mono?: boolean;
	/** Optional leading glyph inside the value cell (the OS mark). */
	icon?: ReactNode;
	/** Tooltip override — defaults to the value itself (which may be truncated). */
	title?: string;
}> = ({ label, value, mono, icon, title }) => (
	<div className="min-w-0 bg-card px-3 py-3">
		<dt className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
			{label}
		</dt>
		<dd
			className={cn(
				"mt-1.5",
				mono ? "truncate font-mono text-xs" : "text-sm font-medium",
				icon && "flex items-center gap-2",
			)}
			title={title ?? value}
		>
			{icon}
			{value}
		</dd>
	</div>
);
