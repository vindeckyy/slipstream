import { AlertTriangle, CheckCircle2, Info, X } from "lucide-react";
import type { HTMLAttributes, ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { stateLabel } from "./state";
import type { AlertVariant, ObservatoryState, StateCopy } from "./types";

const VARIANT_ICON: Record<AlertVariant, ReactNode> = {
	info: <Info className="size-4" />,
	success: <CheckCircle2 className="size-4" />,
	warning: <AlertTriangle className="size-4" />,
	error: <AlertTriangle className="size-4" />,
};

const VARIANT_CLASS: Record<AlertVariant, string> = {
	info: "border-sky-500/30 bg-sky-500/5 text-sky-800 dark:text-sky-200",
	success:
		"border-emerald-500/30 bg-emerald-500/5 text-emerald-800 dark:text-emerald-200",
	warning: "border-warning/40 bg-warning/5 text-foreground",
	error: "border-destructive/40 bg-destructive/5 text-destructive",
};

export interface AlertStripProps
	extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
	title?: ReactNode;
	children: ReactNode;
	variant?: AlertVariant;
	icon?: ReactNode;
	state?: ObservatoryState;
	stateCopy?: StateCopy;
	onRetry?: () => void;
	retryLabel?: ReactNode;
	onDismiss?: () => void;
	dismissLabel?: string;
}

/** A compact, dismissible message surface for operator attention and recovery actions. */
export function AlertStrip({
	title,
	children,
	variant = "info",
	icon,
	state = "ready",
	stateCopy,
	onRetry,
	retryLabel,
	onDismiss,
	dismissLabel = "Dismiss",
	className,
	...props
}: AlertStripProps) {
	const nonReadyState = state === "ready" ? null : state;
	const effectiveVariant = state === "error" ? "error" : variant;
	const stateMessage = nonReadyState
		? stateLabel(nonReadyState, stateCopy)
		: null;

	return (
		<div
			role={effectiveVariant === "error" ? "alert" : "status"}
			aria-live={effectiveVariant === "error" ? "assertive" : "polite"}
			aria-busy={state === "loading" || undefined}
			className={cn(
				"flex items-start gap-3 rounded-lg border px-3.5 py-3 text-sm",
				VARIANT_CLASS[effectiveVariant],
				"motion-reduce:transition-none",
				className,
			)}
			{...props}
		>
			<span aria-hidden="true" className="mt-0.5 shrink-0">
				{icon ?? VARIANT_ICON[effectiveVariant]}
			</span>
			<div className="min-w-0 flex-1">
				{title ? <p className="font-medium">{title}</p> : null}
				<div className={cn(title && "mt-0.5", "text-sm/relaxed")}>
					{children}
				</div>
				{stateMessage ? (
					<p className="mt-2 text-xs text-muted-foreground">{stateMessage}</p>
				) : null}
				{state === "error" && onRetry ? (
					<Button
						type="button"
						variant="outline"
						size="sm"
						className="mt-2"
						onClick={onRetry}
					>
						{retryLabel ?? "Retry"}
					</Button>
				) : null}
			</div>
			{onDismiss ? (
				<Button
					type="button"
					variant="ghost"
					size="icon"
					className="size-7 shrink-0 text-current hover:bg-black/5 dark:hover:bg-white/10"
					aria-label={dismissLabel}
					title={dismissLabel}
					onClick={onDismiss}
				>
					<X className="size-4" />
				</Button>
			) : null}
		</div>
	);
}
