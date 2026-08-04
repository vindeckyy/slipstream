import { CircleAlert, Info, RotateCcw } from "lucide-react";
import type { FC } from "react";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";

export type RestartOfferProps = {
	open: boolean;
	confirmOpen: boolean;
	pending: boolean;
	error: string | null;
	title: string;
	body: string;
	restartLabel: string;
	laterLabel: string;
	confirmTitle: string;
	confirmBody: string;
	confirmLabel: string;
	cancelLabel: string;
	pendingLabel: string;
	onRestart: () => void;
	onLater: () => void;
	onConfirmOpenChange: (open: boolean) => void;
	className?: string;
};

/** Post-save restart offer with an optional confirmation dialog. Never auto-restarts. */
export const RestartOffer: FC<RestartOfferProps> = ({
	open,
	confirmOpen,
	pending,
	error,
	title,
	body,
	restartLabel,
	laterLabel,
	confirmTitle,
	confirmBody,
	confirmLabel,
	cancelLabel,
	pendingLabel,
	onRestart,
	onLater,
	onConfirmOpenChange,
	className,
}) => {
	if (!open && !confirmOpen) return null;

	return (
		<>
			{open ? (
				<div
					role="status"
					aria-live="polite"
					data-testid="config-restart-offer"
					className={cn(
						"flex flex-col gap-3 rounded-xl border border-warning/40 bg-warning/10 px-4 py-3 sm:flex-row sm:items-center sm:justify-between",
						className,
					)}
				>
					<div className="flex min-w-0 items-start gap-3">
						{pending ? (
							<Spinner className="mt-0.5 size-4 shrink-0" />
						) : (
							<Info
								className="mt-0.5 size-4 shrink-0 text-[var(--warning)]"
								aria-hidden="true"
							/>
						)}
						<div className="min-w-0 space-y-0.5">
							<p className="text-sm font-medium">
								{pending ? pendingLabel : title}
							</p>
							<p className="text-xs leading-relaxed text-muted-foreground">
								{body}
							</p>
							{error ? (
								<p
									role="alert"
									className="flex items-start gap-1.5 text-xs text-destructive"
								>
									<CircleAlert className="mt-0.5 size-3.5 shrink-0" aria-hidden />
									{error}
								</p>
							) : null}
						</div>
					</div>
					<div className="flex w-full flex-col gap-2 sm:w-auto sm:flex-row">
						<Button
							type="button"
							variant="outline"
							disabled={pending}
							onClick={onLater}
							className="min-h-10 w-full sm:w-auto"
						>
							{laterLabel}
						</Button>
						<Button
							type="button"
							disabled={pending}
							onClick={() => onConfirmOpenChange(true)}
							className="min-h-10 w-full sm:w-auto"
							aria-busy={pending || undefined}
						>
							<RotateCcw className="size-4" aria-hidden="true" />
							{restartLabel}
						</Button>
					</div>
				</div>
			) : null}

			<Dialog open={confirmOpen} onOpenChange={onConfirmOpenChange}>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>{confirmTitle}</DialogTitle>
						<DialogDescription>{confirmBody}</DialogDescription>
						{error ? (
							<p
								role="alert"
								className="flex items-start gap-1.5 text-sm text-destructive"
							>
								<CircleAlert className="mt-0.5 size-4 shrink-0" aria-hidden />
								{error}
							</p>
						) : null}
					</DialogHeader>
					<DialogFooter className="gap-2 sm:gap-0">
						<Button
							type="button"
							variant="outline"
							disabled={pending}
							onClick={() => onConfirmOpenChange(false)}
						>
							{cancelLabel}
						</Button>
						<Button
							type="button"
							disabled={pending}
							onClick={onRestart}
							aria-busy={pending || undefined}
						>
							{pending ? pendingLabel : confirmLabel}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</>
	);
};
