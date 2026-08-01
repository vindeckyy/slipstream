import type { HTMLAttributes, ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages";
import type { NonReadyObservatoryState, StateCopy } from "./types";

const DEFAULT_STATE_COPY: Record<NonReadyObservatoryState, () => ReactNode> = {
	loading: () => m.common_loading(),
	stale: () => "Showing cached data",
	error: () => m.common_error(),
	empty: () => "No data available",
};

const STATE_ACCENT: Record<NonReadyObservatoryState, string> = {
	loading: "border-border/70 bg-muted/20 text-muted-foreground",
	stale: "border-warning/40 bg-warning/5 text-foreground",
	error: "border-destructive/40 bg-destructive/5 text-destructive",
	empty: "border-dashed border-border/70 bg-muted/15 text-muted-foreground",
};

export function stateLabel(
	state: NonReadyObservatoryState,
	copy?: StateCopy,
): ReactNode {
	return copy?.[state] ?? DEFAULT_STATE_COPY[state]();
}

export interface StateNoticeProps
	extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
	state: NonReadyObservatoryState;
	title?: ReactNode;
	description?: ReactNode;
	stateCopy?: StateCopy;
	onRetry?: () => void;
	retryLabel?: ReactNode;
	compact?: boolean;
}

/**
 * Small, query-free state surface shared by observatory primitives.
 *
 * A notice never owns fetching. Callers provide the state and an optional retry callback, which
 * keeps the component deterministic in Storybook and useful with any data source.
 */
export function StateNotice({
	state,
	title,
	description,
	stateCopy,
	onRetry,
	retryLabel = m.common_retry(),
	compact = false,
	className,
	children,
	...props
}: StateNoticeProps) {
	const message = title ?? stateLabel(state, stateCopy);
	const role = state === "error" ? "alert" : "status";

	return (
		<div
			role={role}
			aria-live={state === "error" ? "assertive" : "polite"}
			className={cn(
				"flex items-center justify-between gap-3 rounded-lg border text-sm",
				compact ? "px-3 py-2" : "min-h-24 px-4 py-5",
				STATE_ACCENT[state],
				"motion-reduce:transition-none",
				className,
			)}
			{...props}
		>
			<div className="flex min-w-0 items-start gap-2.5">
				{state === "loading" ? (
					<Spinner className="mt-0.5 size-4 shrink-0" />
				) : (
					<span
						aria-hidden="true"
						className={cn(
							"mt-1.5 size-1.5 shrink-0 rounded-full bg-current",
							state === "stale" && "text-[var(--warning)]",
							state === "error" && "text-destructive",
						)}
					/>
				)}
				<div className="min-w-0">
					<p className="font-medium">{message}</p>
					{description ? (
						<p className="mt-0.5 text-xs text-muted-foreground">
							{description}
						</p>
					) : null}
					{children}
				</div>
			</div>
			{state === "error" && onRetry ? (
				<Button
					type="button"
					variant="outline"
					size="sm"
					className="shrink-0"
					onClick={onRetry}
				>
					{retryLabel}
				</Button>
			) : null}
		</div>
	);
}
