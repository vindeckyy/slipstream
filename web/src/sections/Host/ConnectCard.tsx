import { Check, Copy, Smartphone } from "lucide-react";
import { type FC, useState } from "react";
import type { HostInfo } from "@/api/gen/model/hostInfo";
import { HelpTip, OptionLabel } from "@/components/option-help";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { m } from "@/paraglide/messages";

/**
 * "Get a device onto this host" — the address to type, and the deep link that skips typing it.
 *
 * The console knew the host's identity and local address all along and never offered either in a
 * form you could hand to a phone: pairing meant reading an IP off the Host page and retyping it on
 * a couch. `slipstream://connect/<unique_id>` is the shipped client grammar
 * (clients/shared/deeplink-vectors.json — the Rust, Swift and Kotlin parsers all test against it),
 * so a client that is already installed opens straight onto this host.
 *
 * No QR code: rendering one needs an encoder we do not bundle, and a wrong QR is worse than none.
 * The link is short enough to send over any chat app, which is what people actually do.
 */
export const ConnectCard: FC<{ host: HostInfo }> = ({ host }) => {
	const deepLink = `slipstream://connect/${host.uniqueid}`;
	return (
		<Card>
			<CardHeader>
				<CardTitle className="flex items-center gap-2 tracking-tight">
					<Smartphone className="size-4 text-muted-foreground" />
					{m.connect_title()}
					<HelpTip
						label={m.connect_title()}
						text="Share the address or deep link so a phone, TV, or PC can find this host. Pairing still happens on the Pairing page."
					/>
				</CardTitle>
				<CardDescription className="max-w-prose leading-relaxed">
					{m.connect_help()}
				</CardDescription>
			</CardHeader>
			<CardContent className="space-y-4">
				<CopyRow
					label={m.connect_address()}
					value={host.local_ip}
					help="LAN address to type into a Slipstream or Moonlight-compatible client on the same network."
					recommended="Use this when typing by hand or the deep link is unavailable"
				/>
				<CopyRow
					label={m.connect_link()}
					value={deepLink}
					help="Opens the Slipstream client straight onto this host when the app is already installed. Send it over chat or open it on the device."
					recommended="Prefer this when the Slipstream client is installed"
				/>
			</CardContent>
		</Card>
	);
};

/** One labelled, monospaced value with a copy button — the point of the card. */
const CopyRow: FC<{
	label: string;
	value: string;
	help: string;
	recommended: string;
}> = ({ label, value, help, recommended }) => {
	const [copied, setCopied] = useState(false);
	const copy = async () => {
		try {
			await navigator.clipboard.writeText(value);
			setCopied(true);
			// Revert the affordance rather than leaving a permanent tick, which would stop reading as
			// feedback the second time you press it.
			setTimeout(() => setCopied(false), 1500);
		} catch {
			// Clipboard denied (insecure origin, or the user said no) — the value is on screen and
			// selectable, so there is nothing worth interrupting them about.
		}
	};
	return (
		<div className="space-y-1.5">
			<OptionLabel label={label} help={help} recommended={recommended} />
			<div className="flex items-center gap-2">
				<code className="min-w-0 flex-1 truncate rounded-md border border-border/60 bg-muted/50 px-3 py-2.5 font-mono text-xs">
					{value}
				</code>
				<Button
					variant="outline"
					size="icon"
					aria-label={m.connect_copy()}
					title={`Copy ${label.toLowerCase()} to the clipboard`}
					onClick={copy}
					className="shrink-0"
				>
					{copied ? (
						<Check className="size-4 text-[var(--success)]" />
					) : (
						<Copy className="size-4" />
					)}
				</Button>
				<HelpTip
					label={m.connect_copy()}
					text={`Copies the ${label.toLowerCase()} so you can paste it into a client or send it to another device.`}
				/>
			</div>
		</div>
	);
};
