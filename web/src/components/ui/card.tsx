import { AnimatedCard } from "@unom/ui/card";
import type { ComponentProps } from "react";
import * as React from "react";
import { cn } from "@/lib/utils";

// The console's Card IS @unom/ui's animated card — tonal surface (`bg-neutral` /
// `--card`) with on-mount motion + material gloss (via UnomProviders). We keep
// the composed shadcn-style sub-component API (Header/Title/Description/Content/
// Footer own their own padding), so the card defaults to `padding={false}` to
// avoid doubling it. Surface elevation is M3-style: hairline border + soft
// tonal shadow instead of a heavy brand ring.
type CardProps = ComponentProps<typeof AnimatedCard>;

const Card = ({
	className,
	padding = false,
	interactive,
	children,
	...props
}: CardProps) => (
	<AnimatedCard
		padding={padding}
		interactive={interactive}
		className={cn(
			"ring-1 ring-border/80 shadow-sm",
			"transition-[box-shadow,background-color,ring-color] duration-200 ease-out",
			"motion-reduce:transition-none",
			interactive &&
				"hover:bg-muted/50 hover:shadow-md hover:ring-border dark:hover:bg-muted/40",
			className,
		)}
		{...props}
	>
		{children}
	</AnimatedCard>
);
Card.displayName = "Card";

const CardHeader = React.forwardRef<
	HTMLDivElement,
	React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
	<div
		ref={ref}
		className={cn(
			"flex flex-col gap-1.5 px-4 pb-3 pt-4 sm:px-6 sm:pb-4 sm:pt-6",
			className,
		)}
		{...props}
	/>
));
CardHeader.displayName = "CardHeader";

const CardTitle = React.forwardRef<
	HTMLDivElement,
	React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
	<div
		ref={ref}
		className={cn(
			"text-base font-semibold leading-snug tracking-tight text-card-foreground",
			className,
		)}
		{...props}
	/>
));
CardTitle.displayName = "CardTitle";

const CardDescription = React.forwardRef<
	HTMLDivElement,
	React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
	<div
		ref={ref}
		className={cn("text-sm leading-relaxed text-muted-foreground", className)}
		{...props}
	/>
));
CardDescription.displayName = "CardDescription";

/**
 * Card body. Pass `flush` for content that should meet the card's edges — a full-bleed table, most
 * commonly — instead of trying to cancel the padding from the outside.
 *
 * `className="p-0"` does NOT work for that: tailwind-merge only resolves conflicts *within the same
 * variant*, so `p-0` cancels `p-4` but leaves `sm:p-6` standing, and the padding silently returns at
 * ≥640px. Every call site that tried it ended up with a doubled inset once a `CardHeader` (which
 * brings its own `sm:p-6`) was nested inside — visible as one card whose title sits 24px further in
 * than its neighbours'.
 */
const CardContent = React.forwardRef<
	HTMLDivElement,
	React.HTMLAttributes<HTMLDivElement> & { flush?: boolean }
>(({ className, flush = false, ...props }, ref) => (
	<div
		ref={ref}
		className={cn(
			!flush && "px-4 pb-4 pt-0 sm:px-6 sm:pb-6 sm:pt-0",
			className,
		)}
		{...props}
	/>
));
CardContent.displayName = "CardContent";

const CardFooter = React.forwardRef<
	HTMLDivElement,
	React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
	<div
		ref={ref}
		className={cn(
			"flex items-center gap-2 px-4 pb-4 pt-0 sm:px-6 sm:pb-6 sm:pt-0",
			className,
		)}
		{...props}
	/>
));
CardFooter.displayName = "CardFooter";

export {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
};
