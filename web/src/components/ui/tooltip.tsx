import { Tooltip as TooltipPrimitive } from "radix-ui";
import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

const TooltipProvider = ({
	delayDuration = 120,
	skipDelayDuration = 200,
	...props
}: ComponentProps<typeof TooltipPrimitive.Provider>) => (
	<TooltipPrimitive.Provider
		delayDuration={delayDuration}
		skipDelayDuration={skipDelayDuration}
		{...props}
	/>
);
TooltipProvider.displayName = "TooltipProvider";

const Tooltip = TooltipPrimitive.Root;

const TooltipTrigger = ({
	className,
	...props
}: ComponentProps<typeof TooltipPrimitive.Trigger>) => (
	<TooltipPrimitive.Trigger
		className={cn("outline-none", className)}
		{...props}
	/>
);
TooltipTrigger.displayName = "TooltipTrigger";

const TooltipContent = ({
	className,
	sideOffset = 6,
	...props
}: ComponentProps<typeof TooltipPrimitive.Content>) => (
	<TooltipPrimitive.Portal>
		<TooltipPrimitive.Content
			sideOffset={sideOffset}
			className={cn(
				"z-50 max-w-72 rounded-lg border border-border/80 bg-popover px-3 py-2 text-xs leading-relaxed text-popover-foreground shadow-md",
				"animate-in fade-in-0 zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95",
				"data-[side=bottom]:slide-in-from-top-1 data-[side=left]:slide-in-from-right-1 data-[side=right]:slide-in-from-left-1 data-[side=top]:slide-in-from-bottom-1",
				"motion-reduce:animate-none",
				className,
			)}
			{...props}
		/>
	</TooltipPrimitive.Portal>
);
TooltipContent.displayName = "TooltipContent";

export { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger };
