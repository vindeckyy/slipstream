// Import the console's REAL stylesheet directly (rememed-style) — the @theme
// blocks process because this is the literal entry Storybook's Vite pipeline sees.
import "../src/styles.css";
// The console loads its brand typeface separately (in __root.tsx); do the same
// here or every story falls back to system-ui and looks off.
import "@fontsource-variable/geist";
import { definePreview } from "@storybook/react-vite";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { defaultMaterialTheme, MaterialProvider } from "@unom/ui/material";
import Section from "@unom/ui/section";
import { MotionConfig } from "motion/react";
import { useEffect } from "react";

// AppShell subscribes to the host event stream. Storybook has no host, so keep that subscription
// local to the preview instead of opening a real SSE connection.
class OfflineEventSource extends EventTarget {
	readonly url: string;
	readonly withCredentials = false;
	readonly readyState = 2;

	constructor(url: string | URL) {
		super();
		this.url = String(url);
	}

	close(): void {
		return;
	}
}

if (typeof window !== "undefined") {
	window.EventSource = OfflineEventSource as unknown as typeof EventSource;
}

// React Query is present so any query-backed component mounts without a real host. Seed the one
// query AppShell owns so its plugin nav stays deterministic and never reaches the host API.
const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			retry: false,
			staleTime: Number.POSITIVE_INFINITY,
			refetchOnReconnect: false,
			refetchOnWindowFocus: false,
		},
		mutations: { retry: false },
	},
});
queryClient.setQueryData(["plugins"], []);

export default definePreview({
	addons: [],
	// The live console pins dark; default the canvas to dark too, with toolbar switches for the
	// palette and the motion policy used by screenshot capture.
	initialGlobals: { theme: "dark", motion: "full" },
	globalTypes: {
		theme: {
			description: "Light/dark color scheme",
			toolbar: {
				title: "Theme",
				icon: "circlehollow",
				items: [
					{ value: "dark", icon: "moon", title: "Dark" },
					{ value: "light", icon: "sun", title: "Light" },
				],
				dynamicTitle: true,
			},
		},
		motion: {
			description: "Motion policy",
			toolbar: {
				title: "Motion",
				icon: "play",
				items: [
					{ value: "full", icon: "play", title: "Full motion" },
					{ value: "reduced", icon: "stop", title: "Reduced motion" },
				],
				dynamicTitle: true,
			},
		},
	},
	decorators: [
		(Story, context) => {
			const dark = (context.globals.theme as string) !== "light";
			const reducedMotion = (context.globals.motion as string) === "reduced";
			// `layout: 'fullscreen'` stories (e.g. the AppShell) own their own padding;
			// everything else gets a comfortable inset.
			const fullscreen = context.parameters.layout === "fullscreen";
			// Mirror `.dark` onto <html> so the body's token-driven background AND any
			// portal-mounted content (radix dialogs, popovers) pick up the right
			// palette — the console keys its whole token set off `html.dark`.
			useEffect(() => {
				document.documentElement.classList.toggle("dark", dark);
				document.documentElement.dataset.motion = reducedMotion
					? "reduced"
					: "full";
			}, [dark, reducedMotion]);
			return (
				<QueryClientProvider client={queryClient}>
					<MaterialProvider theme={defaultMaterialTheme}>
						<MotionConfig reducedMotion={reducedMotion ? "always" : "never"}>
							<div className={dark ? "dark" : ""}>
								<Section maxWidth={false}>
									<div
										className={`min-h-screen bg-background text-foreground ${fullscreen ? "" : "p-6"}`}
									>
										<Story />
									</div>
								</Section>
							</div>
						</MotionConfig>
					</MaterialProvider>
				</QueryClientProvider>
			);
		},
	],
	parameters: {
		controls: { matchers: { color: /(background|color)$/i, date: /Date$/ } },
		layout: "padded",
	},
});
