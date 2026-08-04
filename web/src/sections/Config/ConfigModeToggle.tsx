import type { FC } from "react";
import { cn } from "@/lib/utils";

export type ConfigMode = "recommended" | "all";

export const ConfigModeToggle: FC<{
	mode: ConfigMode;
	label: string;
	recommendedLabel: string;
	allLabel: string;
	onChange: (mode: ConfigMode) => void;
	className?: string;
}> = ({
	mode,
	label,
	recommendedLabel,
	allLabel,
	onChange,
	className,
}) => {
	return (
		<div
			className={cn("flex flex-col gap-2", className)}
			data-testid="config-mode-toggle"
		>
			<p className="text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground">
				{label}
			</p>
			<div
				role="tablist"
				aria-label={label}
				className="inline-flex w-full rounded-xl border border-border/70 bg-muted/70 p-1 sm:w-auto"
			>
				<button
					type="button"
					role="tab"
					aria-selected={mode === "recommended"}
					data-testid="config-mode-recommended"
					className={cn(
						"min-h-10 flex-1 rounded-lg px-4 text-sm font-medium transition-colors sm:flex-none",
						mode === "recommended"
							? "bg-background text-foreground shadow-sm"
							: "text-muted-foreground hover:text-foreground",
					)}
					onClick={() => onChange("recommended")}
				>
					{recommendedLabel}
				</button>
				<button
					type="button"
					role="tab"
					aria-selected={mode === "all"}
					data-testid="config-mode-all"
					className={cn(
						"min-h-10 flex-1 rounded-lg px-4 text-sm font-medium transition-colors sm:flex-none",
						mode === "all"
							? "bg-background text-foreground shadow-sm"
							: "text-muted-foreground hover:text-foreground",
					)}
					onClick={() => onChange("all")}
				>
					{allLabel}
				</button>
			</div>
		</div>
	);
};
