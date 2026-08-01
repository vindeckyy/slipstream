import { Link, useRouterState } from "@tanstack/react-router";
import {
	AppWindow,
	GaugeCircle,
	Home,
	KeyRound,
	LogOut,
	MonitorPlay,
	MoreHorizontal,
	Puzzle,
	Server,
	Settings,
	Users,
	Workflow,
	Wrench,
	X,
} from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { toast } from "@unom/ui/toast";
import { useGetHostInfo, useGetStatus } from "@/api/gen/host/host";
import { useHostEvents } from "@/api/events";
import { pluginIcon, uiPlugins, usePlugins } from "@/api/plugins";
import { CommandPalette } from "@/components/command-palette";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { changeLocale, type Locale, locales, useLocale } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages";

const MLink = motion(Link);

type NavItem = {
	to: string;
	icon: typeof Home;
	label: () => string;
	help: () => string;
};

const navDashboard: NavItem = {
	to: "/",
	icon: Home,
	label: () => m.nav_dashboard(),
	help: () =>
		"Live host overview: session state, paired clients, and recent activity.",
};
const navPairing: NavItem = {
	to: "/pin",
	icon: KeyRound,
	label: () => m.nav_pairing(),
	help: () =>
		"Pair Moonlight or Slipstream clients with a PIN, and manage already-paired devices.",
};
const navLibrary: NavItem = {
	to: "/apps",
	icon: AppWindow,
	label: () => m.nav_library(),
	help: () =>
		"Browse and manage the game library the host advertises to clients.",
};
const navConfig: NavItem = {
	to: "/config",
	icon: Settings,
	label: () => m.nav_config(),
	help: () =>
		"Host capture, input, network, and encoder defaults. Changes usually need a host restart.",
};
const navLogs: NavItem = {
	to: "/troubleshoot",
	icon: Wrench,
	label: () => m.nav_logs(),
	help: () => "Read live host logs and export them for troubleshooting.",
};
const navHost: NavItem = {
	to: "/host",
	icon: Server,
	label: () => m.nav_host(),
	help: () =>
		"Host identity, GPU status, updates, power controls, and competing GameStream host warnings.",
};
const navDisplays: NavItem = {
	to: "/displays",
	icon: MonitorPlay,
	label: () => m.nav_displays(),
	help: () =>
		"Virtual display policy, presets, keep-alive, and live monitor layout.",
};
const navStats: NavItem = {
	to: "/stats",
	icon: GaugeCircle,
	label: () => m.nav_stats(),
	help: () =>
		"Capture performance graphs, recordings, and live stream health.",
};
const navAutomation: NavItem = {
	to: "/automation",
	icon: Workflow,
	label: () => m.nav_automation(),
	help: () =>
		"Run shell commands or webhooks when host events fire (connect, stream, pair, and more).",
};
const navPluginStore: NavItem = {
	to: "/plugins",
	icon: Puzzle,
	label: () => m.nav_plugin_store(),
	help: () => m.nav_plugin_store_help(),
};

const EXACT: readonly string[] = ["/", "/plugins"];

/**
 * The desktop rail is grouped by the operator's job: watch the host, manage clients and displays,
 * reach host tools, then change the console. The existing route targets stay intact, including the
 * compatibility aliases (`/pin`, `/apps`, and `/troubleshoot`).
 */
const NAV_GROUPS: readonly {
	id: string;
	label: () => string;
	items: readonly NavItem[];
}[] = [
	{
		id: "observe",
		label: () => m.status_title(),
		items: [navDashboard, navStats, navLogs],
	},
	{
		id: "operate",
		label: () => m.nav_library(),
		items: [navLibrary, navPairing, navDisplays],
	},
	{
		id: "host",
		label: () => m.nav_host(),
		items: [navHost, navAutomation],
	},
	{
		id: "system",
		label: () => m.nav_tools(),
		items: [navConfig, navPluginStore],
	},
];

/** Keep the most useful observatory views one tap away on a phone. */
const MOBILE_PRIMARY: readonly NavItem[] = [
	navDashboard,
	navLibrary,
	navHost,
	navStats,
];
const MOBILE_OVERFLOW: readonly NavItem[] = [
	navPairing,
	navConfig,
	navLogs,
	navDisplays,
	navAutomation,
	navPluginStore,
];

