import { CircleHelp } from "lucide-react";
import type { OptionHTMLAttributes, ReactNode } from "react";
import { useId } from "react";
import { Badge } from "@/components/ui/badge";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages";

/** Compact ? button that shows a hover/focus description for a control.
 *
 * Self-contained: wraps its own `TooltipProvider` so a HelpTip renders correctly in any
 * context (a page, a card, a story) without an ancestor provider. Nesting providers is
 * safe — the app shell's provider still governs the rest of the tree. */
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
		<TooltipProvider>
			<Tooltip>
				<TooltipTrigger asChild>
					<button
						type="button"
						className={cn(
							"inline-flex size-5 shrink-0 items-center justify-center rounded-full text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50",
							className,
						)}
						aria-label={m.config_help_about({ label })}
						title={text}
					>
						<CircleHelp className="size-3.5" aria-hidden="true" />
					</button>
				</TooltipTrigger>
				<TooltipContent side="top" align="start">
					{text}
				</TooltipContent>
			</Tooltip>
		</TooltipProvider>
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
				{m.config_recommended_badge()}
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
	recommendedSuffix = m.config_recommended_option_suffix(),
	children,
	...props
}: OptionHTMLAttributes<HTMLOptionElement> & {
	recommended?: boolean;
	recommendedSuffix?: string;
}) {
	const label =
		typeof children === "string" || typeof children === "number"
			? recommended
				? `${children} ${recommendedSuffix}`
				: children
			: children;
	return <option {...props}>{label}</option>;
}

/** When a change to a setting takes effect. */
export type SettingEffect =
	| "immediate"
	| "next-session"
	| "next-connect"
	| "restart-required";

/** Localized label for one effect timing. */
export function settingEffectLabel(effect: SettingEffect): string {
	switch (effect) {
		case "immediate":
			return m.setting_effect_immediate();
		case "next-session":
			return m.setting_effect_next_session();
		case "next-connect":
			return m.setting_effect_next_connect();
		case "restart-required":
			return m.setting_effect_restart_required();
	}
}

/** Small visible badge showing when a setting takes effect. */
export function SettingEffectBadge({
	effect,
	className,
}: {
	effect: SettingEffect;
	className?: string;
}) {
	return (
		<Badge variant="outline" className={cn("font-normal", className)}>
			{settingEffectLabel(effect)}
		</Badge>
	);
}

/** Accessibility attributes the control should spread onto the rendered input/select. */
export type SettingControlA11y = {
	"aria-describedby"?: string;
	"aria-invalid"?: true;
};

export type SettingFieldProps = {
	id?: string;
	label: string;
	hint?: ReactNode;
	help?: string;
	recommended?: ReactNode;
	effect?: SettingEffect;
	error?: string;
	warning?: string;
	disabledReason?: string;
	group?: boolean;
	children: (a11y: SettingControlA11y) => ReactNode;
	className?: string;
};

/** Guidance id set shared by `SettingField` and its rendered helper texts. */
export function settingFieldIds(prefix: string) {
	return {
		label: `${prefix}-label`,
		hint: `${prefix}-hint`,
		effect: `${prefix}-effect`,
		error: `${prefix}-error`,
		warning: `${prefix}-warning`,
		disabled: `${prefix}-disabled`,
	};
}

/**
 * One labeled setting: label, optional inline hint, help tip, recommendation,
 * visible effect timing, and inline error/warning/disabled-reason text.
 *
 * `children` receives the accessibility attributes the control must spread, so
 * errors and effect text are announced instead of living only in `title`.
 * Without `group`, `id` pairs the label with a single control; with `group`,
 * a `<fieldset>`/`<legend>` wraps a set of controls (button groups, toggles).
 */
export function SettingField({
	id,
	label,
	hint,
	help,
	recommended,
	effect,
	error,
	warning,
	disabledReason,
	group = false,
	children,
	className,
}: SettingFieldProps) {
	const base = useId();
	const controlId = id ?? base;
	const ids = settingFieldIds(controlId);

	const describedBy = [
		hint ? ids.hint : null,
		effect ? ids.effect : null,
		warning ? ids.warning : null,
		error ? ids.error : null,
		disabledReason ? ids.disabled : null,
	]
		.filter(Boolean)
		.join(" ") || undefined;

	const a11y: SettingControlA11y = {
		"aria-describedby": describedBy,
		...(error ? { "aria-invalid": true } : {}),
	};

	const guidance = (
		<>
			{hint ? (
				<p id={ids.hint} className="text-xs text-muted-foreground">
					{hint}
				</p>
			) : null}
			{effect ? <SettingEffectBadge effect={effect} /> : null}
			{disabledReason ? (
				<p id={ids.disabled} className="text-xs text-muted-foreground">
					{disabledReason}
				</p>
			) : null}
			{warning ? (
				<p
					id={ids.warning}
					role="status"
					className="rounded-md border border-warning/40 bg-warning/10 px-2.5 py-1.5 text-xs text-[var(--warning)]"
				>
					{warning}
				</p>
			) : null}
			{error ? (
				<p
					id={ids.error}
					role="alert"
					className="rounded-md border border-destructive/40 bg-destructive/5 px-2.5 py-1.5 text-xs text-destructive"
				>
					{error}
				</p>
			) : null}
		</>
	);

	if (group) {
		return (
			<fieldset className={cn("min-w-0 space-y-1.5", className)}>
				<legend
					id={ids.label}
					className="flex items-center gap-1.5 text-sm font-medium leading-snug text-foreground"
				>
					{label}
					{help ? <HelpTip label={label} text={help} /> : null}
				</legend>
				{recommended ? <RecommendedMark value={recommended} /> : null}
				{guidance}
				{children(a11y)}
			</fieldset>
		);
	}

	return (
		<div className={cn("min-w-0 space-y-1.5", className)}>
			<OptionLabel
				label={label}
				help={help}
				recommended={recommended}
				htmlFor={controlId}
			/>
			{guidance}
			{children(a11y)}
		</div>
	);
}
