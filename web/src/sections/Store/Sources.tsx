import { toast } from "@unom/ui/toast";
import {
	AlertTriangle,
	Lock,
	RefreshCw,
	ShieldCheck,
	ShieldOff,
	Trash2,
} from "lucide-react";
import { type FC, type FormEvent, useEffect, useState } from "react";
import { ApiError } from "@/api/fetcher";
import {
	type SourceBody,
	type StoreSource,
	useDeleteSource,
	useRefreshCatalog,
	useSetSource,
	useStoreSources,
} from "@/api/store";
import { HelpTip, OptionLabel } from "@/components/option-help";
import { QueryState } from "@/components/query-state";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { fmtDateTimeSecs } from "@/lib/format";
import { m } from "@/paraglide/messages";

/** A source the operator has filled in but not yet agreed to trust. The console password is NOT
 * part of the draft — it is collected by the trust dialog, at the moment the decision is made. */
type SourceDraft = Omit<SourceBody, "password"> & { name: string };

/** Unix seconds → a locale date-time, or "never" for a source that has never fetched.
 * Locale-aware via lib/format.ts — `toLocaleString` follows the browser, not the console. */
const fmtFetched = (secs: number): string =>
	secs > 0 ? fmtDateTimeSecs(secs) : m.store_source_never();

/**
 * Container: the catalog sources. Owns the source listing, the refresh-all action, and add/remove.
 * Adding is a two-step: the form collects the source, and a one-time trust dialog states plainly
 * what trusting a third-party catalog means before anything is written to the host.
 */
export const SourcesTab: FC = () => {
	const sources = useStoreSources();
	const refresh = useRefreshCatalog();
	const save = useSetSource();
	const remove = useDeleteSource();
	// The draft waiting on the trust dialog, and a key that re-mounts (and so clears) the form.
	const [draft, setDraft] = useState<SourceDraft | null>(null);
	const [formKey, setFormKey] = useState(0);
	const [wrongPassword, setWrongPassword] = useState(false);

	const onRefresh = () =>
		refresh.mutate(undefined, {
			onError: () => toast.error(m.store_refresh_failed()),
		});

	const onConfirmAdd = async (password: string) => {
		if (!draft) return;
		setWrongPassword(false);
		try {
			await save.mutateAsync({ ...draft, password });
			setDraft(null);
			setFormKey((k) => k + 1);
		} catch (e) {
			// A rejected password keeps the dialog open so the operator can retry without refilling
			// the form; anything else is a genuine failure to write the source.
			if (e instanceof ApiError && e.status === 401) {
				setWrongPassword(true);
				return;
			}
			toast.error(m.store_add_source_failed());
		}
	};

	const onRemove = async (source: StoreSource) => {
		if (!confirm(m.store_source_remove_confirm({ name: source.name }))) return;
		try {
			await remove.mutateAsync(source.name);
		} catch (e) {
			// 403 is the host refusing to drop its built-in catalog — say exactly that.
			toast.error(
				e instanceof ApiError && e.status === 403
					? m.store_source_builtin_locked()
					: m.store_source_remove_failed(),
			);
		}
	};

	return (
		<div className="flex flex-col gap-card">
			<SourceList
				sources={sources}
				busyName={remove.isPending ? (remove.variables ?? null) : null}
				isRefreshing={refresh.isPending}
				onRefresh={onRefresh}
				onRemove={onRemove}
			/>

			<AddSourceForm
				key={formKey}
				onSubmit={setDraft}
				isSaving={save.isPending}
			/>

			<TrustSourceDialog
				draft={draft}
				isSaving={save.isPending}
				wrongPassword={wrongPassword}
				onCancel={() => {
					setDraft(null);
					setWrongPassword(false);
				}}
				onConfirm={onConfirmAdd}
			/>
		</div>
	);
};

