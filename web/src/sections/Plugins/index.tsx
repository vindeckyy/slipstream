// A plugin's UI, embedded in the console (plugin-ui-surface §5). We probe the plugin's liveness
// first and only mount the iframe when it answers — otherwise the iframe would show the proxy's raw
// 502. The iframe is same-origin (proxied through /plugin-ui), so the plugin can talk to its own
// loopback REST with the operator's session and, optionally, keep the address bar in sync by posting
// `{ type: "ss-ui:navigate", path }` to the parent.
import { useQuery } from "@tanstack/react-query";
import { getRouteApi, useNavigate } from "@tanstack/react-router";
import {
	AlertTriangle,
	CheckCircle2,
	ExternalLink,
	RefreshCw,
} from "lucide-react";
import { type FC, useEffect, useMemo, useRef } from "react";
import { pluginIcon, usePlugins } from "@/api/plugins";
import { useInstalledPlugins } from "@/api/store";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useLocale } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages";
import { SourceChip, TierBadge } from "@/sections/Store/TierBadge";

const route = getRouteApi("/plugins/$pluginId/$");

export const SectionPlugin: FC = () => {
	useLocale();
	const { pluginId, _splat } = route.useParams();
	const navigate = useNavigate();
	const iframeRef = useRef<HTMLIFrameElement>(null);

	// Header metadata (title/version/icon) from the directory; falls back to the id.
	const { data: plugins } = usePlugins();
	const meta = plugins?.find((p) => p.id === pluginId);
	const Icon = pluginIcon(meta?.ui?.icon);
	const title = meta?.title ?? pluginId;

	// Provenance follows the plugin into its own page: an unverified plugin must stay visibly
	// unverified WHILE you use it, not only in the store listing. The store keys installations by
	// package, so the runtime id is matched through `plugin_id`.
	const { data: installed } = useInstalledPlugins();
	const provenance = installed?.find((p) => p.plugin_id === pluginId);

	// Liveness: a 200 from /__health means the plugin is up.
	//
	// Two subtleties, both learned the hard way:
	//
	//  - A 200 is not enough. `fetch` follows redirects, so an expired session — where the gate
	//    answers 302 → /login → 200 HTML — looked exactly like a healthy plugin, and the console
	//    rendered its own login page inside the plugin's iframe. `redirect: "manual"` makes that
	//    an opaque response we can reject instead.
	//  - One failure must not be terminal. The runner is restarted at the end of every successful
	//    install, so a single missed probe is routine; giving up on the first one threw away
	//    whatever the operator had open in another plugin. Retry a few times, and keep probing on a
	//    slower beat while down so it recovers on its own.
	const health = useQuery({
		queryKey: ["plugin-health", pluginId],
		queryFn: async () => {
			const r = await fetch(`/plugin-ui/${pluginId}/__health`, {
				credentials: "same-origin",
				redirect: "manual",
			});
			// `type === "opaqueredirect"` is the gate bouncing us to /login, not the plugin answering.
			if (r.type === "opaqueredirect") throw new Error("session expired");
			if (!r.ok) throw new Error(`health ${r.status}`);
			return true;
		},
		retry: 2,
		refetchInterval: (q) => (q.state.status === "error" ? 5_000 : 20_000),
	});

	// The iframe src is fixed at the initial deep-link path; the plugin's own in-app navigation drives
	// the console URL via postMessage (below), never the src — so there's no reload loop.
	// biome-ignore lint/correctness/useExhaustiveDependencies: intentionally pinned to the initial path
	const initialSrc = useMemo(
		() => `/plugin-ui/${pluginId}/${_splat ?? ""}`,
		[pluginId],
	);
	const healthState: PluginHealthState = health.isError
		? "offline"
		: health.isSuccess
			? "running"
			: "loading";

	// Keep the console address bar in sync with the plugin's internal routing.
	useEffect(() => {
		const onMessage = (e: MessageEvent) => {
			if (e.source !== iframeRef.current?.contentWindow) return;
			const data = e.data as { type?: string; path?: string };
			if (data?.type === "ss-ui:navigate" && typeof data.path === "string") {
				navigate({
					to: "/plugins/$pluginId/$",
					params: { pluginId, _splat: data.path.replace(/^\//, "") },
					replace: true,
				});
			}
		};
		window.addEventListener("message", onMessage);
		return () => window.removeEventListener("message", onMessage);
	}, [pluginId, navigate]);

	return (
		<section
			aria-labelledby="plugin-frame-title"
			className="flex h-[calc(100dvh-8rem)] min-h-0 flex-col gap-3 sm:h-[calc(100dvh-6rem)] lg:h-[calc(100dvh-5rem)]"
		>
			<header className="flex shrink-0 flex-col gap-3 rounded-xl border border-border/70 bg-card/90 p-3 shadow-sm sm:p-4">
				<div className="flex min-w-0 items-start gap-3">
					<span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-primary/12 ring-1 ring-primary/20">
						<Icon className="size-5 text-foreground" aria-hidden />
					</span>
					<div className="min-w-0 flex-1">
						<p className="text-[11px] font-medium uppercase tracking-[0.08em] text-muted-foreground/70">
							{m.nav_plugins()}
						</p>
						<div className="mt-0.5 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
							<h1
								id="plugin-frame-title"
								className="min-w-0 truncate text-lg font-semibold tracking-tight"
							>
								{title}
							</h1>
							{meta?.version && (
								<span className="rounded-md bg-muted px-1.5 py-0.5 text-xs tabular-nums text-muted-foreground">
									v{meta.version}
								</span>
							)}
						</div>
						<div className="mt-2 flex min-w-0 flex-wrap items-center gap-2">
							<code className="max-w-full truncate rounded-md border border-border/60 bg-muted/40 px-2 py-1 text-xs text-muted-foreground">
								{pluginId}
							</code>
							<PluginHealthBadge state={healthState} />
							{provenance && (
								<>
									<TierBadge tier={provenance.tier} />
									{provenance.tier === "external" && provenance.source && (
										<SourceChip
											source={provenance.source}
											className="rounded-md border border-border/60 bg-muted/30 px-2 py-1"
										/>
									)}
								</>
							)}
						</div>
					</div>
					<a
						href={initialSrc}
						target="_blank"
						rel="noopener noreferrer"
						aria-label={`${m.plugin_open_new_tab()} ${title}`}
						title={m.plugin_open_new_tab()}
						className="inline-flex size-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-background/70 text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 sm:h-9 sm:w-auto sm:gap-1.5 sm:px-3"
					>
						<ExternalLink className="size-4" aria-hidden />
						<span className="hidden text-sm font-medium sm:inline">
							{m.plugin_open_new_tab()}
						</span>
					</a>
				</div>
			</header>

			{health.isError ? (
				<OfflineCard title={title} onRetry={() => health.refetch()} />
			) : health.isSuccess ? (
				<iframe
					ref={iframeRef}
					src={initialSrc}
					title={title}
					className="min-h-0 w-full flex-1 rounded-xl border border-border/70 bg-card shadow-sm"
					// The plugin is operator-installed code on our own origin (no new trust boundary —
					// plugin-ui-surface §7.4); allow it to run scripts, forms, popups, and full-window.
					sandbox="allow-scripts allow-forms allow-popups allow-same-origin allow-modals"
					allow="fullscreen"
				/>
			) : (
				// Probing: keep the frame-shaped surface stable while avoiding motion for reduced-motion
				// users.
				<div
					role="status"
					aria-label={m.common_loading()}
					className="relative flex min-h-0 flex-1 items-center justify-center overflow-hidden rounded-xl border border-border/70 bg-muted/30"
				>
					<div
						aria-hidden
						className="absolute inset-0 bg-gradient-to-br from-primary/10 via-transparent to-primary/5 motion-safe:animate-pulse motion-reduce:animate-none"
					/>
					<div className="relative flex flex-col items-center gap-2 text-center">
						<RefreshCw
							className="size-5 text-muted-foreground motion-safe:animate-spin motion-reduce:animate-none"
							aria-hidden
						/>
						<span className="text-sm text-muted-foreground">
							{m.common_loading()}
						</span>
					</div>
				</div>
			)}
		</section>
	);
};

type PluginHealthState = "loading" | "running" | "offline";

const PluginHealthBadge: FC<{ state: PluginHealthState }> = ({ state }) => (
	<Badge
		variant={
			state === "running"
				? "success"
				: state === "offline"
					? "destructive"
					: "secondary"
		}
		className={cn(
			"gap-1.5 whitespace-nowrap",
			state === "loading" && "text-muted-foreground",
		)}
		role="status"
		aria-live="polite"
	>
		{state === "running" ? (
			<CheckCircle2 className="size-3.5" aria-hidden />
		) : state === "offline" ? (
			<AlertTriangle className="size-3.5" aria-hidden />
		) : (
			<RefreshCw
				className="size-3.5 motion-safe:animate-spin motion-reduce:animate-none"
				aria-hidden
			/>
		)}
		{state === "running"
			? m.store_running()
			: state === "offline"
				? m.store_stopped()
				: m.common_loading()}
	</Badge>
);

const OfflineCard: FC<{ title: string; onRetry: () => void }> = ({
	title,
	onRetry,
}) => (
	<div
		role="alert"
		className="flex min-h-0 flex-1 items-center justify-center overflow-y-auto rounded-xl border border-dashed border-border/80 bg-muted/20"
	>
		<div className="flex w-full max-w-lg flex-col items-center gap-4 p-4 text-center sm:p-8">
			<span className="flex size-11 items-center justify-center rounded-full bg-destructive/10 text-destructive ring-1 ring-destructive/20">
				<AlertTriangle className="size-5" aria-hidden />
			</span>
			<div className="space-y-1">
				<h2 className="text-base font-semibold tracking-tight">
					{m.plugin_offline_title()}
				</h2>
				<p className="text-sm text-muted-foreground">
					<span className="font-medium text-foreground">{title}</span>
					{": "}
					{m.plugin_offline_hint()}
				</p>
			</div>
			{/* The exact runner commands, so the operator can act without leaving the page. */}
			<pre className="w-full overflow-x-auto rounded-lg border border-border/60 bg-muted/50 p-3 text-left text-xs text-muted-foreground">
				<code>
					systemctl --user status slipstream-scripting{"\n"}
					Get-ScheduledTask SlipstreamScripting{"  # Windows"}
				</code>
			</pre>
			<Button
				variant="outline"
				size="sm"
				className="w-full sm:w-auto"
				onClick={onRetry}
			>
				<RefreshCw className="size-4" aria-hidden />
				{m.plugin_retry()}
			</Button>
		</div>
	</div>
);
