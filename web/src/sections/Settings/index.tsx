import Section from "@unom/ui/section";
import { toast } from "@unom/ui/toast";
import { LogOut } from "lucide-react";
import type { FC } from "react";
import { OptionLabel } from "@/components/option-help";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { changeLocale, type Locale, locales, useLocale } from "@/lib/i18n";
import {
	type ThemePreference,
	useThemePreference,
} from "@/lib/theme";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages";

const THEME_OPTIONS: readonly {
	value: ThemePreference;
	label: () => string;
}[] = [
	{ value: "system", label: () => m.settings_theme_system() },
	{ value: "light", label: () => m.settings_theme_light() },
	{ value: "dark", label: () => m.settings_theme_dark() },
];

// Settings reads no host API (locale, theme, logout), so it's a single
// presentational section — no container/view split needed.
export const SectionSettings: FC = () => {
	const current = useLocale();
	const { preference, setPreference } = useThemePreference();

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
								help={m.settings_language_help()}
								recommended="en"
								labelClassName="text-base tracking-tight font-semibold"
							/>
						</CardHeader>
						<CardContent>
							<div
								className="inline-flex flex-wrap rounded-lg border border-border/70 bg-muted/30 p-0.5"
								role="group"
								aria-label={m.settings_language()}
							>
								{locales.map((l: Locale) => (
									<Button
										key={l}
										variant={l === current ? "secondary" : "ghost"}
										size="sm"
										className="h-8 min-h-8 uppercase"
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
								label={m.settings_theme()}
								help={m.settings_theme_help()}
								recommended={m.settings_theme_system()}
								labelClassName="text-base tracking-tight font-semibold"
							/>
						</CardHeader>
						<CardContent>
							<div
								className="inline-flex flex-wrap rounded-lg border border-border/70 bg-muted/30 p-0.5"
								role="group"
								aria-label={m.settings_theme()}
							>
								{THEME_OPTIONS.map(({ value, label }) => (
									<Button
										key={value}
										variant={preference === value ? "secondary" : "ghost"}
										size="sm"
										className={cn("h-8 min-h-8")}
										aria-pressed={preference === value}
										onClick={() => setPreference(value)}
									>
										{label()}
									</Button>
								))}
							</div>
						</CardContent>
					</Card>

					<Card>
						<CardHeader className="space-y-1">
							<OptionLabel
								label={m.action_logout()}
								help={m.settings_logout_help()}
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
