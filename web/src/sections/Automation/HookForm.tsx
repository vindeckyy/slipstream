import { Checkbox } from "@unom/ui/form/checkbox";
import { type FC, useEffect, useState } from "react";
import type { HookEntry } from "@/api/gen/model/hookEntry";
import {
	HelpOption,
	HelpTip,
	OptionLabel,
	RecommendedMark,
} from "@/components/option-help";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { m } from "@/paraglide/messages";

/** The event kinds the host publishes, plus the `domain.*` wildcards the hook filter accepts.
 * Same vocabulary as the SSE `?kinds=` filter, so the two stay learnable together. */
export const EVENT_KINDS = [
	"client.*",
	"client.connected",
	"client.disconnected",
	"session.*",
	"session.started",
	"session.ended",
	"stream.*",
	"stream.started",
	"stream.stopped",
	"game.*",
	"game.running",
	"game.exited",
	"pairing.*",
	"pairing.pending",
	"pairing.completed",
	"pairing.denied",
	"display.*",
	"display.created",
	"display.released",
	"library.changed",
	"update.available",
	"update.applied",
	"host.started",
	"host.stopping",
] as const;

/** Hover text for each event kind: when it fires. */
const EVENT_KIND_HELP: Record<(typeof EVENT_KINDS)[number], string> = {
	"client.*":
		"Matches every client event (connected and disconnected).",
	"client.connected":
		"Fires when a client session is admitted (device name, fingerprint, plane).",
	"client.disconnected":
		"Fires when a client session goes away (quit, timeout, or error).",
	"session.*":
		"Matches every A/V session event (started and ended).",
	"session.started":
		"Fires when an A/V session registers (session id, client, mode, HDR). A solid starter for most hooks.",
	"session.ended":
		"Fires when an A/V session ends.",
	"stream.*":
		"Matches every stream event (started and stopped).",
	"stream.started":
		"Fires when video actually starts (mode, HDR, client, launched app when present).",
	"stream.stopped":
		"Fires when video stops. A desktop stream has no game; a stream can outlive its game.",
	"game.*":
		"Matches every launched-game event (running and exited).",
	"game.running":
		"Fires when a launched game's own process is seen running (not merely its launcher).",
	"game.exited":
		"Fires when a launched game is gone (exited by the player, or terminated by the host).",
	"pairing.*":
		"Matches every pairing event (pending, completed, denied).",
	"pairing.pending":
		"Fires once when an unpaired device knocks (not on every retry).",
	"pairing.completed":
		"Fires when a pairing is approved and stored.",
	"pairing.denied":
		"Fires when a pairing request is denied.",
	"display.*":
		"Matches every virtual-display event (created and released).",
	"display.created":
		"Fires when a virtual display is minted (backend and mode).",
	"display.released":
		"Fires when kept virtual displays are released.",
	"library.changed":
		"Fires when the game library is mutated (manual edit or a provider reconcile).",
	"update.available":
		"Fires once per discovered newer release (not on every update check).",
	"update.applied":
		"Fires on the new binary's first start after a successful update.",
	"host.started":
		"Fires when the host's serve planes come up (version, GameStream enabled).",
	"host.stopping":
		"Fires when the host is winding down.",
};

/** Recommended starter event for a first hook. */
const RECOMMENDED_EVENT = "session.started" as const;

const EMPTY: HookEntry = { on: RECOMMENDED_EVENT, run: "" };

/**
 * Add or edit one hook.
 *
 * A hook is either a shell command or a webhook — never both in this form, because "run this AND
 * post that" is two hooks and pretending otherwise makes the failure modes impossible to reason
 * about. The action kind is therefore a choice, not two optional fields.
 */
