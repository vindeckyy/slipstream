import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/utils";
import { StateNotice } from "./state";
import type { ObservatoryState, StateCopy } from "./types";

export interface PageHeaderProps
	extends Omit<HTMLAttributes<HTMLElement>, "title"> {
	title: ReactNode;
	description?: ReactNode;
	eyebrow?: ReactNode;
	meta?: ReactNode;
	actions?: ReactNode;
	state?: ObservatoryState;
	stateCopy?: StateCopy;
	onRetry?: () => void;
	retryLabel?: ReactNode;
}

/** A responsive page heading with optional operator actions and data-state feedback. */
export function PageHeader({
	title,
	description,
	eyebrow,
	meta,
	actions,
	state = "ready",
	stateCopy,
	onRetry,
	retryLabel,
	className,
	...props
}: PageHeaderProps) {
	const nonReadyState = state === "ready" ? null : state;

	return (
		<header
			className={cn(
				"space-y-3 border-b border-border/60 pb-4 sm:space-y-4",
				className,
			)}
			aria-busy={state === "loading" || undefined}
			data-state={state}
			{...props}
		>
			<div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
				<div className="min-w-0 space-y-1.5">
					{eyebrow ? (
						<p className="text-[11px] font-medium uppercase tracking-[0.08em] text-primary">
							{eyebrow}
						</p>
					) : null}
					<h1 className="text-2xl font-semibold tracking-tight sm:text-3xl">
						{title}
					</h1>
					{description ? (
						<p className="max-w-prose text-sm leading-relaxed text-muted-foreground">
							{description}
						</p>
					) : null}
					{meta ? (
						<div className="text-xs text-muted-foreground">{meta}</div>
					) : null}
				</div>
				{actions ? (
					<div className="flex shrink-0 flex-wrap items-center gap-2">
						{actions}
					</div>
				) : null}
			</div>
			{nonReadyState ? (
				<StateNotice
					state={nonReadyState}
					stateCopy={stateCopy}
					compact
					onRetry={onRetry}
					retryLabel={retryLabel}
				/>
			) : null}
		</header>
	);
}
