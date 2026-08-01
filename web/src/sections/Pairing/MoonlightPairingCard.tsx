import { useQueryClient } from "@tanstack/react-query";
import { Info, KeyRound } from "lucide-react";
import { type FC, useEffect, useRef, useState } from "react";
import { getListPairedClientsQueryKey } from "@/api/gen/clients/clients";
import type { PairingStatus } from "@/api/gen/model/pairingStatus";
import {
	getGetPairingStatusQueryKey,
	useGetPairingStatus,
	useSubmitPairingPin,
} from "@/api/gen/pairing/pairing";
import {
	HelpTip,
	OptionLabel,
	RecommendedMark,
} from "@/components/option-help";
import { QueryState } from "@/components/query-state";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import type { Loadable } from "@/lib/query";
import { m } from "@/paraglide/messages";

/** Container: GameStream/Moonlight pairing — poll status, own the PIN entry, submit it. */
export const MoonlightPairingSection: FC = () => {
	const qc = useQueryClient();
	const [pin, setPin] = useState("");
	const pairing = useGetPairingStatus({ query: { refetchInterval: 2_000 } });
	const submit = useSubmitPairingPin();

	// Clear the previous attempt's outcome when a NEW pairing knock arrives.
	//
	// The mutation's success flag outlives the form — the section never unmounts, only the inner
	// <form> is conditional — so the green "PIN sent" note was still on screen above an empty PIN
	// box the next time Moonlight asked. Resetting inside `onSubmit` (the first attempt at this)
	// does nothing: `mutate` moves the status to pending in the same update, so `isSuccess` was
	// already about to go false. The transition that matters is `pin_pending` going false → true.
	const pending = pairing.data?.pin_pending ?? false;
	const wasPending = useRef(pending);
	useEffect(() => {
		if (pending && !wasPending.current) {
			submit.reset();
			setPin("");
		}
		wasPending.current = pending;
	}, [pending, submit.reset]);

	const onSubmit = () => {
		submit.mutate(
			{ data: { pin } },
			{
				onSuccess: () => {
					setPin("");
					qc.invalidateQueries({ queryKey: getGetPairingStatusQueryKey() });
					// The success message tells the operator to check the paired list, so refresh it —
					// both planes, since this card's count spans them.
					qc.invalidateQueries({ queryKey: getListPairedClientsQueryKey() });
				},
			},
		);
	};

	return (
		<MoonlightPairing
			pairing={pairing}
			pin={pin}
			onPinChange={setPin}
			onSubmit={onSubmit}
			isSubmitting={submit.isPending}
			isSuccess={submit.isSuccess}
			isError={submit.isError}
		/>
	);
};

/** GameStream/Moonlight pairing: the client shows a PIN, the operator submits it here. */
export const MoonlightPairing: FC<{
	pairing: Loadable<PairingStatus>;
	pin: string;
	onPinChange: (v: string) => void;
	onSubmit: () => void;
	isSubmitting: boolean;
	isSuccess: boolean;
	isError: boolean;
}> = ({
	pairing,
	pin,
	onPinChange,
	onSubmit,
	isSubmitting,
	isSuccess,
	isError,
}) => {
	const pending = pairing.data?.pin_pending ?? false;
	return (
		<QueryState
			isLoading={pairing.isLoading}
			error={pairing.error}
			refetch={pairing.refetch}
		>
			<Card className="h-full">
				<CardHeader>
					<CardTitle className="flex items-center gap-2 tracking-tight">
						<KeyRound className="size-4 text-muted-foreground" />
						<span className="flex min-w-0 items-center gap-1.5">
							{m.pairing_moonlight_title()}
							<HelpTip
								label={m.pairing_moonlight_title()}
								text="GameStream pairing runs the other way from native: Moonlight shows a PIN, and you type it here. This card only appears when the host runs with GameStream enabled. Arming is not used for Moonlight."
							/>
						</span>
					</CardTitle>
				</CardHeader>
				<CardContent>
					{!pending ? (
						<div className="space-y-3">
							<p className="rounded-lg border border-dashed border-border/70 bg-muted/20 px-4 py-8 text-center text-sm text-muted-foreground">
								{m.pairing_idle()}
							</p>
							<RecommendedMark value="On the Moonlight client, choose Pair. When a PIN appears there, this form unlocks so you can submit it." />
						</div>
					) : (
						<form
							onSubmit={(e) => {
								e.preventDefault();
								onSubmit();
							}}
							className="space-y-4"
						>
							<div className="space-y-1">
								<div className="flex items-center gap-1.5">
									<p className="text-sm font-medium">{m.pairing_waiting()}</p>
									<HelpTip
										label={m.pairing_waiting()}
										text="A Moonlight client is mid-handshake and waiting for you to confirm the PIN it displays. If the client gives up, this form returns to idle."
									/>
								</div>
								<RecommendedMark value="Type the PIN from Moonlight now, then Submit PIN before the client times out." />
							</div>
							<div className="space-y-2">
								<OptionLabel
									label={m.pairing_pin_label()}
									htmlFor="pin"
									help="Digits only, usually 4. Copy them from the Moonlight pairing dialog on the client (not from this host)."
									recommended="Enter the PIN exactly as Moonlight shows it."
								/>
								<Input
									id="pin"
									inputMode="numeric"
									autoComplete="off"
									maxLength={16}
									value={pin}
									onChange={(e) =>
										onPinChange(e.target.value.replace(/\D/g, ""))
									}
									placeholder="0000"
									className="font-mono text-lg tracking-widest"
								/>
							</div>
							<div className="flex items-center gap-1.5">
								<Button type="submit" disabled={pin.length < 4 || isSubmitting}>
									{m.pairing_submit()}
								</Button>
								<HelpTip
									label={m.pairing_submit()}
									text="Delivers the PIN to the waiting handshake. A success note means it was sent, not that pairing finished. If the PIN matches, the client completes pairing and appears under Paired devices."
								/>
							</div>
							{/* A 204 means the PIN was DELIVERED to the waiting handshake, not that pairing
							    succeeded — the ceremony verifies it out-of-band. So report "sent", not
							    "paired", and let the operator confirm via the Paired devices list. */}
							{isSuccess && (
								<p className="flex items-center gap-1.5 rounded-md border border-border/60 bg-muted/30 px-3 py-2 text-sm text-muted-foreground">
									<Info className="size-4 shrink-0" />
									{m.pairing_pin_sent()}
								</p>
							)}
							{isError && (
								<p className="text-sm text-destructive">{m.pairing_failed()}</p>
							)}
						</form>
					)}
				</CardContent>
			</Card>
		</QueryState>
	);
};
