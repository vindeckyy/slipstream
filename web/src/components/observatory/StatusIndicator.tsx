import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/utils";
import { StateNotice } from "./state";
import type {
	ObservatoryState,
	StateCopy,
	StatusIndicatorStatus,
} from "./types";

const STATUS_DOT: Record<StatusIndicatorStatus, string> = {
	healthy: "bg-success",
	degraded: "bg-warning",
	offline: "bg-destructive",
	unknown: "bg-muted-foreground/60",
};

const STATUS_TEXT: Record<StatusIndicatorStatus, string> = {
	healthy: "text-success",
	degraded: "text-warning",
	offline: "text-destructive",
	unknown: "text-muted-foreground",
};

export interface StatusIndicatorProps
	extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
	label: ReactNode;
	status?: StatusIndicatorStatus;
	statusLabel?: ReactNode;
	detail?: ReactNode;
	icon?: ReactNode;
	state?: ObservatoryState;
	stateCopy?: StateCopy;
	onRetry?: () => void;
	retryLabel?: ReactNode;
}

/** An accessible status dot and label for host, stream, or subsystem health. */
export function StatusIndicator({
	label,
	status = "unknown",
	statusLabel,
	detail,
	icon,
	state = "ready",
	stateCopy,
	onRetry,
	retryLabel,
	className,
	...props
}: StatusIndicatorProps) {
	const nonReadyState = state === "ready" ? null : state;

	return (
		<div
			className={cn("min-w-0 space-y-2", className)}
			aria-busy={state === "loading" || undefined}
			data-status={status}
			data-state={state}
			{...props}
		>
			<div className="flex min-w-0 items-center gap-2">
				<span
					aria-hidden="true"
					className={cn(
						"size-2.5 shrink-0 rounded-full",
						STATUS_DOT[status],
						status === "healthy" && "motion-reduce:animate-none animate-pulse",
					)}
				/>
				{icon ? (
					<span aria-hidden="true" className="shrink-0 text-muted-foreground">
						{icon}
					</span>
				) : null}
				<span className="min-w-0 truncate text-sm font-medium">{label}</span>
				<span className={cn("ml-auto shrink-0 text-xs", STATUS_TEXT[status])}>
					{statusLabel ?? status}
				</span>
			</div>
			{detail ? (
				<p className="pl-[18px] text-xs text-muted-foreground">{detail}</p>
			) : null}
			{nonReadyState ? (
				<StateNotice
					state={nonReadyState}
					stateCopy={stateCopy}
					compact
					onRetry={onRetry}
					retryLabel={retryLabel}
				/>
			) : null}
		</div>
	);
}
