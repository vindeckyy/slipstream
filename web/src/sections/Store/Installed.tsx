import { ArrowUpCircle, Ban, Circle, Package, Trash2 } from "lucide-react";
import type { FC } from "react";
import { type InstalledPlugin, useInstalledPlugins } from "@/api/store";
import { HelpTip } from "@/components/option-help";
import { QueryState } from "@/components/query-state";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableRow } from "@/components/ui/table";
import type { Loadable } from "@/lib/query";
import { m } from "@/paraglide/messages";
import { RunnerCardSection } from "./Runner";
import { SourceChip, TierBadge } from "./TierBadge";

/**
 * Container: what's installed. Owns the installed query; the runner switch above it owns its own.
 * Updating and uninstalling are escalated to the parent, which owns the confirm dialogs and the
 * resulting job card.
 */
export const InstalledTab: FC<{
	onUpdate: (plugin: InstalledPlugin) => void;
	onUninstall: (plugin: InstalledPlugin) => void;
	/** Package whose install/uninstall is in flight, or null — only that row's actions disable. */
	busyPkg: string | null;
}> = ({ onUpdate, onUninstall, busyPkg }) => {
	const installed = useInstalledPlugins();
	return (
		<div className="flex flex-col gap-card">
			<RunnerCardSection />
			<InstalledList
				installed={installed}
				onUpdate={onUpdate}
				onUninstall={onUninstall}
				busyPkg={busyPkg}
			/>
		</div>
	);
};

/**
 * One row per installed plugin. The tier badge is permanent and non-negotiable: a plugin installed
 * from a raw spec stays marked unverified here for as long as it's on the host, no matter what it
 * later reports about itself.
 */
export const InstalledList: FC<{
	installed: Loadable<InstalledPlugin[]>;
	onUpdate: (plugin: InstalledPlugin) => void;
	onUninstall: (plugin: InstalledPlugin) => void;
	busyPkg: string | null;
}> = ({ installed, onUpdate, onUninstall, busyPkg }) => {
	const rows = installed.data ?? [];
	return (
		<Card>
			<CardContent flush>
				<CardHeader className="pb-3">
					<CardTitle className="inline-flex items-center gap-1.5 text-base tracking-tight">
						{m.store_installed_title()}
						<HelpTip
							label={m.store_installed_title()}
							text="Plugins already on this host. Tier badges stay permanent: a raw-spec install stays Unverified for as long as it is installed."
						/>
					</CardTitle>
				</CardHeader>

				<QueryState
					isLoading={installed.isLoading}
					error={installed.error}
					refetch={installed.refetch}
				>
					{rows.length === 0 ? (
						<div className="flex flex-col items-center gap-3 px-card pb-card pt-2 text-center">
							<span className="flex size-11 items-center justify-center rounded-full bg-muted text-muted-foreground">
								<Package className="size-5" aria-hidden />
							</span>
							<p className="max-w-sm text-sm text-muted-foreground">
								{m.store_installed_empty()}
							</p>
						</div>
					) : (
						<div className="overflow-x-auto">
							<Table>
								<TableBody>
									{rows.map((p) => (
										<TableRow key={p.pkg} className="align-top">
											<TableCell className="min-w-[12rem] py-3.5">
												<div className="font-medium tracking-tight">
													{p.title ?? p.pkg}
												</div>
												<div className="font-mono text-xs text-muted-foreground">
													{p.pkg}
												</div>
												{p.blocked !== undefined && (
													<p className="mt-2 flex items-start gap-2 rounded-lg border border-destructive/40 bg-destructive/10 px-2 py-1.5 text-xs font-medium text-destructive">
														<Ban className="mt-px size-3.5 shrink-0" />
														<span>
															{m.store_blocked({ reason: p.blocked })}
														</span>
													</p>
												)}
											</TableCell>
											<TableCell className="py-3.5">
												<div className="flex flex-col items-start gap-1">
													<TierBadge tier={p.tier} />
													{p.tier === "external" && p.source && (
														<SourceChip source={p.source} />
													)}
												</div>
											</TableCell>
											<TableCell className="py-3.5 text-sm tabular-nums text-muted-foreground">
												{p.version
													? `v${p.version}`
													: m.store_version_unknown()}
											</TableCell>
											<TableCell className="py-3.5">
												<span className="inline-flex items-center gap-1.5 rounded-md bg-muted/50 px-2 py-1 text-xs text-muted-foreground">
													<Circle
														className={
															p.running
																? "size-2 fill-[var(--success)] text-[var(--success)]"
																: "size-2 fill-muted-foreground text-muted-foreground"
														}
													/>
													{p.running ? m.store_running() : m.store_stopped()}
												</span>
											</TableCell>
											<TableCell className="py-3.5 text-right">
												<div className="flex items-center justify-end gap-2">
													{p.update_available !== undefined && (
														<Button
															size="sm"
															disabled={busyPkg === p.pkg}
															title={`Update to v${p.update_available}. Opens the same confirm dialog a fresh install would.`}
															onClick={() => onUpdate(p)}
														>
															<ArrowUpCircle className="size-4" />
															{m.store_update_to({
																version: p.update_available,
															})}
														</Button>
													)}
													<Button
														variant="ghost"
														size="icon"
														aria-label={m.store_uninstall()}
														title="Remove this plugin from the host. You can install it again from Browse."
														disabled={busyPkg === p.pkg}
														onClick={() => onUninstall(p)}
													>
														<Trash2 className="size-4 text-destructive" />
													</Button>
												</div>
											</TableCell>
										</TableRow>
									))}
								</TableBody>
							</Table>
						</div>
					)}
				</QueryState>
			</CardContent>
		</Card>
	);
};
