import Section from "@unom/ui/section";
import type { FC, ReactNode } from "react";
import { m } from "@/paraglide/messages";

const STATS_TITLE_ID = "stats-page-title";
const STATS_SUMMARY_ID = "stats-page-summary";
const STATS_CAPTURE_ID = "stats-capture";
const STATS_LIVE_ID = "stats-live";
const STATS_RECORDINGS_ID = "stats-recordings";
const STATS_DETAIL_ID = "stats-recording-detail";

/**
 * The Performance page LAYOUT — the single source of how the cards stack. Both the live page
 * (`index.tsx`, slots = the self-contained `*Section` containers) and Storybook (slots = the pure
 * cards with mock state) fill these slots, so the arrangement can never drift between them. `live`
 * and `detail` are nullable slots — the page passes them only when armed / a recording is selected.
 */
export const StatsView: FC<{
	control: ReactNode;
	live: ReactNode;
	recordings: ReactNode;
	detail: ReactNode;
}> = ({ control, live, recordings, detail }) => (
	<Section maxWidth={false}>
		<div className="flex min-w-0 flex-col gap-card">
			<header
				className="space-y-1.5 border-b border-border/60 pb-4"
				aria-labelledby={STATS_TITLE_ID}
				aria-describedby={STATS_SUMMARY_ID}
			>
				<h1 id={STATS_TITLE_ID} className="text-2xl font-semibold tracking-tight">
					{m.stats_title()}
				</h1>
				<p id={STATS_SUMMARY_ID} className="max-w-prose text-sm text-muted-foreground">
					{m.stats_subtitle()}
				</p>
			</header>

			<div
				className={
					live
						? "grid min-w-0 items-start gap-card xl:grid-cols-[minmax(17rem,0.34fr)_minmax(0,0.66fr)]"
						: "min-w-0"
				}
			>
				<section aria-labelledby={STATS_CAPTURE_ID} className="min-w-0">
					<h2 id={STATS_CAPTURE_ID} className="sr-only">
						{m.stats_capture_title()}
					</h2>
					{control}
				</section>
				{live && (
					<section
						aria-labelledby={STATS_LIVE_ID}
						className="min-w-0 border-l-2 border-primary/30 pl-3 sm:pl-4"
					>
						<h2 id={STATS_LIVE_ID} className="sr-only">
							{m.stats_live_title()}
						</h2>
						{live}
					</section>
				)}
			</div>

			<section
				aria-labelledby={STATS_RECORDINGS_ID}
				className="min-w-0 space-y-card border-t border-border/60 pt-card"
			>
				<h2 id={STATS_RECORDINGS_ID} className="sr-only">
					{m.stats_recordings_title()}
				</h2>
				{recordings}
				{detail && (
					<section
						aria-labelledby={STATS_DETAIL_ID}
						className="min-w-0 border-t border-border/60 pt-card"
					>
						<h3 id={STATS_DETAIL_ID} className="sr-only">
							{m.stats_detail_title()}
						</h3>
						{detail}
					</section>
				)}
			</section>
		</div>
	</Section>
);