/** Dense M3 / console nav row: tonal active fill + leading indicator rail. */
const navItemClass =
	"group relative flex min-h-8 items-center gap-2.5 rounded-md px-2.5 py-1.5 text-[13px] leading-snug text-muted-foreground outline-none transition-colors hover:bg-muted/70 hover:text-foreground focus-visible:bg-muted/70 focus-visible:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:ring-offset-1 focus-visible:ring-offset-card";

const navItemActiveClass =
	"bg-primary/12 text-foreground font-medium hover:bg-primary/15";

const sectionLabelClass =
	"px-2.5 pb-1 pt-0.5 text-[11px] font-medium uppercase tracking-[0.06em] text-muted-foreground/65";

export function AppShell({ children }: { children: ReactNode }) {
	// Read the locale so the whole shell re-renders on a language switch.
	useLocale();
	// One subscription to the host's event stream for the whole console — it invalidates the queries
	// each event affects, so pages update on the transition instead of on their own timer. The
	// polling intervals stay as a floor in case the stream is unavailable.
	useHostEvents();
	return (
		<TooltipProvider>
			<div className="flex min-h-screen bg-background">
				{/* Desktop sidebar (≥ sm). Sticky at viewport height: the page (body) scrolls with
			    long content, but the sidebar stays pinned — the explicit h-dvh stops the flex
			    stretch that would otherwise grow it (and push the language switcher) below the
			    fold. overflow-y-auto lets the nav itself scroll on very short viewports. */}
				<aside className="sticky top-0 hidden h-dvh w-60 shrink-0 flex-col overflow-y-auto border-r border-border/80 bg-card sm:flex">
					<div className="flex flex-col gap-6 px-3 pb-4 pt-4">
						<Link
							to="/"
							aria-label={m.app_name()}
							className="flex items-center rounded-md px-1.5 py-1 outline-none focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:ring-offset-1 focus-visible:ring-offset-card"
						>
							<img
								src="/slipstream-logo.png"
								alt={m.app_name()}
								className="h-12 w-auto max-w-full object-contain"
								draggable={false}
							/>
						</Link>

						<motion.nav
							initial={{ opacity: 0 }}
							animate={{ opacity: 1 }}
							transition={{ duration: 0.18 }}
							className="flex flex-col gap-4"
							aria-label={m.app_name()}
						>
							{NAV_GROUPS.map(({ id, label, items }) => (
								<div key={id} className="flex flex-col gap-0.5">
									<p className={sectionLabelClass}>{label()}</p>
									{items.map((item) => (
										<NavTooltipLink key={item.to} item={item} />
									))}
								</div>
							))}
						</motion.nav>

						<PluginNavSection />
					</div>

					<div className="mt-auto space-y-2 border-t border-border/70 px-3 py-3">
						<LanguageSwitcher />
						<SignOutButton />
					</div>
				</aside>

				<div className="flex min-w-0 flex-1 flex-col overflow-x-hidden">
					{/* Mobile top bar (< sm): brand + language. The sidebar is hidden here. */}
					<header className="flex items-center gap-2 border-b border-border/80 bg-card px-3 py-2.5 sm:hidden">
						<img
							src="/slipstream-logo.png"
							alt={m.app_name()}
							className="h-7 w-auto max-w-[70%]"
							draggable={false}
						/>
						<div className="ml-auto shrink-0">
							<LanguageSwitcher />
						</div>
					</header>

					<main className="flex-1 bg-muted/15">
						{/* Mobile: side gutter so content isn't overly narrow; pb-24 leaves room for
					    the fixed bottom nav. Desktop: denser console padding; muted wash vs the
					    solid sidebar is the content frame. */}
						<div className="mx-auto max-w-[1700px] px-4 py-5 pb-24 sm:px-6 sm:py-6 sm:pb-6 lg:px-8 lg:py-7">
							<ConsoleStatusHeader />
							{children}
						</div>
					</main>
				</div>

				<MobileNav />
			</div>
		</TooltipProvider>
	);
}

