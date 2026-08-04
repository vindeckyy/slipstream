import { Save as SaveIcon } from "lucide-react";
import type { FC } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/** Fixed mobile save bar that sits above the bottom nav + safe-area inset. */
export const DirtySaveBar: FC<{
	unsavedLabel: string;
	saveLabel: string;
	loadingLabel: string;
	discardLabel?: string;
	pending?: boolean;
	onSave: () => void;
	onDiscard?: () => void;
	className?: string;
}> = ({
	unsavedLabel,
	saveLabel,
	loadingLabel,
	discardLabel,
	pending = false,
	onSave,
	onDiscard,
	className,
}) => {
	return (
		<div
			className={cn(
				"fixed inset-x-0 z-40 border-t border-border bg-card/95 p-3 shadow-[0_-8px_24px_rgba(0,0,0,0.18)] backdrop-blur sm:hidden",
				className,
			)}
			style={{
				bottom: "calc(4.5rem + env(safe-area-inset-bottom, 0px))",
			}}
			data-testid="config-dirty-save-bar"
		>
			<div className="flex w-full flex-col gap-2">
				<Badge variant="warning" className="self-start">
					{unsavedLabel}
				</Badge>
				<div className="flex w-full gap-2">
					{onDiscard && discardLabel ? (
						<Button
							type="button"
							variant="outline"
							disabled={pending}
							onClick={onDiscard}
							className="min-h-11 flex-1"
						>
							{discardLabel}
						</Button>
					) : null}
					<Button
						type="button"
						disabled={pending}
						onClick={onSave}
						className="min-h-11 flex-1"
						aria-busy={pending || undefined}
					>
						<SaveIcon className="size-4" aria-hidden="true" />
						{pending ? loadingLabel : saveLabel}
					</Button>
				</div>
			</div>
		</div>
	);
};
