import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@unom/ui/button";
import { Power, RotateCcw } from "lucide-react";
import { type FC, useEffect, useState } from "react";
import { getGetHostInfoQueryKey, useGetStatus } from "@/api/gen/host/host";
import { HelpTip } from "@/components/option-help";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";
import { m } from "@/paraglide/messages";

type PowerPhase = "idle" | "restarting" | "offline";

const RECONNECT_TIMEOUT_MS = 2 * 60 * 1000;
const POLL_MS = 1500;

/**
 * Restart / shut down the Slipstream host process (not the OS). Confirm dialog only — session
 * cookie is enough; no password re-prompt (unlike Update → Apply).
 */
export const PowerCard: FC = () => {
	const queryClient = useQueryClient();
	const status = useGetStatus({
		query: { refetchInterval: 5_000 },
	});
	const streamActive =
		Boolean(status.data?.video_streaming) ||
		(status.data?.active_sessions ?? 0) > 0;

	const [phase, setPhase] = useState<PowerPhase>("idle");
	const [confirm, setConfirm] = useState<"restart" | "shutdown" | null>(null);
	const [busy, setBusy] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [timedOut, setTimedOut] = useState(false);

	useEffect(() => {
		if (phase !== "restarting" && phase !== "offline") return;
		const started = Date.now();
		let cancelled = false;
		let sawOffline = false;
		const tick = async () => {
			while (!cancelled) {
				try {
					const res = await fetch("/api/v1/host", {
						credentials: "same-origin",
						});
					if (phase === "restarting" && res.ok) {
						await queryClient.invalidateQueries({
							queryKey: getGetHostInfoQueryKey(),
						});
						setPhase("idle");
						setTimedOut(false);
						return;
					}
					if (phase === "offline") {
						if (!res.ok) sawOffline = true;
						if (sawOffline && res.ok) {
							// Offline phase: operator started the host by hand after shutdown.
							setPhase("idle");
							setTimedOut(false);
							return;
						}
					}
				} catch {
					// Expected while the host is down.
					if (phase === "offline") sawOffline = true;
				}
				if (phase === "restarting" && Date.now() - started > RECONNECT_TIMEOUT_MS) {
					setTimedOut(true);
					return;
				}
				await new Promise((r) => setTimeout(r, POLL_MS));
			}
		};
		void tick();
		return () => {
			cancelled = true;
		};
	}, [phase, queryClient]);

	const run = async (action: "restart" | "shutdown") => {
		setBusy(true);
		setError(null);
		try {
			const res = await fetch(
				action === "restart"
					? "/api/v1/host/restart"
					: "/api/v1/host/shutdown",
				{
					method: "POST",
					credentials: "same-origin",
				},
			);
			if (res.status === 202) {
				setConfirm(null);
				setPhase(action === "restart" ? "restarting" : "offline");
				setTimedOut(false);
				return;
			}
			const body = (await res.json().catch(() => null)) as {
				error?: string;
			} | null;
			setError(
				body?.error ??
					(action === "restart"
						? m.host_power_restart_failed()
						: m.host_power_shutdown_failed()),
			);
		} catch {
			// After 202 the socket often dies mid-flight — treat that as success for the action
			// we just asked for.
			setConfirm(null);
			setPhase(action === "restart" ? "restarting" : "offline");
		} finally {
			setBusy(false);
		}
	};

	return (
		<Card className="overflow-hidden">
			<CardHeader className="border-b border-border/60 bg-muted/15">
				<CardTitle className="flex items-center gap-2 tracking-tight">
					<Power className="size-4 text-primary" aria-hidden />
					{m.host_power()}
					<HelpTip label={m.host_power()} text={m.host_power_help()} />
				</CardTitle>
			</CardHeader>
			<CardContent className="space-y-4 pt-4 sm:pt-5">
				{phase === "restarting" && (
					<div
						role="status"
						className="flex items-start gap-3 rounded-lg border border-border/70 bg-muted/20 px-3.5 py-3 text-sm"
					>
						<Spinner className="mt-0.5 size-4 shrink-0" />
						<div className="min-w-0 space-y-1">
							<p className="font-medium">{m.host_power_restarting()}</p>
							<p className="text-muted-foreground">
								{timedOut
									? m.host_power_timed_out()
									: m.host_power_restarting_hint()}
							</p>
						</div>
					</div>
				)}
				{phase === "offline" && (
					<div
						role="status"
						className="rounded-lg border border-border/70 bg-muted/20 px-3.5 py-3 text-sm"
					>
						<p className="font-medium">{m.host_power_offline()}</p>
						<p className="mt-1 text-muted-foreground">
							{m.host_power_offline_hint()}
						</p>
					</div>
				)}
				{phase === "idle" && (
					<div className="flex flex-col gap-3 sm:flex-row sm:flex-wrap">
						<div className="flex items-center gap-1.5">
							<Button
								size="sm"
								variant="default"
								onClick={() => {
									setError(null);
									setConfirm("restart");
								}}
								title={m.host_power_restart_help()}
							>
								<RotateCcw className="size-3.5" aria-hidden />
								{m.host_power_restart()}
							</Button>
							<HelpTip
								label={m.host_power_restart()}
								text={m.host_power_restart_help()}
							/>
						</div>
						<div className="flex items-center gap-1.5">
							<Button
								size="sm"
								variant="destructive"
								onClick={() => {
									setError(null);
									setConfirm("shutdown");
								}}
								title={m.host_power_shutdown_help()}
							>
								<Power className="size-3.5" aria-hidden />
								{m.host_power_shutdown()}
							</Button>
							<HelpTip
								label={m.host_power_shutdown()}
								text={m.host_power_shutdown_help()}
							/>
						</div>
					</div>
				)}

				<Dialog
					open={confirm !== null}
					onOpenChange={(open) => {
						if (!open) {
							setConfirm(null);
							setError(null);
						}
					}}
				>
					<DialogContent>
						<DialogHeader>
							<DialogTitle>
								{confirm === "shutdown"
									? m.host_power_shutdown_confirm_title()
									: m.host_power_restart_confirm_title()}
							</DialogTitle>
							<DialogDescription>
								{confirm === "shutdown"
									? m.host_power_shutdown_confirm_body()
									: m.host_power_restart_confirm_body()}
							</DialogDescription>
						</DialogHeader>
						{streamActive && (
							<p className="text-sm font-medium text-destructive">
								{m.host_power_stream_warning()}
							</p>
						)}
						{error && <p className="text-sm text-destructive">{error}</p>}
						<DialogFooter>
							<Button
								type="button"
								variant="outline"
								disabled={busy}
								onClick={() => setConfirm(null)}
							>
								{m.host_power_cancel()}
							</Button>
							<Button
								type="button"
								variant={
									confirm === "shutdown" ? "destructive" : "default"
								}
								disabled={busy || confirm === null}
								onClick={() => {
									if (confirm) void run(confirm);
								}}
							>
								{busy ? <Spinner className="size-3.5" /> : null}
								{m.host_power_confirm()}
							</Button>
						</DialogFooter>
					</DialogContent>
				</Dialog>
			</CardContent>
		</Card>
	);
};