/** The source table: health per source, with the built-in one locked. */
export const SourceList: FC<{
	sources: {
		data?: StoreSource[];
		isLoading: boolean;
		error: unknown;
		refetch?: () => void;
	};
	/** Name of the source whose delete is in flight, or null. */
	busyName: string | null;
	isRefreshing: boolean;
	onRefresh: () => void;
	onRemove: (source: StoreSource) => void;
}> = ({ sources, busyName, isRefreshing, onRefresh, onRemove }) => {
	const rows = sources.data ?? [];
	return (
		<Card>
			<CardHeader className="flex-row items-center justify-between space-y-0 pb-3">
				<div className="space-y-1">
					<CardTitle className="inline-flex items-center gap-1.5 text-base tracking-tight">
						{m.store_sources_title()}
						<HelpTip
							label={m.store_sources_title()}
							text="Catalogs this host fetches for Browse. The built-in unom catalog stays; every other source is one you add and vouch for."
						/>
					</CardTitle>
				</div>
				<Button
					variant="outline"
					size="sm"
					disabled={isRefreshing}
					title="Re-fetch every catalog index now. Use this after adding a source or when listings look stale."
					onClick={onRefresh}
				>
					<RefreshCw
						className={isRefreshing ? "size-4 animate-spin" : "size-4"}
					/>
					{m.store_refresh_all()}
				</Button>
			</CardHeader>
			<CardContent className="space-y-4">
				<p className="max-w-prose text-sm text-muted-foreground">
					{m.store_sources_help()}
				</p>

				<QueryState
					isLoading={sources.isLoading}
					error={sources.error}
					refetch={sources.refetch}
				>
					<div className="flex flex-col gap-2">
						{rows.map((s) => (
							<div
								key={s.name}
								className="flex flex-col gap-3 rounded-lg border border-border/70 bg-muted/30 p-3 sm:flex-row sm:items-start"
							>
								<div className="min-w-0 flex-1 space-y-1.5">
									<div className="flex flex-wrap items-center gap-2">
										<span className="font-medium tracking-tight">{s.name}</span>
										{s.builtin && (
											<Badge variant="secondary" className="gap-1">
												<Lock className="size-3" />
												{m.store_source_builtin()}
											</Badge>
										)}
										{s.signed ? (
											<Badge variant="outline" className="gap-1">
												<ShieldCheck className="size-3" />
												{m.store_source_signed()}
											</Badge>
										) : (
											<Badge
												variant="outline"
												className="gap-1 border-amber-600/40 text-amber-700 dark:border-amber-500/40 dark:text-amber-400"
											>
												<ShieldOff className="size-3" />
												{m.store_source_unsigned()}
											</Badge>
										)}
										{s.stale && (
											<Badge
												variant="outline"
												className="gap-1 border-amber-600/40 text-amber-700 dark:border-amber-500/40 dark:text-amber-400"
											>
												<AlertTriangle className="size-3" />
												{m.store_source_stale()}
											</Badge>
										)}
									</div>
									<p className="truncate font-mono text-xs text-muted-foreground">
										{s.url}
									</p>
									<p className="text-xs text-muted-foreground">
										{m.store_source_entries({ count: s.entry_count })} ·{" "}
										{m.store_source_fetched({ when: fmtFetched(s.fetched_at) })}
									</p>
									{s.error && (
										<p className="rounded-md border border-destructive/30 bg-destructive/10 px-2 py-1 text-xs text-destructive">
											{s.error}
										</p>
									)}
								</div>
								{/* The built-in catalog gets no delete button at all — not a disabled one. */}
								{!s.builtin && (
									<Button
										variant="ghost"
										size="icon"
										className="self-end sm:self-start"
										aria-label={m.store_source_remove()}
										title="Remove this catalog. Plugins already installed from it stay installed."
										disabled={busyName === s.name}
										onClick={() => onRemove(s)}
									>
										<Trash2 className="size-4 text-destructive" />
									</Button>
								)}
							</div>
						))}
					</div>
				</QueryState>
			</CardContent>
		</Card>
	);
};