function NavTooltipLink({ item }: { item: NavItem }) {
	const { to, icon: Icon, label, help } = item;
	return (
		<Tooltip>
			<TooltipTrigger asChild>
				<MLink
					whileTap={{ scale: 0.985 }}
					to={to}
					title={help()}
					activeOptions={{ exact: EXACT.includes(to) }}
					className={navItemClass}
					activeProps={{ className: navItemActiveClass }}
				>
					{/* TanStack Link sets data-status="active" on the active route. */}
					<span
						aria-hidden
						className="pointer-events-none absolute inset-y-1 left-0 w-0.5 rounded-full bg-primary opacity-0 [[data-status=active]_&]:opacity-100"
					/>
					<Icon className="relative size-4 shrink-0 opacity-80 [[data-status=active]_&]:opacity-100" />
					<span className="relative min-w-0 flex-1 truncate">{label()}</span>
				</MLink>
			</TooltipTrigger>
			<TooltipContent side="right" align="center">
				{help()}
			</TooltipContent>
		</Tooltip>
	);
}

function ConsoleStatusHeader() {
	const host = useGetHostInfo({
		query: {
			staleTime: 5 * 60_000,
			refetchOnWindowFocus: false,
		},
	});
	const status = useGetStatus({
		query: {
			refetchInterval: (query) =>
				query.state.data?.video_streaming ||
				query.state.data?.audio_streaming ||
				(query.state.data?.active_sessions ?? 0) > 0
					? 3_000
					: 15_000,
		},
	});
	const data = status.data;
	const isActive =
		data?.video_streaming ||
		data?.audio_streaming ||
		(data?.active_sessions ?? 0) > 0;
	const statusLabel = status.isError
		? m.common_error()
		: data
			? isActive
				? m.status_streaming()
				: m.status_idle()
			: m.common_loading();
	const statusTone = status.isError
		? "border-destructive/35 bg-destructive/10 text-destructive"
		: isActive
			? "border-primary/35 bg-primary/10 text-primary"
			: "border-border/70 bg-muted/45 text-muted-foreground";
	const pairedClients =
		(data?.paired_clients ?? 0) + (data?.native_paired_clients ?? 0);
	const sessionLabel =
		data && data.active_sessions > 0
			? m.status_sessions_active({ count: data.active_sessions })
			: m.status_no_session();

	return (
		<header className="mb-5 flex flex-col gap-3 border-b border-border/70 pb-4 sm:flex-row sm:items-center sm:justify-between">
			<Link
				to="/host"
				className="group flex min-w-0 items-center gap-2.5 rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:ring-offset-2"
			>
				<span className="flex size-8 shrink-0 items-center justify-center rounded-lg border border-primary/25 bg-primary/10 text-primary">
					<Server className="size-4" aria-hidden />
				</span>
				<span className="min-w-0">
					<span className="block text-[10px] font-semibold uppercase tracking-[0.08em] text-muted-foreground/70">
						{m.nav_host()}
					</span>
					<span className="block truncate text-sm font-semibold text-foreground group-hover:text-primary">
						{host.data?.hostname ?? m.app_name()}
					</span>
					<span className="block truncate text-xs text-muted-foreground">
						{host.data?.local_ip ?? m.app_tagline()}
					</span>
				</span>
			</Link>

			<div className="flex flex-wrap items-center gap-1.5 text-xs">
				<CommandPalette />
				<span
					className={cn(
						"inline-flex min-h-7 items-center gap-1.5 rounded-full border px-2.5 py-1 font-medium",
						statusTone,
					)}
					role="status"
					aria-live="polite"
				>
					<span
						aria-hidden
						className={cn(
							"size-1.5 rounded-full bg-muted-foreground/60",
							status.isError && "bg-destructive",
							isActive && "bg-primary",
						)}
					/>
					{statusLabel}
				</span>
				{data && (
					<>
						<span className="inline-flex min-h-7 items-center gap-1.5 rounded-full border border-border/70 bg-card/70 px-2.5 py-1 text-muted-foreground">
							{sessionLabel}
						</span>
						<span className="inline-flex min-h-7 items-center gap-1.5 rounded-full border border-border/70 bg-card/70 px-2.5 py-1 text-muted-foreground">
							<Users className="size-3.5" aria-hidden />
							<span className="tabular-nums">{pairedClients}</span>
							<span>{m.status_paired_count()}</span>
						</span>
						{data.pin_pending && (
							<Link
								to="/pin"
								className="inline-flex min-h-7 items-center rounded-full border border-[var(--warning)]/40 bg-[var(--warning)]/10 px-2.5 py-1 font-medium text-[var(--warning)] outline-none transition-colors hover:bg-[var(--warning)]/20 focus-visible:ring-2 focus-visible:ring-ring/50"
							>
								{m.status_pin_pending()}
							</Link>
						)}
					</>
				)}
			</div>
		</header>
	);
}

