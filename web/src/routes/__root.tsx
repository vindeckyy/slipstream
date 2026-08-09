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
			{ rel: "manifest", href: "/manifest.webmanifest" },
		],
	}),
	component: RootComponent,
});

function RootComponent() {
	useEffect(() => {
		adoptStoredLocale();
	}, []);
	const isDarkTheme = useDarkTheme();
	const locale = useLocale();
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
				<MotionConfig reducedMotion="user">
					{isAuthScreen ? (
						<Outlet />
					) : (
						<AppShell>
							<Outlet />
						</AppShell>
					)}
				</MotionConfig>
				{/* Sonner toaster (lazy client-side)  -  success feedback for auto-saved settings. */}
				<Toaster />
				<Scripts />
			</body>
		</html>
	);
}
