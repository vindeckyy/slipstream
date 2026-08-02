import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "@unom/ui/toast";
import { Radio } from "lucide-react";
import { useState, type FC } from "react";
import {
	getHostConfig,
	hostConfigQueryKey,
	setHostConfig,
	type HostConfigState,
} from "@/api/host-config";
import { getGetHostInfoQueryKey } from "@/api/gen/host/host";
import type { HostInfo } from "@/api/gen/model/hostInfo";
import { HelpTip } from "@/components/option-help";
import { Badge } from "@/components/ui/badge";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { Spinner } from "@/components/ui/spinner";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";

async function restartHost(): Promise<void> {
	const response = await fetch("/api/v1/host/restart", {
		method: "POST",
		credentials: "same-origin",
	});
	if (response.status === 202) return;
	const body = (await response.json().catch(() => null)) as {
		error?: string;
	} | null;
	throw new Error(body?.error || `Host restart failed (HTTP ${response.status})`);
}

export const MoonlightCard: FC<{ host: HostInfo }> = ({ host }) => {
	const queryClient = useQueryClient();
	const config = useQuery({
		queryKey: hostConfigQueryKey,
		queryFn: getHostConfig,
		staleTime: 5_000,
	});
	const [requested, setRequested] = useState<boolean | null>(null);
	const toggle = useMutation({
		mutationFn: async (enabled: boolean) => {
			if (!config.data) throw new Error("Host configuration is still loading.");
			const settings = structuredClone(config.data.settings);
			settings.network.gamestream = enabled;
			if (enabled) settings.network.mdns = true;
			const state = await setHostConfig(settings);
			await restartHost();
			return { state, enabled };
		},
		onMutate: (enabled) => setRequested(enabled),
		onSuccess: ({ state, enabled }: { state: HostConfigState; enabled: boolean }) => {
			setRequested(null);
			queryClient.setQueryData(hostConfigQueryKey, state);
			void queryClient.invalidateQueries({
				queryKey: getGetHostInfoQueryKey(),
				refetchType: "all",
			});
			toast.success(
				enabled
					? "Moonlight broadcast enabled. Restarting host."
					: "Moonlight broadcast disabled. Restarting host.",
			);
		},
		onError: () => setRequested(null),
	});

	const configured = config.data?.settings.network.gamestream ?? null;
	const desired = requested ?? configured ?? host.gamestream;
	const busy = config.isLoading || toggle.isPending;
	const statusText = toggle.isPending
		? "Restarting host..."
		: host.gamestream === desired
			? host.gamestream
				? "Broadcasting to Moonlight clients"
				: "Moonlight broadcast is off"
			: "Waiting for host restart";
	const error = toggle.error ?? (config.isError ? config.error : null);

	return (
		<Card className="overflow-hidden">
			<CardHeader className="border-b border-border/60 bg-muted/15">
				<CardTitle className="flex items-center gap-2 tracking-tight">
					<Radio className="size-4 text-primary" aria-hidden />
					Moonlight broadcast
					<HelpTip
						label="Moonlight broadcast"
						text="Enables the GameStream compatibility plane and LAN discovery for Moonlight clients. The host restarts to apply the switch. Use it only on a trusted network."
					/>
				</CardTitle>
				<CardDescription>
					Make this host visible to Moonlight clients with one switch.
				</CardDescription>
			</CardHeader>
			<CardContent className="space-y-4 pt-4 sm:pt-5">
				<div className="flex items-center justify-between gap-4">
					<div className="min-w-0 space-y-1">
						<p className="text-sm font-medium">Broadcast to Moonlight clients</p>
						<p className="text-xs leading-relaxed text-muted-foreground">
							Enabling this also turns on mDNS discovery so Moonlight can find the host on your LAN.
						</p>
					</div>
					<Switch
						id="moonlight-broadcast"
						checked={desired}
						disabled={busy || config.isError}
						onCheckedChange={(enabled) => toggle.mutate(enabled)}
						aria-label="Broadcast to Moonlight clients"
						className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
					/>
				</div>
				<div className="flex flex-wrap items-center gap-2 text-sm">
					<span
						aria-hidden
						className={cn(
							"size-2 rounded-full",
							host.gamestream ? "bg-emerald-500" : "bg-muted-foreground/40",
						)}
					/>
					<span className="text-muted-foreground">{statusText}</span>
					{host.gamestream ? <Badge variant="secondary">Active</Badge> : null}
					{toggle.isPending ? <Spinner className="size-4" /> : null}
				</div>
				{error ? (
					<p role="alert" className="text-sm text-destructive">
						{error instanceof Error ? error.message : "Could not update Moonlight broadcast."}
					</p>
				) : null}
			</CardContent>
		</Card>
	);
};