/** Desktop sidebar: the dynamic "Plugins" group, fed by the plugin directory. */
function PluginNavSection() {
	const { data } = usePlugins();
	const plugins = uiPlugins(data);
	if (plugins.length === 0) return null;
	return (
		<motion.div
			initial={{ opacity: 0, y: -4 }}
			animate={{ opacity: 1, y: 0 }}
			transition={{ duration: 0.16 }}
			className="mt-1 flex flex-col gap-0.5"
		>
			<div className="mb-1 border-t border-border/70" />
			<p className={sectionLabelClass}>{m.nav_plugins()}</p>
			{plugins.map((p, index) => {
				const Icon = pluginIcon(p.ui?.icon);
				return (
					// The motion wrapper is a DIV around the link, not `motion(Link)`: wrapping Link
					// erases TanStack's typed `params`, and these entries need `$pluginId`.
					<motion.div
						key={p.id}
						initial={{ opacity: 0, x: -6 }}
						animate={{ opacity: 1, x: 0 }}
						transition={{ duration: 0.14, delay: index * 0.025 }}
						whileTap={{ scale: 0.985 }}
					>
						<Link
							to="/plugins/$pluginId/$"
							params={{ pluginId: p.id, _splat: "" }}
							title={`${p.title} plugin UI.`}
							className={navItemClass}
							activeProps={{ className: navItemActiveClass }}
						>
							<span
								aria-hidden
								className="pointer-events-none absolute inset-y-1 left-0 w-0.5 rounded-full bg-primary opacity-0 [[data-status=active]_&]:opacity-100"
							/>
							<Icon className="relative size-4 shrink-0 opacity-80 [[data-status=active]_&]:opacity-100" />
							<span className="relative min-w-0 flex-1 truncate">
								{p.title}
							</span>
						</Link>
					</motion.div>
				);
			})}
		</motion.div>
	);
}

