import { AnimatedButton, buttonVariants } from "@unom/ui/button";
import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";
export type ButtonProps = ComponentProps<typeof AnimatedButton>;

export const Button = ({
	className,
	variant,
	...props
}: ButtonProps) => {
	const v = variant ?? "default";

	return (
		<AnimatedButton
			variant={variant}
			className={cn(
				"transition-[color,background-color,box-shadow,opacity,border-color,filter] duration-150 ease-out",
				"motion-reduce:transition-none",
				"focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/45",
				v === "default" &&
					"shadow-sm hover:brightness-[1.04] hover:shadow-md active:brightness-95 active:shadow-sm",
				v === "destructive" &&
					"shadow-sm hover:brightness-[1.04] hover:shadow-md active:brightness-95 active:shadow-sm",
				v === "success" &&
					"shadow-sm hover:brightness-[1.04] hover:shadow-md active:brightness-95 active:shadow-sm",
				v === "accent" &&
					"shadow-sm hover:brightness-[1.04] hover:shadow-md active:brightness-95 active:shadow-sm",
				v === "outline" &&
					"border-border bg-background/70 shadow-none hover:bg-muted hover:text-foreground dark:bg-input/25 dark:hover:bg-muted/70",
				v === "secondary" &&
					"bg-secondary text-secondary-foreground shadow-none hover:bg-secondary/80 active:bg-secondary/70",
				v === "ghost" &&
					"shadow-none hover:bg-muted hover:text-foreground dark:hover:bg-muted/80 active:bg-muted/90",
				v === "link" && "shadow-none hover:text-primary active:opacity-80",
				className,
			)}
			{...props}
		/>
	);
};
Button.displayName = "Button";

export { buttonVariants };
