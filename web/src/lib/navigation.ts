import {
	AppWindow,
	GaugeCircle,
	Home,
	KeyRound,
	MonitorPlay,
	Puzzle,
	Server,
	Settings,
	SlidersHorizontal,
	Video,
	Workflow,
	Wrench,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { m } from "@/paraglide/messages";

/** Stable ids for the static console destinations. */
export type NavDestinationId =
	| "dashboard"
	| "sessions"
	| "pairing"
	| "library"
	| "displays"
	| "host"
	| "stats"
	| "logs"
	| "automation"
	| "config"
	| "settings"
	| "plugins";

export type NavGroupId = "watch" | "connect" | "host" | "tools";

export type NavDestination = {
	id: NavDestinationId;
	/** Canonical path used by the shell and command palette. */
	to: string;
	/** Older paths that still resolve; kept for bookmarks and deep links. */
	aliases?: readonly string[];
	icon: LucideIcon;
	label: () => string;
	help: () => string;
	keywords: readonly string[];
	/** Surfaces in the command palette "Common" badge. */
	common?: boolean;
	/** Exact active match (home and plugin store index). */
	exact?: boolean;
};

export type NavGroup = {
	id: NavGroupId;
	label: () => string;
	itemIds: readonly NavDestinationId[];
};

/**
 * One registry for desktop rail, mobile nav, and the command palette.
 * Compatibility aliases (`/pin`, `/apps`, `/troubleshoot`) stay as routes; links use the
 * canonical destinations below.
 */
export const NAV_DESTINATIONS: readonly NavDestination[] = [
	{
		id: "dashboard",
		to: "/",
		icon: Home,
		label: () => m.nav_dashboard(),
		help: () => m.nav_dashboard_help(),
		keywords: ["dashboard", "overview", "status", "live"],
		common: true,
		exact: true,
	},
	{
		id: "sessions",
		to: "/sessions",
		icon: Video,
		label: () => m.nav_sessions(),
		help: () => m.nav_sessions_help(),
		keywords: ["session", "stream", "play", "active", "sessions"],
	},
	{
		id: "pairing",
		to: "/pairing",
		aliases: ["/pin"],
		icon: KeyRound,
		label: () => m.nav_pairing(),
		help: () => m.nav_pairing_help(),
		keywords: ["pair", "pin", "client", "pairing"],
		common: true,
	},
	{
		id: "library",
		to: "/library",
		aliases: ["/apps"],
		icon: AppWindow,
		label: () => m.nav_library(),
		help: () => m.nav_library_help(),
		keywords: ["library", "game", "app", "apps"],
		common: true,
	},
	{
		id: "displays",
		to: "/displays",
		icon: MonitorPlay,
		label: () => m.nav_displays(),
		help: () => m.nav_displays_help(),
		keywords: ["display", "screen", "virtual", "displays"],
	},
	{
		id: "host",
		to: "/host",
		icon: Server,
		label: () => m.nav_host(),
		help: () => m.nav_host_help(),
		keywords: ["host", "gpu", "update", "system", "restart", "shutdown"],
	},
	{
		id: "stats",
		to: "/stats",
		icon: GaugeCircle,
		label: () => m.nav_stats(),
		help: () => m.nav_stats_help(),
		keywords: ["stats", "capture", "recording", "monitor", "performance"],
	},
	{
		id: "logs",
		to: "/logs",
		aliases: ["/troubleshoot"],
		icon: Wrench,
		label: () => m.nav_logs(),
		help: () => m.nav_logs_help(),
		keywords: ["logs", "troubleshoot", "debug", "export"],
	},
	{
		id: "automation",
		to: "/automation",
		icon: Workflow,
		label: () => m.nav_automation(),
		help: () => m.nav_automation_help(),
		keywords: ["automation", "hook", "webhook"],
	},
	{
		id: "config",
		to: "/config",
		icon: SlidersHorizontal,
		label: () => m.nav_config(),
		help: () => m.nav_config_help(),
		keywords: [
			"config",
			"configuration",
			"capture",
			"encoder",
			"network",
			"input",
		],
		common: true,
	},
	{
		id: "settings",
		to: "/settings",
		icon: Settings,
		label: () => m.nav_settings(),
		help: () => m.nav_settings_help(),
		keywords: ["settings", "language", "theme", "logout", "sign out"],
	},
	{
		id: "plugins",
		to: "/plugins",
		icon: Puzzle,
		label: () => m.nav_plugin_store(),
		help: () => m.nav_plugin_store_help(),
		keywords: ["plugin", "store", "install", "package"],
		exact: true,
	},
];

const byId = Object.fromEntries(
	NAV_DESTINATIONS.map((destination) => [destination.id, destination]),
) as Record<NavDestinationId, NavDestination>;

export function navDestination(id: NavDestinationId): NavDestination {
	return byId[id];
}

/** Desktop rail groups: Watch / Connect / Host / Tools (not Library as a heading). */
export const NAV_GROUPS: readonly NavGroup[] = [
	{
		id: "watch",
		label: () => m.nav_watch(),
		itemIds: ["dashboard", "sessions", "stats", "logs"],
	},
	{
		id: "connect",
		label: () => m.nav_connect(),
		itemIds: ["pairing", "library", "displays"],
	},
	{
		id: "host",
		label: () => m.nav_host(),
		itemIds: ["host", "automation"],
	},
	{
		id: "tools",
		label: () => m.nav_tools(),
		itemIds: ["config", "settings", "plugins"],
	},
];

/** Mobile bottom-nav primaries: Dashboard, Library, Host, Pairing (+ More). */
export const MOBILE_PRIMARY_IDS: readonly NavDestinationId[] = [
	"dashboard",
	"library",
	"host",
	"pairing",
];

/** Mobile More sheet: everything else from the static registry. */
export const MOBILE_OVERFLOW_IDS: readonly NavDestinationId[] = [
	"sessions",
	"stats",
	"logs",
	"displays",
	"automation",
	"config",
	"settings",
	"plugins",
];

export function destinationsFor(
	ids: readonly NavDestinationId[],
): NavDestination[] {
	return ids.map((id) => byId[id]);
}

/** True when `pathname` is this destination or one of its aliases. */
export function pathMatchesDestination(
	pathname: string,
	destination: NavDestination,
): boolean {
	const paths = [destination.to, ...(destination.aliases ?? [])];
	return paths.some((path) => {
		if (destination.exact) return pathname === path;
		return pathname === path || pathname.startsWith(`${path}/`);
	});
}