/** Mobile bottom navigation (< sm): four observatory tabs + a larger, readable More sheet. */
function MobileNav() {
	const [moreOpen, setMoreOpen] = useState(false);
	const pathname = useRouterState({ select: (s) => s.location.pathname });
	const { data } = usePlugins();
	const plugins = uiPlugins(data);
	const moreTriggerRef = useRef<HTMLButtonElement>(null);
	const moreMenuRef = useRef<HTMLDivElement>(null);
	useEffect(() => {
		setMoreOpen(false);
	}, [pathname]);
	useEffect(() => {
		if (!moreOpen) return;
		const menu = moreMenuRef.current;
		const focusFrame = window.requestAnimationFrame(() => {
			const firstFocusable = menu?.querySelector<HTMLElement>(
				'a[href], button:not([disabled]), [tabindex]:not([tabindex="-1"])',
			);
			firstFocusable?.focus();
		});
		const closeOnEscapeOrTrap = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				event.preventDefault();
				setMoreOpen(false);
				return;
			}
			if (event.key !== "Tab" || !menu) return;
			const focusable = Array.from(
				menu.querySelectorAll<HTMLElement>(
					'a[href], button:not([disabled]), [tabindex]:not([tabindex="-1"])',
				),
			).filter((element) => !element.hasAttribute("aria-hidden"));
			if (focusable.length === 0) {
				event.preventDefault();
				return;
			}
			const first = focusable[0]!;
			const last = focusable[focusable.length - 1]!;
			if (event.shiftKey && document.activeElement === first) {
				event.preventDefault();
				last.focus();
			} else if (!event.shiftKey && document.activeElement === last) {
				event.preventDefault();
				first.focus();
			}
		};
		window.addEventListener("keydown", closeOnEscapeOrTrap);
		return () => {
			window.cancelAnimationFrame(focusFrame);
			window.removeEventListener("keydown", closeOnEscapeOrTrap);
			moreTriggerRef.current?.focus();
		};
	}, [moreOpen]);
	// Highlight "More" when the current route lives in the overflow — plugins included.
	const overflowActive =
		pathname.startsWith("/plugins/") ||
		MOBILE_OVERFLOW.some(
			(n) => pathname === n.to || pathname.startsWith(`${n.to}/`),
		);
	const tab =
		"relative flex min-w-0 flex-1 flex-col items-center justify-center gap-0.5 px-0.5 py-2 text-muted-foreground outline-none transition-colors focus-visible:text-foreground focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50";
	const tabActive = "text-primary";
	const lbl =
		"flex min-h-7 w-full items-center justify-center text-center text-[10px] leading-tight";
	const menuItem =
		"flex min-h-14 min-w-0 items-center gap-3 rounded-xl border border-border/70 bg-muted/25 px-3 py-2 text-muted-foreground outline-none transition-colors hover:border-border hover:bg-muted/50 focus-visible:ring-2 focus-visible:ring-ring/50";
	return (
		<>
			<nav
				className="fixed inset-x-0 bottom-0 z-50 sm:hidden"
				style={{ paddingBottom: "env(safe-area-inset-bottom)" }}
				aria-label={m.app_name()}
			>
				<AnimatePresence initial={false}>
					{moreOpen && (
						<>
							<motion.button
								key="mobile-nav-backdrop"
								type="button"
								aria-label={m.nav_close_menu()}
								initial={{ opacity: 0 }}
								animate={{ opacity: 1 }}
								exit={{ opacity: 0 }}
								transition={{ duration: 0.16 }}
								className="fixed inset-0 z-40 bg-black/50"
								onClick={() => setMoreOpen(false)}
							/>
							<motion.div
								key="mobile-nav-menu"
								id="mobile-nav-menu"
								ref={moreMenuRef}
								role="dialog"
								aria-modal="true"
								aria-labelledby="mobile-nav-title"
								initial={{ opacity: 0, y: 10 }}
								animate={{ opacity: 1, y: 0 }}
								exit={{ opacity: 0, y: 10 }}
								transition={{ duration: 0.18 }}
								className="absolute inset-x-0 bottom-full overflow-hidden rounded-t-2xl border border-b-0 border-border/80 bg-card shadow-[0_-8px_24px_rgba(0,0,0,0.35)]"
							>
								<div className="flex items-center justify-between gap-3 px-4 pb-2 pt-3">
									<div className="min-w-0">
										<p
											id="mobile-nav-title"
											className="text-sm font-semibold text-foreground"
										>
											{m.nav_more()}
										</p>
										<p className="truncate text-xs text-muted-foreground">
											{m.app_tagline()}
										</p>
									</div>
									<button
										type="button"
										aria-label={m.nav_close_menu()}
										onClick={() => setMoreOpen(false)}
										className="flex size-9 shrink-0 items-center justify-center rounded-lg text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50"
									>
										<X className="size-4" aria-hidden />
									</button>
								</div>
								<div className="grid max-h-[min(68vh,34rem)] grid-cols-2 gap-2 overflow-y-auto px-3 pb-3">
									{MOBILE_OVERFLOW.map(({ to, icon: Icon, label, help }) => (
										<Link
											key={to}
											to={to}
											title={help()}
											onClick={() => setMoreOpen(false)}
											activeOptions={{ exact: EXACT.includes(to) }}
											className={menuItem}
											activeProps={{
												className: cn(
													menuItem,
													"border-primary/35 bg-primary/10 text-primary",
												),
											}}
										>
											<span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted/55 [[data-status=active]_&]:bg-primary/15">
												<Icon className="size-5" aria-hidden />
											</span>
											<span className="min-w-0 text-left text-xs font-medium leading-snug">
												{label()}
											</span>
										</Link>
									))}
									{plugins.map((p) => {
										const Icon = pluginIcon(p.ui?.icon);
										const help = `${p.title} plugin UI.`;
										return (
											<Link
												key={p.id}
												to="/plugins/$pluginId/$"
												params={{ pluginId: p.id, _splat: "" }}
												title={help}
												onClick={() => setMoreOpen(false)}
												className={menuItem}
												activeProps={{
													className: cn(
														menuItem,
														"border-primary/35 bg-primary/10 text-primary",
													),
												}}
											>
												<span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted/55 [[data-status=active]_&]:bg-primary/15">
													<Icon className="size-5" aria-hidden />
												</span>
												<span className="min-w-0 text-left text-xs font-medium leading-snug">
													{p.title}
												</span>
											</Link>
										);
									})}
									<button
										type="button"
										title={m.action_logout()}
										className={menuItem}
										onClick={() => {
											setMoreOpen(false);
											void (async () => {
												try {
													const res = await fetch("/_auth/logout", {
														method: "POST",
													});
													if (!res.ok)
														throw new Error(`logout failed: ${res.status}`);
													window.location.href = "/login";
												} catch {
													toast.error(m.settings_logout_failed());
												}
											})();
										}}
									>
										<span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted/55">
											<LogOut className="size-5" aria-hidden />
										</span>
										<span className="min-w-0 text-left text-xs font-medium leading-snug">
											{m.action_logout()}
										</span>
									</button>
								</div>
							</motion.div>
						</>
					)}
				</AnimatePresence>
				<div className="grid grid-cols-5 border-t border-border/80 bg-card/95 backdrop-blur-md">
					{MOBILE_PRIMARY.map(({ to, icon: Icon, label, help }) => (
						<Link
							key={to}
							to={to}
							title={help()}
							onClick={() => setMoreOpen(false)}
							activeOptions={{ exact: EXACT.includes(to) }}
							className={tab}
							activeProps={{ className: tabActive }}
						>
							<span
								aria-hidden
								className="absolute top-1 h-0.5 w-4 rounded-full bg-primary opacity-0 [[data-status=active]_&]:opacity-100"
							/>
							<Icon className="size-5 shrink-0" />
							<span className={lbl}>{label()}</span>
						</Link>
					))}
					<button
						type="button"
						ref={moreTriggerRef}
						onClick={() => setMoreOpen((o) => !o)}
						title="Open the rest of the console pages."
						aria-expanded={moreOpen}
						aria-controls="mobile-nav-menu"
						aria-haspopup="dialog"
						aria-current={overflowActive ? "page" : undefined}
						className={cn(tab, (moreOpen || overflowActive) && tabActive)}
					>
						<span
							aria-hidden
							className={cn(
								"absolute top-1 h-0.5 w-4 rounded-full bg-primary opacity-0",
								(moreOpen || overflowActive) && "opacity-100",
							)}
						/>
						<MoreHorizontal className="size-5 shrink-0" />
						<span className={lbl}>{m.nav_more()}</span>
					</button>
				</div>
			</nav>
		</>
	);
}

