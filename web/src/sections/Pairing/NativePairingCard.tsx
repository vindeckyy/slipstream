import { useQueryClient } from "@tanstack/react-query";
import { KeyRound, Smartphone, Timer } from "lucide-react";
import { type FC, useEffect, useRef } from "react";
import type { NativePairStatus } from "@/api/gen/model/nativePairStatus";
import {
	getGetNativePairingQueryKey,
	getListNativeClientsQueryKey,
	useArmNativePairing,
	useDisarmNativePairing,
	useGetNativePairing,
} from "@/api/gen/native/native";
import { HelpTip, RecommendedMark } from "@/components/option-help";
import { QueryState } from "@/components/query-state";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import type { Loadable } from "@/lib/query";
import { m } from "@/paraglide/messages";

/** Seconds → `m:ss`. */
function fmtTime(secs: number): string {
	const s = Math.max(0, Math.floor(secs));
	return `${Math.floor(s / 60)}:${(s % 60).toString().padStart(2, "0")}`;
}

/**
 * Container: native (slipstream/1) pairing — arm a window, poll fast while armed
 * for the live countdown, slow otherwise.
 */
export const NativePairingSection: FC = () => {
	const qc = useQueryClient();
	const native = useGetNativePairing({
		query: { refetchInterval: (q) => (q.state.data?.armed ? 1_000 : 4_000) },
	});
	const arm = useArmNativePairing();
	const disarm = useDisarmNativePairing();

	// A device pairs via the QUIC PIN ceremony, NOT through approve/deny, so nothing else
	// invalidates the paired-devices list on the happy path — it would stay stale until remount.
	// The status poll's `paired_clients` count is the pairing signal: when it rises, refresh the
	// list so the newly paired device appears immediately.
	const pairedCount = native.data?.paired_clients;
	const prevPairedCount = useRef(pairedCount);
	useEffect(() => {
		if (
			prevPairedCount.current !== undefined &&
			pairedCount !== undefined &&
			pairedCount !== prevPairedCount.current
		) {
			qc.invalidateQueries({ queryKey: getListNativeClientsQueryKey() });
		}
		prevPairedCount.current = pairedCount;
	}, [pairedCount, qc]);

	const refresh = () =>
		qc.invalidateQueries({ queryKey: getGetNativePairingQueryKey() });
	const onArm = () =>
		arm.mutate({ data: { ttl_secs: 120 } }, { onSuccess: refresh });
	const onDisarm = () => disarm.mutate(undefined, { onSuccess: refresh });

	return (
		<NativePairingCard
			status={native}
			onArm={onArm}
			onDisarm={onDisarm}
			isArming={arm.isPending}
			isDisarming={disarm.isPending}
		/>
	);
};

/** Native (slipstream/1) pairing: arm a window → DISPLAY the PIN the user enters on their device. */
export const NativePairingCard: FC<{
	status: Loadable<NativePairStatus>;
	onArm: () => void;
	onDisarm: () => void;
	isArming: boolean;
	isDisarming: boolean;
}> = ({ status, onArm, onDisarm, isArming, isDisarming }) => {
	const d = status.data;
	return (
		<QueryState
			isLoading={status.isLoading}
			error={status.error}
			refetch={status.refetch}
		>
			<Card className="h-full">
				<CardHeader>
					<CardTitle className="flex items-center gap-2 tracking-tight">
						<Smartphone className="size-4 text-muted-foreground" />
						<span className="flex min-w-0 items-center gap-1.5">
							{m.pairing_native_title()}
							<HelpTip
								label={m.pairing_native_title()}
								text="Arms a 2-minute window and shows a one-time PIN here. Enter that PIN in your Slipstream app (or CLI) to finish pairing. Random devices cannot pair while this is disarmed."
							/>
						</span>
					</CardTitle>
				</CardHeader>
				<CardContent className="space-y-4">
					{!d?.enabled ? (
						<p className="rounded-lg border border-dashed border-border/70 bg-muted/20 px-4 py-8 text-center text-sm text-muted-foreground">
							{m.pairing_native_disabled()}
						</p>
					) : d.armed && d.pin ? (
						<div className="space-y-4">
							<div className="space-y-1">
								<div className="flex items-center gap-1.5">
									<p className="text-sm">{m.pairing_native_enter()}</p>
									<HelpTip
										label={m.pairing_native_enter()}
										text="This PIN is shown only while pairing is armed. Type it on the Slipstream client (Pair with PIN, or Pair & Stream on Steam Deck). Wrong guesses fail the ceremony; arm again for a fresh PIN."
									/>
								</div>
								<RecommendedMark value="Stay on this page until the client confirms, then check Paired devices below." />
							</div>
							<div className="rounded-xl border border-border/70 bg-muted/30 py-6 text-center font-mono text-4xl font-semibold tracking-[0.3em] tabular-nums">
								{d.pin}
							</div>
							{d.expires_in_secs != null && (
								<p className="flex items-center justify-center gap-1.5 text-sm text-muted-foreground">
									<Timer className="size-4" />
									{m.pairing_native_expires()} {fmtTime(d.expires_in_secs)}
								</p>
							)}
							<div className="flex w-full items-center gap-1.5">
								<Button
									variant="outline"
									className="min-w-0 flex-1"
									disabled={isDisarming}
									onClick={onDisarm}
								>
									{m.pairing_native_cancel()}
								</Button>
								<HelpTip
									label={m.pairing_native_cancel()}
									text="Ends the pairing window early and discards this PIN. Arm again when you are ready at the device."
								/>
							</div>
						</div>
					) : (
						<>
							<CardDescription className="text-sm leading-relaxed text-muted-foreground">
								{m.pairing_native_desc()}
							</CardDescription>
							<RecommendedMark value="Arm when you are standing at the device ready to type the PIN. The window lasts 2 minutes." />
							<div className="flex items-center gap-1.5">
								<Button disabled={isArming} onClick={onArm}>
									<KeyRound className="size-4" />
									{m.pairing_native_arm()}
								</Button>
								<HelpTip
									label={m.pairing_native_arm()}
									text="Starts a 2-minute pairing window and displays a PIN on this card. Open your Slipstream app, select this host, and enter that PIN to trust it."
								/>
							</div>
						</>
					)}
				</CardContent>
			</Card>
		</QueryState>
	);
};
