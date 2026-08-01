import { CircleHelp } from "lucide-react";
import type { OptionHTMLAttributes, ReactNode } from "react";
import { Badge } from "@/components/ui/badge";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

/** Compact ? button that shows a hover/focus description for a control. */
export function HelpTip({
	label,
	text,
	className,
}: {
	label: string;
	text: string;
	className?: string;
}) {
	return (
		<Tooltip>
			<TooltipTrigger asChild>
				<button
					type="button"
					className={cn(
						"inline-flex size-5 shrink-0 items-center justify-center rounded-full text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50",
						className,
					)}
					aria-label={`About ${label}`}
					title={text}
				>
					<CircleHelp className="size-3.5" aria-hidden="true" />
				</button>
			</TooltipTrigger>
			<TooltipContent side="top" align="start">
				{text}
			</TooltipContent>
		</Tooltip>
	);
}

/** Small "Recommended" chip plus the preferred value/text. */
export function RecommendedMark({
	value,
	className,
}: {
	value: ReactNode;
	className?: string;
}) {
	return (
		<div
			className={cn(
				"inline-flex max-w-full flex-wrap items-center gap-1.5 text-xs leading-relaxed text-muted-foreground",
				className,
			)}
		>
			<Badge variant="secondary" className="font-normal">
				Recommended
			</Badge>
			<span className="min-w-0">{value}</span>
		</div>
	);
}

/** Label row with optional help tip for menus, chips, and form fields. */
export function OptionLabel({
	label,
	help,
	recommended,
	htmlFor,
	className,
	labelClassName,
}: {
	label: ReactNode;
	help?: string;
	recommended?: ReactNode;
	htmlFor?: string;
	className?: string;
	labelClassName?: string;
}) {
	const textLabel = typeof label === "string" ? label : "option";
	return (
		<div className={cn("min-w-0 space-y-1", className)}>
			<div className="flex items-center gap-1.5">
				{htmlFor ? (
					<label
						htmlFor={htmlFor}
						className={cn(
							"text-sm font-medium leading-snug text-foreground",
							labelClassName,
						)}
					>
						{label}
					</label>
				) : (
					<span
						className={cn(
							"text-sm font-medium leading-snug text-foreground",
							labelClassName,
						)}
					>
						{label}
					</span>
				)}
				{help ? <HelpTip label={textLabel} text={help} /> : null}
			</div>
			{recommended ? <RecommendedMark value={recommended} /> : null}
		</div>
	);
}

/** `<option>` helper that can mark the preferred choice and carry a hover title. */
export function HelpOption({
	recommended = false,
	children,
	...props
}: OptionHTMLAttributes<HTMLOptionElement> & {
	recommended?: boolean;
}) {
	const label =
		typeof children === "string" || typeof children === "number"
			? recommended
				? `${children} (recommended)`
				: children
			: children;
	return <option {...props}>{label}</option>;
}