export const HookForm: FC<{
	/** The hook being edited, `EMPTY`-seeded for a new one, or null when closed. */
	value: HookEntry | null;
	onCancel: () => void;
	onSave: (hook: HookEntry) => void;
}> = ({ value, onCancel, onSave }) => {
	const [draft, setDraft] = useState<HookEntry>(EMPTY);
	const [kind, setKind] = useState<"run" | "webhook">("run");
	const [filtered, setFiltered] = useState(false);

	// Re-seed whenever a different hook is opened (the dialog stays mounted between edits).
	useEffect(() => {
		if (!value) return;
		setDraft(value);
		setKind(value.webhook ? "webhook" : "run");
		setFiltered(!!value.filter);
	}, [value]);

	const set = (patch: Partial<HookEntry>) =>
		setDraft((d) => ({ ...d, ...patch }));

	const action = kind === "run" ? (draft.run ?? "") : (draft.webhook ?? "");
	const ready = draft.on.trim().length > 0 && action.trim().length > 0;

	const commit = () => {
		// Emit exactly one action field, and drop an unticked filter entirely — leaving `{}` behind
		// would read as "filter on nothing" to anyone reading the config file later.
		const out: HookEntry = {
			on: draft.on.trim(),
			...(kind === "run"
				? { run: action.trim(), webhook: null }
				: { webhook: action.trim(), run: null }),
			...(filtered && draft.filter ? { filter: draft.filter } : {}),
			...(draft.debounce_ms ? { debounce_ms: draft.debounce_ms } : {}),
			...(draft.timeout_s ? { timeout_s: draft.timeout_s } : {}),
			...(kind === "webhook" && draft.hmac_secret_file
				? { hmac_secret_file: draft.hmac_secret_file }
				: {}),
		};
		onSave(out);
	};

	return (
		<Dialog open={value !== null} onOpenChange={(o) => !o && onCancel()}>
			<DialogContent className="max-h-[85vh] max-w-xl overflow-y-auto">
				<DialogHeader>
					<DialogTitle>{m.automation_hook_title()}</DialogTitle>
					<DialogDescription>{m.automation_hook_help()}</DialogDescription>
				</DialogHeader>

				<div className="space-y-4">
					<div className="space-y-2">
						<OptionLabel
							label={m.automation_field_on()}
							htmlFor="hook-on"
							help={m.automation_field_on_help()}
							recommended={`${RECOMMENDED_EVENT} for a first hook (or client.connected for connect/disconnect scripts)`}
						/>
						<select
							id="hook-on"
							value={draft.on}
							onChange={(e) => set({ on: e.target.value })}
							className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm shadow-sm outline-none transition-colors focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40 disabled:cursor-not-allowed disabled:opacity-50"
						>
							{EVENT_KINDS.map((k) => (
								<HelpOption
									key={k}
									value={k}
									title={EVENT_KIND_HELP[k]}
									recommended={k === RECOMMENDED_EVENT}
								>
									{k}
								</HelpOption>
							))}
						</select>
					</div>

					<fieldset className="space-y-2 rounded-lg border border-border/70 bg-muted/15 p-3">
						<OptionLabel
							label={m.automation_field_action()}
							help="One action per hook: a local shell command, or a webhook POST. Use two hooks if you need both. Webhooks cannot target loopback; use Run for anything on this machine."
							recommended="Run a command for local scripts"
						/>
						<div className="inline-flex rounded-lg border border-border/70 bg-background/60 p-0.5">
							{(["run", "webhook"] as const).map((k) => (
								<Button
									key={k}
									type="button"
									size="sm"
									variant={kind === k ? "secondary" : "ghost"}
									className="h-7 px-2.5"
									aria-pressed={kind === k}
									title={
										k === "run"
											? `${m.automation_action_run_help()} Recommended for local scripts.`
											: m.automation_action_webhook_help()
									}
									onClick={() => setKind(k)}
								>
									{k === "run"
										? m.automation_action_run()
										: m.automation_action_webhook()}
								</Button>
							))}
						</div>
						<Input
							id="hook-action"
							aria-label={m.automation_field_action()}
							autoComplete="off"
							spellCheck={false}
							value={action}
							placeholder={
								kind === "run" ? "/usr/local/bin/on-stream.sh" : "https://…"
							}
							onChange={(e) =>
								set(
									kind === "run"
										? { run: e.target.value }
										: { webhook: e.target.value },
								)
							}
						/>
						<p className="text-xs text-muted-foreground">
							{kind === "run"
								? m.automation_action_run_help()
								: m.automation_action_webhook_help()}
						</p>
					</fieldset>

					{kind === "webhook" && (
						<div className="space-y-2">
							<OptionLabel
								label={m.automation_field_hmac()}
								htmlFor="hook-hmac"
								help={m.automation_field_hmac_help()}
								recommended="Leave blank unless the receiver verifies X-Slipstream-Signature"
							/>
							<Input
								id="hook-hmac"
								autoComplete="off"
								spellCheck={false}
								value={draft.hmac_secret_file ?? ""}
								placeholder="/path/to/webhook-secret"
								onChange={(e) => set({ hmac_secret_file: e.target.value })}
							/>
						</div>
					)}

					<div className="space-y-2 rounded-lg border border-border/70 bg-muted/15 px-3 py-2.5">
						<label className="flex items-start gap-3 text-sm font-normal">
							<Checkbox
								checked={filtered}
								onCheckedChange={(n) => setFiltered(n === true)}
								className="mt-0.5"
							/>
							<span className="min-w-0 space-y-1">
								<span className="flex items-center gap-1.5">
									<span className="font-medium">
										{m.automation_field_filter()}
									</span>
									<HelpTip
										label={m.automation_field_filter()}
										text="When enabled, the hook only fires if every filled filter field matches the event exactly. Leave off unless you need per-client or per-game targeting."
									/>
								</span>
								<RecommendedMark value="Off, unless you need a specific client or game" />
							</span>
						</label>
					</div>

					{filtered && (
						<div className="grid gap-3 rounded-lg border border-border/70 bg-muted/10 p-3 sm:grid-cols-2">
							<div className="space-y-2">
								<OptionLabel
									label={m.automation_filter_client()}
									htmlFor="hook-client"
									help="Exact client/device name from the event (e.g. Living Room TV). Blank means any client."
								/>
								<Input
									id="hook-client"
									value={draft.filter?.client ?? ""}
									onChange={(e) =>
										set({ filter: { ...draft.filter, client: e.target.value } })
									}
								/>
							</div>
							<div className="space-y-2">
								<OptionLabel
									label={m.automation_filter_app()}
									htmlFor="hook-app"
									help="Exact game/app id from the event (e.g. steam:570). Blank means any app."
								/>
								<Input
									id="hook-app"
									value={draft.filter?.app ?? ""}
									onChange={(e) =>
										set({ filter: { ...draft.filter, app: e.target.value } })
									}
								/>
							</div>
						</div>
					)}

					<div className="grid gap-3 sm:grid-cols-2">
						<div className="space-y-2">
							<OptionLabel
								label={m.automation_field_debounce()}
								htmlFor="hook-debounce"
								help="Minimum milliseconds between firings of this hook. 0 (blank) means every matching event fires."
								recommended="0 / blank unless the event can spam"
							/>
							<Input
								id="hook-debounce"
								type="number"
								min={0}
								value={draft.debounce_ms ?? 0}
								onChange={(e) =>
									set({ debounce_ms: Number(e.target.value) || 0 })
								}
							/>
						</div>
						{kind === "run" && (
							<div className="space-y-2">
								<OptionLabel
									label={m.automation_field_timeout()}
									htmlFor="hook-timeout"
									help="Seconds before a run command's process group is killed (1-600). Leave unset for the host default of 30s."
									recommended="Blank / host default (30s)"
								/>
								<Input
									id="hook-timeout"
									type="number"
									min={1}
									max={600}
									value={draft.timeout_s ?? 30}
									onChange={(e) =>
										set({ timeout_s: Number(e.target.value) || 30 })
									}
								/>
							</div>
						)}
					</div>
				</div>

				<DialogFooter>
					<Button variant="outline" onClick={onCancel}>
						{m.common_cancel()}
					</Button>
					<Button disabled={!ready} onClick={commit}>
						{m.automation_hook_save()}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
};
