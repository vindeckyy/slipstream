import { useQueryClient } from "@tanstack/react-query";
import { Trash2 } from "lucide-react";
import type { FC } from "react";
import {
	getListPairedClientsQueryKey,
	useListPairedClients,
	useUnpairClient,
} from "@/api/gen/clients/clients";
import {
	getListNativeClientsQueryKey,
	useListNativeClients,
	useUnpairNativeClient,
} from "@/api/gen/native/native";
import { HelpTip, RecommendedMark } from "@/components/option-help";
import { QueryState } from "@/components/query-state";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import { m } from "@/paraglide/messages";

/** The two pairing protocols a device can be paired over. */
export type PairedProtocol = "native" | "moonlight";

/** One paired device, normalized across the native + Moonlight lists. */
export interface PairedRow {
	protocol: PairedProtocol;
	fingerprint: string;
	/** Native devices carry a name; Moonlight clients carry a cert subject; either may be empty. */
	name: string;
}

/**
 * Container: ALL paired devices in one list. Merges the native (slipstream/1) clients and the
 * GameStream/Moonlight clients — two separate host endpoints — into a single table tagged by
 * protocol, and routes each unpair back to the right endpoint.
 */
export const PairedDevicesSection: FC = () => {
	const qc = useQueryClient();
	const native = useListNativeClients();
	const moonlight = useListPairedClients();
	const unpairNative = useUnpairNativeClient();
	const unpairMoonlight = useUnpairClient();

	const rows: PairedRow[] = [
		...(native.data ?? []).map(
			(c): PairedRow => ({
				protocol: "native",
				fingerprint: c.fingerprint,
				name: c.name,
			}),
		),
		...(moonlight.data ?? []).map(
			(c): PairedRow => ({
				protocol: "moonlight",
				fingerprint: c.fingerprint,
				name: c.subject ?? "",
			}),
		),
	];

	const onUnpair = (protocol: PairedProtocol, fingerprint: string) => {
		if (!confirm(m.pairing_native_unpair_confirm())) return;
		if (protocol === "native") {
			unpairNative.mutate(
				{ fingerprint },
				{
					onSuccess: () =>
						qc.invalidateQueries({ queryKey: getListNativeClientsQueryKey() }),
				},
			);
		} else {
			unpairMoonlight.mutate(
				{ fingerprint },
				{
					onSuccess: () =>
						qc.invalidateQueries({ queryKey: getListPairedClientsQueryKey() }),
				},
			);
		}
	};

	// The fingerprint of the row whose unpair is in flight (if any) — so only THAT row's button
	// disables, not every row's.
	const pendingFingerprint =
		(unpairNative.isPending
			? unpairNative.variables?.fingerprint
			: undefined) ??
		(unpairMoonlight.isPending
			? unpairMoonlight.variables?.fingerprint
			: undefined) ??
		null;

	return (
		<PairedDevices
			rows={rows}
			isLoading={native.isLoading || moonlight.isLoading}
			error={native.error ?? moonlight.error}
			refetch={() => {
				native.refetch();
				moonlight.refetch();
			}}
			onUnpair={onUnpair}
			pendingFingerprint={pendingFingerprint}
		/>
	);
};

/** All paired devices (native + Moonlight) in one table, differentiated by a protocol badge. */
export const PairedDevices: FC<{
	rows: PairedRow[];
	isLoading: boolean;
	error: unknown;
	refetch: () => void;
	onUnpair: (protocol: PairedProtocol, fingerprint: string) => void;
	/** Fingerprint of the row whose unpair is in flight, or null — only that row disables. */
	pendingFingerprint: string | null;
}> = ({ rows, isLoading, error, refetch, onUnpair, pendingFingerprint }) => (
	<Card>
		<CardHeader>
			<div className="space-y-1">
				<CardTitle className="flex items-center gap-1.5 tracking-tight">
					{m.pairing_native_devices()}
					<HelpTip
						label={m.pairing_native_devices()}
						text="Every device trusted by this host, across Slipstream (native) and Moonlight. After pairing, reconnects need no PIN. Unpair removes trust so the device must pair again to stream."
					/>
				</CardTitle>
				<RecommendedMark value="Keep only devices you still use. Unpair lost, shared, or retired clients." />
			</div>
		</CardHeader>

		<CardContent flush>
			<QueryState isLoading={isLoading} error={error} refetch={refetch}>
				{rows.length === 0 ? (
					<p className="mx-4 mb-4 rounded-lg border border-dashed border-border/70 bg-muted/20 px-4 py-8 text-center text-sm text-muted-foreground sm:mx-6 sm:mb-6">
						{m.pairing_native_empty()}
					</p>
				) : (
					<div className="overflow-x-auto">
						<Table>
							<TableHeader>
								<TableRow className="hover:bg-transparent">
									<TableHead>
										<span className="inline-flex items-center gap-1.5">
											{m.clients_name()}
											<HelpTip
												label={m.clients_name()}
												text="Friendly label for native devices, or the certificate subject for Moonlight clients. Empty means the client did not send a name."
											/>
										</span>
									</TableHead>
									<TableHead>
										<span className="inline-flex items-center gap-1.5">
											{m.pairing_protocol()}
											<HelpTip
												label={m.pairing_protocol()}
												text="slipstream/1 is the native Slipstream client path (Pair a device or Approve). Moonlight is GameStream-compatible clients paired via the Moonlight PIN card."
											/>
										</span>
									</TableHead>
									<TableHead>
										<span className="inline-flex items-center gap-1.5">
											{m.clients_fingerprint()}
											<HelpTip
												label={m.clients_fingerprint()}
												text="Short view of the client's cryptographic identity. The host pins this after pairing so reconnects are automatic."
											/>
										</span>
									</TableHead>
									<TableHead className="w-12">
										<span className="sr-only">{m.action_unpair()}</span>
									</TableHead>
								</TableRow>
							</TableHeader>
							<TableBody>
								{rows.map((r) => (
									<TableRow key={`${r.protocol}:${r.fingerprint}`}>
										<TableCell className="font-medium">
											{r.name || "—"}
										</TableCell>
										<TableCell>
											<Badge
												variant={
													r.protocol === "native" ? "default" : "secondary"
												}
											>
												{r.protocol === "native"
													? m.pairing_protocol_native()
													: m.pairing_protocol_moonlight()}
											</Badge>
										</TableCell>
										<TableCell className="font-mono text-xs text-muted-foreground">
											{r.fingerprint.slice(0, 16)}…
										</TableCell>
										<TableCell>
											<div className="flex items-center justify-end gap-1">
												<Button
													variant="ghost"
													size="icon"
													aria-label={m.action_unpair()}
													disabled={pendingFingerprint === r.fingerprint}
													onClick={() => onUnpair(r.protocol, r.fingerprint)}
												>
													<Trash2 className="size-4 text-destructive" />
												</Button>
												<HelpTip
													label={m.action_unpair()}
													text="Removes this device from the allow-list. It must complete pairing again before it can connect."
												/>
											</div>
										</TableCell>
									</TableRow>
								))}
							</TableBody>
						</Table>
					</div>
				)}
			</QueryState>
		</CardContent>
	</Card>
);
