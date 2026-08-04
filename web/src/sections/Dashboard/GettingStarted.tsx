import { Link } from "@tanstack/react-router";
import {
	ArrowRight,
	Gamepad2,
	KeyRound,
	Monitor,
	ShieldCheck,
} from "lucide-react";
import type { FC, ReactNode } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { navDestination } from "@/lib/navigation";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages";

const HOST = navDestination("host");
const PAIRING = navDestination("pairing");
const LIBRARY = navDestination("library");
const DISPLAYS = navDestination("displays");

export type GettingStartedProps = {
	/** When true, the pair step shows a pending-PIN badge. */
	pinPending: boolean;
	/**
	 * Host preflight when known. Omit or pass `null` when the query has not
	 * returned (or failed); the Dashboard must stay usable either way.
	 */
	preflightReady?: boolean | null;
	onDismiss: () => void;
};

/**
 * Presentational first-run checklist. Visibility (paired count, local dismiss)
 * is owned by the Dashboard container so stories can drive every state.
 */
export const GettingStartedCard: FC<GettingStartedProps> = ({
	pinPending,
	preflightReady = null,
	onDismiss,
}) => {
	return (
		<Card className="overflow-hidden ring-[var(--ss-action)]/25">
			<CardHeader className="border-b border-border/60 bg-muted/15 pb-4 sm:pb-4">
				<CardTitle className="tracking-tight">
					{m.getting_started_title()}
				</CardTitle>
				<CardDescription>{m.getting_started_subtitle()}</CardDescription>
			</CardHeader>
			<CardContent className="pt-4 sm:pt-5">
				<ol className="flex flex-col gap-2">
					<StepRow
						to={HOST.to}
						icon={<ShieldCheck className="size-4" aria-hidden />}
						label={m.getting_started_host()}
						help={m.getting_started_host_help()}
						badge={
							preflightReady === true ? (
								<Badge variant="success">{m.getting_started_host_ready()}</Badge>
							) : preflightReady === false ? (
								<Badge variant="destructive">
									{m.getting_started_host_blocked()}
								</Badge>
							) : null
						}
					/>
					<StepRow
						to={PAIRING.to}
						icon={<KeyRound className="size-4" aria-hidden />}
						label={m.getting_started_pair()}
						help={m.getting_started_pair_help()}
						badge={
							pinPending ? (
								<Badge variant="warning">
									{m.getting_started_pair_pending()}
								</Badge>
							) : (
								<Badge variant="outline">
									{m.getting_started_pair_waiting()}
								</Badge>
							)
						}
					/>
					<StepRow
						to={LIBRARY.to}
						icon={<Gamepad2 className="size-4" aria-hidden />}
						label={m.getting_started_apps()}
						help={m.getting_started_apps_help()}
						secondary={
							<Link
								to={DISPLAYS.to}
								className="inline-flex min-h-9 items-center gap-1.5 rounded-md px-1 text-sm font-medium text-muted-foreground underline-offset-4 hover:text-foreground hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
							>
								<Monitor className="size-3.5" aria-hidden />
								{m.getting_started_displays()}
							</Link>
						}
					/>
				</ol>
			</CardContent>
			<CardFooter className="border-t border-border/60 pt-4">
				<Button
					type="button"
					variant="ghost"
					className="min-h-9 px-3"
					onClick={onDismiss}
				>
					{m.getting_started_skip()}
				</Button>
			</CardFooter>
		</Card>
	);
};

const StepRow: FC<{
	to: string;
	icon: ReactNode;
	label: string;
	help: string;
	badge?: ReactNode;
	secondary?: ReactNode;
}> = ({ to, icon, label, help, badge, secondary }) => (
	<li className="rounded-lg border border-border/60 bg-background/50">
		<Link
			to={to}
			className={cn(
				"flex min-h-12 items-start gap-3 px-3 py-3 sm:px-4",
				"rounded-lg transition-colors hover:bg-muted/40",
				"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50",
			)}
		>
			<span className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-md bg-muted text-foreground">
				{icon}
			</span>
			<span className="min-w-0 flex-1">
				<span className="flex flex-wrap items-center gap-2">
					<span className="text-sm font-medium text-foreground">{label}</span>
					{badge}
				</span>
				<span className="mt-0.5 block text-xs leading-relaxed text-muted-foreground">
					{help}
				</span>
			</span>
			<ArrowRight
				className="mt-1.5 size-4 shrink-0 text-muted-foreground"
				aria-hidden
			/>
		</Link>
		{secondary ? (
			<div className="border-t border-border/40 px-3 pb-3 pt-1 sm:px-4">
				{secondary}
			</div>
		) : null}
	</li>
);
