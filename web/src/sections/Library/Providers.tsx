import { useQueryClient } from "@tanstack/react-query";
import { toast } from "@unom/ui/toast";
import { Trash2 } from "lucide-react";
import type { FC } from "react";
import {
	getGetLibraryQueryKey,
	useDeleteProviderEntries,
} from "@/api/gen/library/library";
import type { GameEntry } from "@/api/gen/model/gameEntry";
import { HelpTip, RecommendedMark } from "@/components/option-help";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { apiErrorMessage } from "@/lib/errors";
import { m } from "@/paraglide/messages";

/**
 * Provider-owned entries: who put them there, and how to get rid of them.
 *
 * A plugin can sync entries into the library (RFC §8) and they are then refused to hand-edit or
 * delete individually — the host answers 409 and points at the provider's own reconcile. Which is
 * correct, and completely opaque if the plugin is gone: uninstalling it leaves its games in the
 * library with no console-side way to remove them. `DELETE /library/provider/{provider}` is the
 * documented clean-uninstall path and nothing called it.
 *
 * Renders nothing when no entry carries a provider, so an ordinary library sees no extra chrome.
 */
export const ProvidersCard: FC<{
	entries: GameEntry[];
	/** The provider currently filtered to, or null for "everything". */
	active: string | null;
	onFilter: (provider: string | null) => void;
}> = ({ entries, active, onFilter }) => {
	const qc = useQueryClient();
	const purge = useDeleteProviderEntries();

	// Count per provider, in first-seen order — the list is small and operator-facing.
	const counts = new Map<string, number>();
	for (const e of entries) {
		if (e.provider) counts.set(e.provider, (counts.get(e.provider) ?? 0) + 1);
	}
	if (counts.size === 0) return null;

	const onPurge = async (provider: string, count: number) => {
		if (!confirm(m.library_provider_purge_confirm({ provider, count }))) return;
		try {
			await purge.mutateAsync({ provider });
			// The host emits `library.changed`, but don't wait for the round trip to redraw.
			qc.invalidateQueries({ queryKey: getGetLibraryQueryKey() });
			if (active === provider) onFilter(null);
			toast.success(m.library_provider_purged({ provider }));
		} catch (e) {
			toast.error(apiErrorMessage(e) ?? m.library_provider_purge_failed());
		}
	};

	return (
		<Card>
			<CardHeader className="pb-3">
				<CardTitle className="text-base">
					{m.library_providers_title()}
				</CardTitle>
				<CardDescription>{m.library_providers_help()}</CardDescription>
			</CardHeader>
			<CardContent className="space-y-2">
				{[...counts.entries()].map(([provider, count]) => (
					<div
						key={provider}
						className="flex flex-col gap-3 rounded-lg border border-border/70 bg-muted/30 p-3 sm:flex-row sm:items-start"
					>
						<div className="flex min-w-0 flex-wrap items-center gap-2">
							<span className="font-medium tracking-tight">{provider}</span>
							<Badge variant="secondary">
								{m.library_provider_count({ count })}
							</Badge>
						</div>
						<div className="flex flex-wrap gap-3 sm:ml-auto">
							<div className="inline-flex max-w-full flex-col gap-1">
								<div className="flex items-center gap-1">
									<Button
										size="sm"
										variant={active === provider ? "default" : "outline"}
										aria-pressed={active === provider}
										onClick={() =>
											onFilter(active === provider ? null : provider)
										}
									>
										{active === provider
											? m.library_provider_show_all()
											: m.library_provider_filter()}
									</Button>
									<HelpTip
										label={m.library_provider_filter()}
										text={m.library_provider_filter_help()}
									/>
								</div>
								<RecommendedMark
									value={m.library_provider_filter_recommended()}
								/>
							</div>
							<div className="inline-flex max-w-full flex-col gap-1">
								<div className="flex items-center gap-1">
									<Button
										size="sm"
										variant="outline"
										disabled={purge.isPending}
										aria-label={m.library_provider_purge()}
										onClick={() => onPurge(provider, count)}
									>
										<Trash2 className="size-4 text-destructive" />
									</Button>
									<HelpTip
										label={m.library_provider_purge()}
										text={m.library_provider_purge_help()}
									/>
								</div>
								<RecommendedMark
									value={m.library_provider_purge_recommended()}
								/>
							</div>
						</div>
					</div>
				))}
			</CardContent>
		</Card>
	);
};
