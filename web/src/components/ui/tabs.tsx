// The console's Tabs ARE @unom/ui's radix tabs, adapted to this app's tokens.
//
// @unom/ui styles inactive triggers `text-secondary` and the active one
// `data-[state=active]:text-main`. Those tokens come from @unom/ui's own palette;
// in the console's theme `text-secondary` lands on (near) the tab strip's own
// background, so every inactive tab renders as an invisible gap — the tab bar
// looks like a single lonely label with dead space beside it. Caught in a
// browser pass, not by types: the markup and the a11y roles are entirely correct.
//
// Surface treatment here follows M3 tonal containers: muted track, selected
// chip on the raised surface, clear hover/focus without restyling every call site.
import {
	Tabs as TabsBase,
	TabsContent as TabsContentBase,
	TabsList as TabsListBase,
	TabsTrigger as TabsTriggerBase,
} from "@unom/ui/tabs";
import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

const Tabs = ({
	className,
	...props
}: ComponentProps<typeof TabsBase>) => (
	<TabsBase className={cn("gap-4", className)} {...props} />
);
Tabs.displayName = "Tabs";

const TabsList = ({
	className,
	...props
}: ComponentProps<typeof TabsListBase>) => (
	<TabsListBase
		className={cn(
			"h-10 gap-0.5 rounded-lg border border-border/70 bg-muted/90 p-1 text-muted-foreground shadow-inner",
			"transition-[background-color,border-color] duration-150 ease-out",
			"motion-reduce:transition-none",
			className,
		)}
		{...props}
	/>
);
TabsList.displayName = "TabsList";

const TabsTrigger = ({
	className,
	...props
}: ComponentProps<typeof TabsTriggerBase>) => (
	<TabsTriggerBase
		className={cn(
			"text-muted-foreground hover:bg-background/50 hover:text-foreground",
			"data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm",
			"data-[state=active]:ring-1 data-[state=active]:ring-border/80",
			"transition-[color,background-color,box-shadow,ring-color] duration-150 ease-out",
			"motion-reduce:transition-none",
			"focus-visible:ring-[3px] focus-visible:ring-ring/45",
			className,
		)}
		{...props}
	/>
);
TabsTrigger.displayName = "TabsTrigger";

const TabsContent = ({
	className,
	...props
}: ComponentProps<typeof TabsContentBase>) => (
	<TabsContentBase
		className={cn(
			"mt-1 outline-none focus-visible:ring-[3px] focus-visible:ring-ring/45",
			"motion-reduce:transition-none",
			className,
		)}
		{...props}
	/>
);
TabsContent.displayName = "TabsContent";

export { Tabs, TabsContent, TabsList, TabsTrigger };
