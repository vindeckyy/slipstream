import Section from "@unom/ui/section";
import type { FC, ReactNode } from "react";
import { m } from "@/paraglide/messages";

/**
 * The Pairing page LAYOUT — the single source of how the four sub-cards are arranged. Both the live
 * page (`index.tsx`, slots = the self-contained `*Section` containers) and Storybook (slots = the
 * pure cards with mock state) fill these slots, so the arrangement can never drift between them.
 */
export const PairingView: FC<{
	pending: ReactNode;
	native: ReactNode;
	moonlight: ReactNode;
	paired: ReactNode;
}> = ({ pending, native, moonlight, paired }) => (
	<Section maxWidth={false}>
		<div className="flex flex-col gap-card">
			<div className="space-y-1">
				<h1 className="text-2xl font-semibold tracking-tight">
					{m.pairing_title()}
				</h1>
			</div>

			{pending}
			<div className="flex flex-col gap-card lg:grid lg:grid-cols-2">
				{native}
				{moonlight}
			</div>
			{paired}
		</div>
	</Section>
);
