import type { ComponentProps, ReactNode } from "react";
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { StateNotice } from "./state";
import type { ObservatoryState, StateCopy } from "./types";

export interface MetricCardProps
	extends Omit<ComponentProps<typeof Card>, "title" | "children"> {
	title: ReactNode;
	value?: ReactNode;
	unit?: ReactNode;
	description?: ReactNode;
	trend?: ReactNode;
	icon?: ReactNode;
	footer?: ReactNode;
	state?: ObservatoryState;
	stateCopy?: StateCopy;
	onRetry?: () => void;
	retryLabel?: ReactNode;
}

/**
 * A compact metric surface with a stable layout across data states.
 *
 * A value can remain visible while `stale` or `error` is reported, which lets an operator keep
 * reading the last known measurement during a failed refresh.
 */
export function MetricCard({
	title,
	value,
	unit,
	description,
	trend,
	icon,
	footer,
	state = "ready",
	stateCopy,
	onRetry,
	retryLabel,
	className,
	...props
}: MetricCardProps) {
	const hasValue = value !== undefined && value !== null;
	const showValue = hasValue && state !== "empty";
	const showNotice = state !== "ready";
	const noticeOnly = !showValue && showNotice;

	return (
		<Card
			className={cn(
				"min-w-0",
				state === "stale" && "ring-warning/30",
				state === "error" && "ring-destructive/30",
				className,
			)}
			aria-busy={state === "loading" || undefined}
			data-state={state}
			{...props}
		>
			<CardHeader className="flex-row items-start justify-between gap-3 space-y-0">
				<div className="min-w-0 space-y-1">
					<CardTitle className="truncate text-sm font-medium text-muted-foreground">
						{title}
					</CardTitle>
					{description ? (
						<CardDescription className="line-clamp-2">
							{description}
						</CardDescription>
					) : null}
				</div>
				{icon ? (
					<span
						aria-hidden="true"
						className="flex size-8 shrink-0 items-center justify-center rounded-md border border-border/60 bg-muted/40 text-muted-foreground shadow-[inset_0_-1px_0_rgba(0,0,0,0.08)]"
					>
						{icon}
					</span>
				) : null}
			</CardHeader>

			<CardContent className="space-y-3">
				{noticeOnly ? (
					<StateNotice
						state={state as Exclude<ObservatoryState, "ready">}
						stateCopy={stateCopy}
						onRetry={onRetry}
						retryLabel={retryLabel}
					/>
				) : (
					<>
						{showValue ? (
							<div className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
								{/* Signal readout: mono tabular numerals, the broadcast meter register. */}
								<span className="font-mono text-3xl font-medium tracking-tight tabular-nums">
									{value}
								</span>
								{unit ? (
									<span className="text-sm text-muted-foreground">{unit}</span>
								) : null}
								{trend ? (
									<span className="ml-auto text-xs text-muted-foreground">
										{trend}
									</span>
								) : null}
							</div>
						) : (
							<div
								aria-hidden="true"
								className="h-9 w-24 rounded-md bg-muted/60 motion-reduce:animate-none animate-pulse"
							/>
						)}
						{showNotice ? (
							<StateNotice
								state={state as Exclude<ObservatoryState, "ready">}
								stateCopy={stateCopy}
								compact
								onRetry={onRetry}
								retryLabel={retryLabel}
							/>
						) : null}
					</>
				)}
			</CardContent>

			{footer ? <CardFooter>{footer}</CardFooter> : null}
		</Card>
	);
}
