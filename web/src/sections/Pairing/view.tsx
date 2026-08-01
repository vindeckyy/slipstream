import Section from "@unom/ui/section";
import type { FC, ReactNode } from "react";
import { HelpTip, RecommendedMark } from "@/components/option-help";
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
				<div className="flex items-center gap-1.5">
					<h1 className="text-2xl font-semibold tracking-tight">
						{m.pairing_title()}
					</h1>
					<HelpTip
						label={m.pairing_title()}
						text="Admit Slipstream apps and Moonlight clients to this host once. After pairing they reconnect without a PIN."
					/>
				</div>
				<RecommendedMark value="Slipstream app: use Pair a device, then type the PIN on the client. Moonlight: start Pair on the client, then enter its PIN in the Moonlight card." />
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
