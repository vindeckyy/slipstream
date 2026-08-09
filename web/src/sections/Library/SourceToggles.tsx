import { useQueryClient } from "@tanstack/react-query";
import { toast } from "@unom/ui/toast";
import { Check } from "lucide-react";
import type { FC } from "react";
import {
	getGetLibraryQueryKey,
	getListLibraryScannersQueryKey,
	useListLibraryScanners,
	useSetLibraryScanner,
} from "@/api/gen/library/library";
import type { ScannerInfo } from "@/api/gen/model/scannerInfo";
import { HelpTip, RecommendedMark } from "@/components/option-help";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { m } from "@/paraglide/messages";

/** Hover copy + optional recommendation for one scanner chip. */
function scannerGuidance(scanner: ScannerInfo): {
	help: string;
	recommended?: string;
} {
	switch (scanner.id) {
		case "steam":
			return {
				help: m.library_source_help_steam(),
				recommended: m.library_source_recommended_steam(),
			};
		case "lutris":
			return { help: m.library_source_help_lutris() };
		case "heroic":
			return { help: m.library_source_help_heroic() };
		case "epic":
			return { help: m.library_source_help_epic() };
		case "gog":
			return { help: m.library_source_help_gog() };
		default:
			return {
				help: m.library_source_help_generic({ label: scanner.label }),
			};
	}
}

/**
 * Container: the game-source (library scanner) toggles — owns the scanner query and the toggle
 * mutation. The host reports only the scanners available on the current host, so whatever arrives
 * is renderable as-is.
 * Rendered only once the list is loaded: this is a secondary control, and when the API is down
 * the grid's own QueryState already tells the story — no second error banner.
 */
export const SourceTogglesSection: FC = () => {
	const qc = useQueryClient();
	const scanners = useListLibraryScanners();
	const toggle = useSetLibraryScanner();

	const onToggle = async (scanner: ScannerInfo) => {
		try {
			// The PUT answers with the full updated list — seed the query cache with it directly,
			// then refetch the library so the grid reflects the new source set.
			const list = await toggle.mutateAsync({
				id: scanner.id,
				data: { enabled: !scanner.enabled },
			});
			qc.setQueryData(getListLibraryScannersQueryKey(), list);
			await qc.invalidateQueries({ queryKey: getGetLibraryQueryKey() });
		} catch {
			toast.error(m.library_sources_failed());
		}
	};

	if (!scanners.data) return null;
	return (
		<SourceToggles
			scanners={scanners.data}
			busyId={toggle.isPending ? (toggle.variables?.id ?? null) : null}
			onToggle={onToggle}
		/>
	);
};

/** The sources card: one pressed/unpressed chip per scanner (pressed = the host scans it). */
export const SourceToggles: FC<{
	scanners: ScannerInfo[];
	/** Scanner id whose toggle is in flight, or null — only that chip disables. */
	busyId: string | null;
	onToggle: (scanner: ScannerInfo) => void;
}> = ({ scanners, busyId, onToggle }) => (
	<Card>
		<CardHeader className="pb-3">
			<CardTitle className="text-base">{m.library_sources_title()}</CardTitle>
			<CardDescription>{m.library_sources_help()}</CardDescription>
		</CardHeader>
		<CardContent>
			<fieldset
				aria-label={m.library_sources_title()}
				className="m-0 flex min-w-0 flex-wrap gap-3 rounded-lg border border-border/70 bg-muted/30 p-3"
			>
				{scanners.map((scanner) => {
					const { help, recommended } = scannerGuidance(scanner);
					return (
						<div
							key={scanner.id}
							className="inline-flex max-w-full flex-col gap-1"
						>
							<div className="flex items-center gap-1">
								<Button
									size="sm"
									variant={scanner.enabled ? "default" : "outline"}
									aria-pressed={scanner.enabled}
									disabled={busyId === scanner.id}
									onClick={() => onToggle(scanner)}
								>
									{scanner.enabled && <Check className="size-4" />}
									{scanner.label}
								</Button>
								<HelpTip label={scanner.label} text={help} />
							</div>
							{recommended ? <RecommendedMark value={recommended} /> : null}
						</div>
					);
				})}
			</fieldset>
		</CardContent>
	</Card>
);
