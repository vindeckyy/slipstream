import type { ComponentProps, ReactNode } from "react";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { StateNotice } from "./state";
import type {
	ObservatoryState,
	StateCopy,
	TimelineEvent,
	TimelineTone,
} from "./types";

const TONE_DOT: Record<TimelineTone, string> = {
	neutral: "bg-muted-foreground/60",
	info: "bg-sky-500",
	success: "bg-emerald-500",
	warning: "bg-amber-500",
	danger: "bg-red-500",
};

const TONE_RING: Record<TimelineTone, string> = {
	neutral: "border-muted-foreground/30 bg-muted/40",
	info: "border-sky-500/30 bg-sky-500/5",
	success: "border-emerald-500/30 bg-emerald-500/5",
	warning: "border-amber-500/30 bg-amber-500/5",
	danger: "border-red-500/30 bg-red-500/5",
};

export interface EventTimelineProps<Event extends TimelineEvent = TimelineEvent>
	extends Omit<ComponentProps<typeof Card>, "title" | "children"> {
	events?: readonly Event[];
	title?: ReactNode;
	description?: ReactNode;
	ariaLabel?: string;
	state?: ObservatoryState;
	stateCopy?: StateCopy;
	emptyMessage?: ReactNode;
	onRetry?: () => void;
	retryLabel?: ReactNode;
	renderEvent?: (event: Event, index: number) => ReactNode;
}

/** A deterministic timeline that renders supplied events and never performs live work. */
export function EventTimeline<Event extends TimelineEvent = TimelineEvent>({
	events = [],
	title,
	description,
	ariaLabel,
	state = "ready",
	stateCopy,
	emptyMessage,
	onRetry,
	retryLabel,
	renderEvent,
	className,
	...props
}: EventTimelineProps<Event>) {
	const effectiveState: ObservatoryState =
		events.length === 0 && state === "ready" ? "empty" : state;
	const nonReadyState = effectiveState === "ready" ? null : effectiveState;
	const effectiveCopy =
		emptyMessage === undefined
			? stateCopy
			: { ...stateCopy, empty: emptyMessage };

	return (
		<Card
			className={cn(
				"min-w-0",
				effectiveState === "stale" && "ring-warning/30",
				effectiveState === "error" && "ring-destructive/30",
				className,
			)}
			aria-busy={effectiveState === "loading" || undefined}
			data-state={effectiveState}
			{...props}
		>
			{title || description ? (
				<CardHeader className="space-y-1">
					{title ? <CardTitle className="text-base">{title}</CardTitle> : null}
					{description ? (
						<CardDescription>{description}</CardDescription>
					) : null}
				</CardHeader>
			) : null}
			<CardContent className="space-y-3">
				{nonReadyState && events.length === 0 ? (
					<StateNotice
						state={nonReadyState}
						stateCopy={effectiveCopy}
						onRetry={onRetry}
						retryLabel={retryLabel}
					/>
				) : null}
				{nonReadyState && events.length > 0 ? (
					<StateNotice
						state={nonReadyState}
						stateCopy={effectiveCopy}
						compact
						onRetry={onRetry}
						retryLabel={retryLabel}
					/>
				) : null}
				{events.length > 0 ? (
					<ol className="space-y-0" aria-label={ariaLabel ?? "Event timeline"}>
						{events.map((event, index) => {
							const tone = event.tone ?? "neutral";
							return (
								<li
									key={event.id}
									className="flex gap-3 motion-reduce:transition-none"
								>
									<div className="relative flex w-6 shrink-0 justify-center">
										<span
											aria-hidden="true"
											className={cn(
												"z-10 mt-0.5 flex size-6 items-center justify-center rounded-full border",
												TONE_RING[tone],
											)}
										>
											{event.icon ?? (
												<span
													className={cn("size-2 rounded-full", TONE_DOT[tone])}
												/>
											)}
										</span>
										{index < events.length - 1 ? (
											<span
												aria-hidden="true"
												className="absolute bottom-0 top-7 w-px bg-border/70"
											/>
										) : null}
									</div>
									<div className="min-w-0 flex-1 pb-5 last:pb-0">
										{renderEvent ? (
											renderEvent(event, index)
										) : (
											<>
												<div className="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
													<p className="min-w-0 text-sm font-medium">
														{event.title}
													</p>
													{event.timestamp ? (
														<time className="shrink-0 text-xs text-muted-foreground">
															{event.timestamp}
														</time>
													) : null}
												</div>
												{event.description ? (
													<p className="mt-1 text-xs leading-relaxed text-muted-foreground">
														{event.description}
													</p>
												) : null}
											</>
										)}
									</div>
								</li>
							);
						})}
					</ol>
				) : null}
			</CardContent>
		</Card>
	);
}
