import * as React from "react";
import { cn } from "@/lib/utils";

export type SwitchProps = Omit<
	React.ButtonHTMLAttributes<HTMLButtonElement>,
	"onChange"
> & {
	checked: boolean;
	onCheckedChange: (checked: boolean) => void;
};

/** Accessible toggle for Sunshine-style config forms. */
export function Switch({
	checked,
	onCheckedChange,
	className,
	disabled,
	id,
	...rest
}: SwitchProps) {
	return (
		<button
			type="button"
			role="switch"
			id={id}
			aria-checked={checked}
			disabled={disabled}
			onClick={() => onCheckedChange(!checked)}
			className={cn(
				"relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border transition-colors",
				checked ? "border-primary bg-primary" : "border-input bg-muted",
				disabled && "cursor-not-allowed opacity-50",
				className,
			)}
			{...rest}
		>
			<span
				aria-hidden
				className={cn(
					"pointer-events-none block size-5 rounded-full bg-background shadow transition-transform",
					checked ? "translate-x-[1.35rem]" : "translate-x-0.5",
				)}
			/>
		</button>
	);
}
