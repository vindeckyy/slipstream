/// <reference types="vite/client" />

import type { QueryClient } from "@tanstack/react-query";
import {
	createRootRouteWithContext,
	HeadContent,
	Outlet,
	Scripts,
	useRouterState,
} from "@tanstack/react-router";
import "@fontsource-variable/geist";
import { Toaster } from "@unom/ui/toast";
import { MotionConfig } from "motion/react";
import { useEffect, useSyncExternalStore } from "react";
import { AppShell } from "@/components/app-shell";
import { adoptStoredLocale, useLocale } from "@/lib/i18n";
import appCss from "@/styles.css?url";

type ThemePreference = "dark" | "light" | "system";

const themeStorageKey = "slipstream-theme";
const darkThemeMediaQuery = "(prefers-color-scheme: dark)";

// Run before the stylesheet can paint so the selected palette is present on first paint.
const themeBootScript = `(() => {
  let stored = null;
  try {
    stored = window.localStorage.getItem("${themeStorageKey}");
  } catch {}

  const preference =
    stored === "dark" || stored === "light" || stored === "system"
      ? stored
      : "system";
  const dark =
    preference === "dark" ||
    (preference === "system" &&
      (typeof window.matchMedia !== "function" ||
        window.matchMedia("${darkThemeMediaQuery}").matches));

  document.documentElement.classList.toggle("dark", dark);
  document.documentElement.style.colorScheme = dark ? "dark" : "light";
})();`;

function readThemePreference(): ThemePreference {
	if (typeof window === "undefined") return "system";
	try {
		const stored = window.localStorage.getItem(themeStorageKey);
		if (stored === "dark" || stored === "light" || stored === "system") {
			return stored;
		}
	} catch {
		// Private browsing and blocked storage should not prevent the app from booting.
	}
	return "system";
}

function resolveDarkTheme(): boolean {
	if (typeof window === "undefined") return true;
	const preference = readThemePreference();
	if (preference === "dark") return true;
	if (preference === "light") return false;
	return (
		typeof window.matchMedia !== "function" ||
		window.matchMedia(darkThemeMediaQuery).matches
	);
}

function subscribeToTheme(onStoreChange: () => void) {
	if (typeof window === "undefined") return () => {};

	const media = window.matchMedia?.(darkThemeMediaQuery);
	media?.addEventListener("change", onStoreChange);
	window.addEventListener("storage", onStoreChange);

	return () => {
		media?.removeEventListener("change", onStoreChange);
		window.removeEventListener("storage", onStoreChange);
	};
}

function useDarkTheme() {
	return useSyncExternalStore(subscribeToTheme, resolveDarkTheme, () => true);
}

export interface RouterContext {
	queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RouterContext>()({
	head: () => ({
		meta: [
			{ charSet: "utf-8" },
			{ name: "viewport", content: "width=device-width, initial-scale=1" },
			{ name: "color-scheme", content: "dark light" },
			{ name: "theme-color", content: "#0891b2" },
			{ name: "apple-mobile-web-app-capable", content: "yes" },
			{ name: "apple-mobile-web-app-title", content: "Slipstream" },
			{ title: "Slipstream" },
		],
		links: [
			{ rel: "stylesheet", href: appCss },
			{ rel: "icon", type: "image/png", href: "/favicon-mark.png" },
			// Installable on a phone — this console is used from a couch as often as from a desk,
			// and a home-screen launcher beats retyping a LAN IP. Standalone display, no service
			// worker: an offline shell for a console whose every screen is live host state would
			// only ever show stale numbers convincingly.
			{ rel: "manifest", href: "/manifest.webmanifest" },
		],
	}),
	component: RootComponent,
});

function RootComponent() {
	// Adopt the persisted/browser locale AFTER hydration — the initial render stays at the base
	// locale to match SSR (see lib/i18n.ts), so this is the single, mismatch-free locale switch.
	useEffect(() => {
		adoptStoredLocale();
	}, []);
	const isDarkTheme = useDarkTheme();
	// `lang` must track the locale the page is actually rendered in — it is what tells a screen
	// reader which pronunciation to use, and it was pinned to "en" while the app switched to German
	// underneath it. `adoptStoredLocale` also sets it on the live document; this keeps SSR honest.
	const locale = useLocale();
	// The auth screens render bare (no sidebar); everything else gets the app shell.
	const isAuthScreen = useRouterState({
		select: (s) =>
			s.location.pathname === "/login" || s.location.pathname === "/setup",
	});
	return (
		<html
			lang={locale}
			className={isDarkTheme ? "dark" : undefined}
			style={{ colorScheme: isDarkTheme ? "dark" : "light" }}
			suppressHydrationWarning
		>
			<head>
				<script>{themeBootScript}</script>
				<HeadContent />
			</head>
			<body className="min-h-screen">
				{/* Motion defaults to `reducedMotion: "never"`, so every card, nav item and button
				    animated at full strength even for someone whose OS asks for less. "user" honours
				    the OS setting. */}
				<MotionConfig reducedMotion="user">
					{isAuthScreen ? (
						<Outlet />
					) : (
						<AppShell>
							<Outlet />
						</AppShell>
					)}
				</MotionConfig>
				{/* Sonner toaster (lazy client-side) — success feedback for auto-saved settings. */}
				<Toaster />
				<Scripts />
			</body>
		</html>
	);
}
