import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Radio } from "lucide-react";
import { useState, type FC } from "react";
import {
	getGetHostConfigQueryKey,
	getHostConfig,
	setMoonlightBroadcast,
	useSetMoonlightBroadcast,
} from "@/api/gen/host/host";
import type {
	ApiError,
	HostConfigState,
} from "@/api/gen/model";
import { getGetHostInfoQueryKey } from "@/api/gen/host/host";
import type { HostInfo } from "@/api/gen/model/hostInfo";
import { HelpTip, SettingEffectBadge } from "@/components/option-help";
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

export const MoonlightCard: FC<{ host: HostInfo }> = ({ host }) => {
	const queryClient = useQueryClient();
	const config = useQuery({
		queryKey: getGetHostConfigQueryKey(),
		queryFn: getHostConfig,
		staleTime: 5_000,
	});
	const [requested, setRequested] = useState<boolean | null>(null);
	const toggle = useSetMoonlightBroadcast({
		mutation: {
			onSuccess: (state: HostConfigState) => {
				setRequested(null);
				queryClient.setQueryData(getGetHostConfigQueryKey(), state);
				void queryClient.invalidateQueries({
					queryKey: getGetHostInfoQueryKey(),
					refetchType: "all",
				});
			},
			onError: () => setRequested(null),
		},
	});

	const configured = config.data?.settings.network?.gamestream ?? null;
	const desired = requested ?? configured ?? host.gamestream;
	const busy = config.isLoading || toggle.isPending;
	const error =
		toggle.error && "error" in toggle.error
			? toggle.error.error
			: config.isError
				? "Could not load host configuration."
				: null;

	return (
		<Card className="overflow-hidden">
			<CardHeader className="border-b border-border/60 bg-muted/15">
				<CardTitle className="flex items-center gap-2 tracking-tight">
					<Radio className="size-4 text-primary" aria-hidden />
					Moonlight broadcast
					<HelpTip
						label="Moonlight broadcast"
						text="Enables the GameStream compatibility plane and LAN discovery for Moonlight clients. The change is stored and needs a host restart to take effect. Use it only on a trusted network."
					/>
					<SettingEffectBadge effect="restart-required" />
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
						onCheckedChange={(enabled) => toggle.mutate({ data: { enabled } })}
						aria-label="Broadcast to Moonlight clients"
						className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
					/>
				</div>
				<div className="flex flex-wrap items-center gap-2 text-sm">
					<span
						aria-hidden
						className={cn(
							"size-2 rounded-full",
							host.gamestream ? "bg-success" : "bg-muted-foreground/40",
						)}
					/>
					<span className="text-muted-foreground">
						{host.gamestream
							? "Broadcasting to Moonlight clients"
							: "Moonlight broadcast is off"}
						{host.gamestream !== desired ? " (stored, awaiting restart)" : ""}
					</span>
					{host.gamestream ? <Badge variant="secondary">Active</Badge> : null}
					{toggle.isPending ? <Spinner className="size-4" /> : null}
				</div>
				{error ? (
					<p role="alert" className="text-sm text-destructive">
						{error}
					</p>
				) : null}
			</CardContent>
		</Card>
	);
};
