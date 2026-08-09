import {
	AlertTriangle,
	Ban,
	Check,
	Download,
	PackageSearch,
	Search,
} from "lucide-react";
import { type FC, useMemo, useState } from "react";
import { pluginIcon } from "@/api/plugins";
import { type StoreEntry, useStoreCatalog } from "@/api/store";
import { HelpTip } from "@/components/option-help";
import { QueryState } from "@/components/query-state";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages";
import { RunnerBanner } from "./Runner";
import { SourceChip, TierBadge } from "./TierBadge";

/** Case-insensitive substring match across the fields an operator would actually search by. */
function matches(entry: StoreEntry, needle: string): boolean {
	if (!needle) return true;
	const q = needle.toLowerCase();
	return [entry.title, entry.description, entry.pkg, entry.author].some((f) =>
		f.toLowerCase().includes(q),
	);
}

/**
 * Container: the catalog. Owns the catalog query plus the local search/source filter; installing is
 * escalated to the parent, which owns the tier-appropriate dialog and the resulting job — so this
 * subsection never installs anything itself.
 */
export const BrowseTab: FC<{
	onInstall: (entry: StoreEntry) => void;
	onInstallSpec: () => void;
}> = ({ onInstall, onInstallSpec }) => {
	const catalog = useStoreCatalog();
	// Sources that could not be fetched — the difference between "this host has no plugins" and
	// "the console could not find out".
	const failedSources = (catalog.data?.sources ?? []).filter(
		(src) => src.error || src.stale,
	);
	const [query, setQuery] = useState("");
	const [source, setSource] = useState<string | null>(null);

	const entries = catalog.data?.plugins ?? [];
	const sources = catalog.data?.sources ?? [];
	const shown = useMemo(
		() =>
			entries.filter(
				(e) => (source === null || e.source === source) && matches(e, query),
			),
		[entries, source, query],
	);

	return (
		<div className="flex flex-col gap-card">
			<RunnerBanner />

			<div className="flex flex-col gap-3 rounded-lg border border-border/70 bg-muted/30 p-3 sm:flex-row sm:items-center sm:p-3.5">
				<div className="flex w-full items-center gap-1.5 sm:max-w-sm sm:flex-1">
					<div className="relative min-w-0 flex-1">
						<Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
						<Input
							type="search"
							className="bg-background/60 pl-9"
							aria-label={m.store_search_placeholder()}
							placeholder={m.store_search_placeholder()}
							title="Filter catalog entries by title, description, package name, or author."
							value={query}
							onChange={(e) => setQuery(e.target.value)}
						/>
					</div>
					<HelpTip
						label="Search plugins"
						text="Filters the catalog by title, description, package name, or author. Combine with a source chip to narrow further."
					/>
				</div>
				{/* One chip per source, so an operator can see a third-party catalog's entries alone. */}
				{sources.length > 1 && (
					<fieldset
						aria-label={m.store_tab_sources()}
						className="m-0 flex min-w-0 flex-wrap items-center gap-2 border-0 p-0"
					>
						<HelpTip
							label="Source filter"
							text="Limit Browse to one catalog, or show every trusted source. External entries still carry their source name."
						/>
						<Button
							size="sm"
							variant={source === null ? "default" : "outline"}
							aria-pressed={source === null}
							title="Show plugins from every trusted catalog."
							onClick={() => setSource(null)}
						>
							{m.store_filter_all()}
						</Button>
						{sources.map((s) => (
							<Button
								key={s.name}
								size="sm"
								variant={source === s.name ? "default" : "outline"}
								aria-pressed={source === s.name}
								title={`Show only plugins from the "${s.name}" catalog.`}
								onClick={() => setSource(s.name)}
							>
								{s.name}
							</Button>
						))}
					</fieldset>
				)}
			</div>

			<QueryState
				isLoading={catalog.isLoading}
				error={catalog.error}
				refetch={catalog.refetch}
			>
				{shown.length === 0 ? (
					<Card>
						<CardContent
							flush
							className="flex flex-col items-center gap-3 p-10 text-center sm:p-12"
						>
							<span className="flex size-11 items-center justify-center rounded-full bg-muted text-muted-foreground">
								<PackageSearch className="size-5" aria-hidden />
							</span>
							<p className="max-w-md text-sm text-muted-foreground">
								{entries.length > 0
									? m.store_no_match()
									: failedSources.length > 0
										? // An all-sources-failed catalog is a SUCCESSFUL request that happens to
											// carry nothing, so "no plugins available" was the console reporting a
											// broken fetch as an empty store. Name the sources that failed.
											m.store_all_sources_failed({
												sources: failedSources.map((f) => f.name).join(", "),
											})
										: m.store_empty()}
							</p>
						</CardContent>
					</Card>
				) : (
					<div className="@container">
						<div className="grid grid-cols-1 gap-3 @xl:grid-cols-2 @4xl:grid-cols-3">
							{shown.map((entry) => (
								<StoreCard
									key={`${entry.source}/${entry.id}`}
									entry={entry}
									onInstall={() => onInstall(entry)}
								/>
							))}
						</div>
					</div>
				)}
			</QueryState>

			{/* The ONLY way to the raw-spec install. Deliberately a quiet footer link, not a button on
			    a card: an unverified install should take a decision, never a stray click. */}
			<div className="flex items-center justify-center gap-1.5 border-t border-border/60 pt-4">
				<button
					type="button"
					onClick={onInstallSpec}
					title="Install a raw package spec with no catalog review. Prefer a catalog entry from Browse when one exists."
					className="text-xs text-muted-foreground underline underline-offset-4 transition-colors hover:text-foreground"
				>
					{m.store_spec_open()}
				</button>
				<HelpTip
					label={m.store_spec_open()}
					text="Opens the unverified install dialog. Prefer Browse for verified or external catalog entries; use this only when you have a specific package spec."
				/>
			</div>
		</div>
	);
};

