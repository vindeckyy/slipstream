import Section from "@unom/ui/section";
import type { FC, ReactNode } from "react";
import { m } from "@/paraglide/messages";

const LOGS_TITLE_ID = "logs-page-title";
const LOGS_SUMMARY_ID = "logs-page-summary";
const LOGS_VIEWER_ID = "logs-viewer";

/**
 * The Logs page LAYOUT — the live page (`index.tsx`) and the Storybook story fill the single
 * `viewer` slot, so the arrangement can never drift between them (same pattern as StatsView).
 */
export const LogsView: FC<{ viewer: ReactNode }> = ({ viewer }) => (
	<Section maxWidth={false}>
		<div className="flex min-w-0 flex-col gap-card">
			<header
				className="space-y-1.5 border-b border-border/60 pb-4"
				aria-labelledby={LOGS_TITLE_ID}
				aria-describedby={LOGS_SUMMARY_ID}
			>
				<h1 id={LOGS_TITLE_ID} className="text-2xl font-semibold tracking-tight">
					{m.logs_title()}
				</h1>
				<p id={LOGS_SUMMARY_ID} className="max-w-prose text-sm text-muted-foreground">
					{m.logs_subtitle()}
				</p>
			</header>

			<section
				aria-labelledby={LOGS_VIEWER_ID}
				aria-describedby={LOGS_SUMMARY_ID}
				className="min-w-0"
			>
				<h2 id={LOGS_VIEWER_ID} className="sr-only">
					{m.logs_title()}
				</h2>
				{viewer}
			</section>
		</div>
	</Section>
);