function LanguageSwitcher() {
	const current = useLocale();
	const helps: Record<Locale, string> = {
		en: "Switch the console language to English. Recommended for most users.",
		de: "Switch the console language to German.",
	};
	return (
		// biome-ignore lint/a11y/useSemanticElements: an aria-labelled role="group" is the right pattern for this small control cluster — no single semantic element fits.
		<div
			className="inline-flex max-w-full flex-wrap gap-0.5 rounded-md border border-border/70 bg-muted/40 p-0.5"
			role="group"
			aria-label={m.settings_language()}
		>
			{locales.map((l: Locale) => (
				<Tooltip key={l}>
					<TooltipTrigger asChild>
						<button
							type="button"
							onClick={() => changeLocale(l)}
							aria-pressed={l === current}
							title={helps[l]}
							className={cn(
								"rounded px-2 py-1 text-[11px] uppercase tracking-wide outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring/50",
								l === current
									? "bg-card text-foreground font-medium shadow-sm"
									: "text-muted-foreground hover:text-foreground",
							)}
						>
							{l}
						</button>
					</TooltipTrigger>
					<TooltipContent side="top">
						{helps[l]}
						{l === "en" ? " Recommended." : ""}
					</TooltipContent>
				</Tooltip>
			))}
		</div>
	);
}

function SignOutButton() {
	const onLogout = async () => {
		try {
			const res = await fetch("/_auth/logout", { method: "POST" });
			if (!res.ok) throw new Error(`logout failed: ${res.status}`);
			window.location.href = "/login";
		} catch {
			toast.error(m.settings_logout_failed());
		}
	};
	return (
		<button
			type="button"
			onClick={() => void onLogout()}
			title={m.action_logout()}
			className="flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-[13px] text-muted-foreground outline-none transition-colors hover:bg-muted/70 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50"
		>
			<LogOut className="size-4 shrink-0 opacity-80" aria-hidden />
			<span className="truncate">{m.action_logout()}</span>
		</button>
	);
}