/**
 * Catalog platform identifiers map to short display names. Proper nouns are deliberately not
 * routed through i18n, and `macos` is spelled the way Apple spells it.
 */
const PLATFORM_LABELS: Record<string, string> = {
	linux: "Linux",
	macos: "macOS",
};

/**
 * `CardContent` zeroes its top padding (`pt-0`/`sm:pt-0`) because it normally sits under a
 * `CardHeader` that already supplies it — these cards have no header, so the top padding
 * has to come back explicitly, at BOTH breakpoints. `p-card` alone does not do it: `card`
 * is a custom `--spacing-*` token, which tailwind-merge does not recognise as a spacing
 * value and therefore never dedupes against `pt-0`, leaving the longhand to win.
 */
const HEADERLESS_CARD_PADDING = "p-card pt-card sm:pt-card";

/** One catalog entry. Blocked entries shout; incompatible ones grey out; neither can be installed. */
export const StoreCard: FC<{ entry: StoreEntry; onInstall: () => void }> = ({
	entry,
	onInstall,
}) => {
	const Icon = pluginIcon(entry.icon);
	const blocked = entry.blocked !== undefined;
	const installed = entry.installed_version !== undefined;
	const installable = !blocked && entry.compatible;

	return (
		<Card
			className={cn(
				"flex flex-col transition-[box-shadow] duration-200 hover:ring-accent/70",
				blocked && "ring-2 ring-destructive/60 hover:ring-destructive/60",
				!entry.compatible && !blocked && "opacity-60",
			)}
		>
			<CardContent
				className={cn("flex flex-1 flex-col gap-3", HEADERLESS_CARD_PADDING)}
			>
				<div className="flex items-start gap-3">
					<span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-primary/12 ring-1 ring-primary/20">
						<Icon className="size-5 text-foreground" />
					</span>
					<div className="min-w-0 flex-1">
						<h3
							className="truncate font-medium tracking-tight"
							title={entry.title}
						>
							{entry.title}
						</h3>
						<p className="truncate text-xs text-muted-foreground">
							{m.store_by_author({ author: entry.author })} · v{entry.version}
						</p>
					</div>
				</div>

				<div className="flex flex-wrap items-center gap-2">
					<TierBadge tier={entry.tier} />
					{/* Attribution, never verification: an external entry names who curated it. */}
					{entry.tier === "external" && <SourceChip source={entry.source} />}
				</div>

				<p className="line-clamp-3 text-sm leading-relaxed text-muted-foreground">
					{entry.description}
				</p>

				<div className="flex flex-wrap gap-1.5">
					{entry.platforms.map((p) => (
						<Badge key={p} variant="secondary" className="font-normal">
							{PLATFORM_LABELS[p] ?? "Other"}
						</Badge>
					))}
				</div>

				{blocked && (
					<p className="flex items-start gap-2 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm font-medium text-destructive">
						<Ban className="mt-0.5 size-4 shrink-0" />
						<span>{m.store_blocked({ reason: entry.blocked ?? "" })}</span>
					</p>
				)}

				{!entry.compatible && !blocked && (
					<p className="flex items-start gap-2 rounded-md bg-amber-500/10 px-2.5 py-1.5 text-xs text-amber-700 dark:text-amber-400">
						<AlertTriangle className="mt-px size-3.5 shrink-0" />
						<span>{entry.incompatible_reason ?? m.store_incompatible()}</span>
					</p>
				)}

				{/* Footer pinned to the bottom so cards in a row line their actions up. */}
				<div className="mt-auto flex flex-wrap items-center gap-3 border-t border-border/50 pt-3">
					{entry.update_available ? (
						<Button
							size="sm"
							disabled={!installable}
							title={
								installable
									? `Update to v${entry.version}. Opens a confirm dialog for this catalog entry.`
									: "This entry cannot be updated on this host."
							}
							onClick={onInstall}
						>
							<Download className="size-4" />
							{m.store_update_to({ version: entry.version })}
						</Button>
					) : installed ? (
						<span
							className="inline-flex items-center gap-1.5 text-sm text-muted-foreground"
							title="Already installed at the catalog version shown above."
						>
							<Check className="size-4 text-[var(--success)]" />
							{m.store_installed_label()}
						</span>
					) : (
						<Button
							size="sm"
							disabled={!installable}
							title={
								!installable
									? "This entry cannot be installed on this host."
									: entry.tier === "external"
										? "Install from an external catalog. Opens a warning confirm; enable the plugin runner if it is off."
										: "Install from the built-in catalog. Enable the plugin runner afterward if it is off."
							}
							onClick={onInstall}
						>
							<Download className="size-4" />
							{m.store_install()}
						</Button>
					)}
					{entry.homepage && (
						<a
							href={entry.homepage}
							target="_blank"
							rel="noreferrer"
							title="Open this plugin's homepage in a new tab."
							className="ml-auto text-xs text-muted-foreground underline underline-offset-4 transition-colors hover:text-foreground"
						>
							{m.store_homepage()}
						</a>
					)}
				</div>
			</CardContent>
		</Card>
	);
};