/** The add-source form. Reports a draft; the parent takes it through the trust dialog. */
export const AddSourceForm: FC<{
	onSubmit: (draft: SourceDraft) => void;
	isSaving: boolean;
}> = ({ onSubmit, isSaving }) => {
	const [name, setName] = useState("");
	const [url, setUrl] = useState("");
	const [publicKey, setPublicKey] = useState("");

	const handleSubmit = (e: FormEvent) => {
		e.preventDefault();
		const key = publicKey.trim();
		if (!name.trim() || !url.trim()) return;
		onSubmit({
			name: name.trim(),
			url: url.trim(),
			public_key: key ? key : undefined,
		});
	};

	return (
		<Card className="max-w-xl">
			<CardHeader className="pb-3">
				<CardTitle className="inline-flex items-center gap-1.5 text-base tracking-tight">
					{m.store_add_source_title()}
					<HelpTip
						label={m.store_add_source_title()}
						text="Adds a third-party catalog. Entries from it show as External. Only add a catalog whose operator you trust."
					/>
				</CardTitle>
			</CardHeader>
			<CardContent>
				<form onSubmit={handleSubmit} className="space-y-4">
					<div className="space-y-2">
						<OptionLabel
							label={m.store_field_source_name()}
							htmlFor="store-source-name"
							help="Short label shown in Browse filters and on external entries. Pick something you will recognize later."
							recommended="A short unique name"
						/>
						<Input
							id="store-source-name"
							required
							autoComplete="off"
							spellCheck={false}
							title="Short unique name for this catalog source."
							value={name}
							onChange={(e) => setName(e.target.value)}
						/>
					</div>
					<div className="space-y-2">
						<OptionLabel
							label={m.store_field_source_url()}
							htmlFor="store-source-url"
							help="HTTPS index URL for this catalog. The host fetches plugin listings from here."
							recommended="HTTPS catalog index URL"
						/>
						<Input
							id="store-source-url"
							required
							type="url"
							inputMode="url"
							title="HTTPS URL of the catalog index."
							value={url}
							onChange={(e) => setUrl(e.target.value)}
						/>
					</div>
					<div className="space-y-2">
						<OptionLabel
							label={m.store_field_source_key()}
							htmlFor="store-source-key"
							help="An ed25519:... key. With a key set, the host only accepts a signed index from this source."
							recommended="Set when the source publishes an ed25519 key"
						/>
						<Input
							id="store-source-key"
							autoComplete="off"
							spellCheck={false}
							placeholder="ed25519:..."
							title="Optional ed25519 public key for verifying this catalog's index."
							value={publicKey}
							onChange={(e) => setPublicKey(e.target.value)}
						/>
						<p className="text-xs text-muted-foreground">
							{m.store_field_source_key_help()}
						</p>
					</div>
					<Button
						type="submit"
						disabled={isSaving || !name.trim() || !url.trim()}
						title="Continue to the trust confirmation. Nothing is saved until you confirm with the console password."
					>
						{m.store_add_source()}
					</Button>
				</form>
			</CardContent>
		</Card>
	);
};

/** The one-time trust warning shown before a third-party catalog is written to the host. */
export const TrustSourceDialog: FC<{
	draft: SourceDraft | null;
	isSaving: boolean;
	onCancel: () => void;
	onConfirm: (password: string) => void;
	/** Set when the BFF rejected the password (401) — say so and keep the dialog open. */
	wrongPassword?: boolean;
}> = ({ draft, isSaving, onCancel, onConfirm, wrongPassword }) => {
	const [password, setPassword] = useState("");
	// The dialog stays mounted between drafts; clear the password whenever it closes.
	useEffect(() => {
		if (!draft) setPassword("");
	}, [draft]);
	return (
		<Dialog open={draft !== null} onOpenChange={(open) => !open && onCancel()}>
			{draft && (
				<DialogContent>
					<DialogHeader>
						<DialogTitle className="flex items-center gap-2">
							<AlertTriangle className="size-5 shrink-0 text-amber-600 dark:text-amber-500" />
							{m.store_source_trust_title()}
							<HelpTip
								label={m.store_source_trust_title()}
								text="Adding a catalog is a trust-root change. Every future install from it rides on this decision."
							/>
						</DialogTitle>
						<DialogDescription>
							{m.store_source_trust_body({ name: draft.name })}
						</DialogDescription>
					</DialogHeader>

					<p className="rounded-lg bg-muted/60 px-3 py-2.5 font-mono text-xs break-all text-muted-foreground">
						{draft.url}
					</p>

					{!draft.public_key && (
						<p className="rounded-lg border border-amber-600/40 bg-amber-500/10 px-3 py-2.5 text-sm text-amber-700 dark:border-amber-500/40 dark:text-amber-400">
							{m.store_source_trust_unsigned()}
						</p>
					)}

					{/* Adding a source is a trust-root change: every future install rides on it, so the
				    console password is re-entered here and verified at the BFF, exactly as for a
				    host update. */}
					<div className="space-y-2">
						<OptionLabel
							label={m.store_source_password()}
							htmlFor="store-source-password"
							help="The console password is verified before this source is written. A browser session alone cannot add a catalog."
							recommended="Required"
						/>
						<Input
							id="store-source-password"
							type="password"
							autoComplete="current-password"
							title="Enter the console password to authorize adding this catalog."
							value={password}
							onChange={(e) => setPassword(e.target.value)}
						/>
						{wrongPassword && (
							<p role="alert" className="text-xs text-destructive">
								{m.update_apply_wrong_password()}
							</p>
						)}
					</div>

					<DialogFooter>
						<Button
							variant="outline"
							onClick={onCancel}
							disabled={isSaving}
							title="Close without adding this catalog."
						>
							{m.common_cancel()}
						</Button>
						<Button
							disabled={isSaving || password.length === 0}
							title="Trust this catalog and write it to the host."
							onClick={() => onConfirm(password)}
						>
							{m.store_source_trust_confirm()}
						</Button>
					</DialogFooter>
				</DialogContent>
			)}
		</Dialog>
	);
};
