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
import { useEffect } from "react";
import { AppShell } from "@/components/app-shell";
import { adoptStoredLocale, useLocale } from "@/lib/i18n";
import { themeBootScriptSource, useDarkTheme } from "@/lib/theme";
import appCss from "@/styles.css?url";

const themeBootScript = themeBootScriptSource();

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
				{/* Direction contract (impeccable seed 802fa826): a broadcast control room fused with
				    a creator-hardware desk instrument. THESIS: a streaming host console that reads as a
				    physical broadcast desk — signal meters, program/preview monitors, keycap controls —
				    refusing the generic SaaS cyan-card dashboard. OWN-WORLD: deep charcoal gunmetal
				    ground, bone/putty keycap surfaces, the Slipstream ASCII blocky mark as a silkscreened
				    chassis label, cyan reserved for the live/on-air signal, one safety-orange action per
				    surface, amber as the always-on status glow. STORY: the operator sees the host as a
				    broadcast desk — what is on air, what is ready, and one obvious action — and the
				    console feels like a physical instrument, not a web template. FIRST VIEWPORT: a
				    gunmetal chassis frame with the ASCII wordmark as a brushed label, a program/preview
				    monitor pair for stream state, keycap nav, signal-meter status tiles, and the amber
				    status line. FORM: broadcast control room (structure) fused with creator-hardware desk
				    (material), seed key 802fa826. FINISH: unreviewed and undocumented is unfinished; this
				    build ends with the finish review, the verdict, and DESIGN.md. */}
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
