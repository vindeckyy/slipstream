import { Gamepad2, XCircle } from "lucide-react";
import type { FC } from "react";
import type { ActiveGame } from "@/api/gen/model/activeGame";
import type { GameEntry } from "@/api/gen/model/gameEntry";
import { HelpTip } from "@/components/option-help";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages";

/**
 * What the host is running, and — when a client has vanished — how long its game has left.
 *
 * The `grace` rows are the reason this card exists at all. A game whose session is gone is on a
 * countdown to being closed, which costs unsaved progress; that has to be visible somewhere, with a
 * way to say "yes, do it now" or (by reconnecting) "no". The live rows come almost free alongside.
 */
export const RunningGames: FC<{
	games: ActiveGame[];
	/** The catalog, for box art — matched by `app_id`. Absent while `/library` is still loading. */
	library?: GameEntry[];
	onEnd: (game: ActiveGame) => void;
	isEnding: boolean;
}> = ({ games, library, onEnd, isEnding }) => {
	// Older hosts omit `games` from `/status`; treat missing as empty so Home does not crash.
	const rows = games ?? [];
	if (rows.length === 0) return null;
	return (
		<Card>
			<CardHeader className="pb-3 sm:pb-3">
				<CardTitle className="flex items-center gap-2">
					<Gamepad2 className="size-4 shrink-0 text-muted-foreground" />
					{m.games_title()}
					<HelpTip
						label={m.games_title()}
						text="Games the host is launching, running, or holding in a grace countdown after a client disconnect. End now closes a grace title early, or stops the live session for an active title."
					/>
				</CardTitle>
			</CardHeader>
			<CardContent className="flex flex-col gap-2">
				{rows.map((g, i) => (
					<GameRow
						// A host can run the same title for two clients at once (native admits concurrent
						// sessions), so the title alone is not a key. The index is the last resort: every
						// grace row has a null `session_id`, so two waiting copies of the same title on the
						// same plane produced identical keys and React collapsed them into one row.
						key={`${g.plane}:${g.session_id ?? "grace"}:${g.app_id ?? g.title}:${i}`}
						game={g}
						art={coverFor(g, library)}
						onEnd={() => onEnd(g)}
						isEnding={isEnding}
					/>
				))}
			</CardContent>
		</Card>
	);
};

const endNowHelp = (waiting: boolean): string =>
	waiting
		? "Close this game now instead of waiting out the grace countdown. Unsaved progress in the game may be lost."
		: "Stop this running game by ending its session. With multiple live sessions the console confirms first, because one stop ends all of them.";

const GameRow: FC<{
	game: ActiveGame;
	art?: string;
	onEnd: () => void;
	isEnding: boolean;
}> = ({ game, art, onEnd, isEnding }) => {
	const waiting = game.state === "grace";
	return (
		<div
			className={cn(
				"flex items-center gap-3 rounded-lg border border-transparent px-2 py-2 sm:px-3",
				waiting ? "border-destructive/30 bg-destructive/10" : "bg-muted/30",
			)}
		>
			{/* Fixed slot so rows line up whether or not a title has a cover. Plenty won't: an
			    operator-typed command has no catalog entry behind it, a custom entry may carry no
			    art, and nothing does until `/library` has loaded — an empty box reads as broken, so
			    the placeholder says "game" instead of nothing. */}
			<div className="flex h-14 w-10 shrink-0 items-center justify-center overflow-hidden rounded-md bg-muted">
				{art ? (
					<img
						src={art}
						alt=""
						loading="lazy"
						className="size-full object-cover"
					/>
				) : (
					<Gamepad2 className="size-4 text-muted-foreground/60" />
				)}
			</div>
			<div className="min-w-0 flex-1">
				<div className="flex flex-wrap items-center gap-2">
					<span className="truncate text-sm font-semibold">{game.title}</span>
					<Badge variant={waiting ? "destructive" : stateVariant(game.state)}>
						{stateLabel(game.state)}
					</Badge>
				</div>
				<p className="mt-0.5 truncate text-xs text-muted-foreground">
					{waiting
						? m.games_closing_in({
								time: formatCountdown(game.grace_remaining_s ?? 0),
							})
						: [game.client, planeLabel(game.plane)].filter(Boolean).join(" · ")}
				</p>
			</div>
			<div className="flex shrink-0 items-center gap-1">
				<Button
					variant={waiting ? "destructive" : "outline"}
					size="sm"
					disabled={isEnding}
					title={endNowHelp(waiting)}
					onClick={onEnd}
				>
					<XCircle className="size-3.5" />
					{m.games_end_now()}
				</Button>
				<HelpTip label={m.games_end_now()} text={endNowHelp(waiting)} />
			</div>
		</div>
	);
};

/**
 * Box art for a running game, matched against the already-fetched catalog by store-qualified id —
 * so this card costs no extra request. `undefined` for an operator-typed GameStream command (no
 * catalog entry behind it) or a title whose entry carries no art.
 */
function coverFor(game: ActiveGame, library?: GameEntry[]): string | undefined {
	if (!game.app_id || !library) return undefined;
	const entry = library.find((e) => e.id === game.app_id);
	return entry?.art.portrait ?? entry?.art.header ?? undefined;
}

/** `mm:ss` — the countdown reads as a duration, not a number of seconds. */
function formatCountdown(seconds: number): string {
	const s = Math.max(0, Math.floor(seconds));
	return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

function stateLabel(state: string): string {
	switch (state) {
		case "launching":
			return m.games_state_launching();
		case "running":
			return m.games_state_running();
		case "exited":
			return m.games_state_exited();
		case "grace":
			return m.games_state_grace();
		default:
			return state;
	}
}

function stateVariant(state: string): "success" | "secondary" | "outline" {
	if (state === "running") return "success";
	if (state === "launching") return "secondary";
	return "outline";
}

/** The compat plane is worth naming: it is why a row may show an IP instead of a device name. */
function planeLabel(plane: string): string {
	return plane === "gamestream" ? "GameStream" : "";
}
