import Section from "@unom/ui/section";
import { toast } from "@unom/ui/toast";
import { LogOut } from "lucide-react";
import type { FC } from "react";
import { OptionLabel } from "@/components/option-help";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { changeLocale, type Locale, locales, useLocale } from "@/lib/i18n";
import { m } from "@/paraglide/messages";

// Settings reads no API (just the locale + a logout button), so it's a single
// presentational section — no container/view split needed.
export const SectionSettings: FC = () => {
	const current = useLocale();

	const onLogout = async () => {
		try {
			const res = await fetch("/_auth/logout", { method: "POST" });
			if (!res.ok) throw new Error(`logout failed: ${res.status}`);
			window.location.href = "/login";
		} catch {
			// The logout POST failed, so the session cookie likely survives. Navigating to /login
			// anyway would look logged out while a live session still re-admits on the next gated
			// nav — surface the failure and stay put so the user can retry.
			toast.error(m.settings_logout_failed());
		}
	};

	return (
		<Section maxWidth={false}>
			<div className="flex flex-col gap-card">
				<header className="space-y-1.5 border-b border-border/60 pb-4">
					<h1 className="text-2xl font-semibold tracking-tight">
						{m.settings_title()}
					</h1>
				</header>

				<div className="grid max-w-2xl gap-card">
					<Card>
						<CardHeader className="space-y-1">
							<OptionLabel
								label={m.settings_language()}
								help="Chooses the language for labels and messages in this management console."
								recommended="en"
								labelClassName="text-base tracking-tight font-semibold"
							/>
						</CardHeader>
						<CardContent>
							<div className="inline-flex flex-wrap rounded-lg border border-border/70 bg-muted/30 p-0.5">
								{locales.map((l: Locale) => (
									<Button
										key={l}
										variant={l === current ? "secondary" : "ghost"}
										size="sm"
										className="h-8 uppercase"
										aria-pressed={l === current}
										onClick={() => changeLocale(l)}
									>
										{l}
									</Button>
								))}
							</div>
						</CardContent>
					</Card>

					<Card>
						<CardHeader className="space-y-1">
							<OptionLabel
								label={m.action_logout()}
								help="Clears your signed-in session in this browser. You will need the console password to open Settings and other management pages again."
								labelClassName="text-base tracking-tight font-semibold"
							/>
						</CardHeader>
						<CardContent>
							<Button variant="outline" onClick={onLogout}>
								<LogOut className="size-4" />
								{m.action_logout()}
							</Button>
						</CardContent>
					</Card>
				</div>
			</div>
		</Section>
	);
};
