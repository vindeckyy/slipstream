import { ease } from "@unom/style";
import { motion } from "motion/react";
import { type FC, useState } from "react";
import Logo from "@/components/logo";
import { OptionLabel } from "@/components/option-help";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { TooltipProvider } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages";

const ERROR_ID = "login-password-error";

export const LoginView: FC<{
	onSubmit: (password: string) => void;
	error: boolean;
	busy: boolean;
}> = ({ onSubmit, error, busy }) => {
	const [password, setPassword] = useState("");
	const canSubmit = password.length > 0 && !busy;

	return (
		<TooltipProvider>
			<div className="relative flex min-h-svh flex-col items-center justify-center overflow-hidden bg-background px-4 py-10 sm:px-6 sm:py-16">
				{/* Desk atmosphere: the amber instrument lamp + a faint chassis grain (not a
				    cyan SaaS wash). See DESIGN.md. */}
				<div
					aria-hidden
					className="pointer-events-none absolute inset-0"
					style={{
						background: `
						radial-gradient(ellipse 80% 50% at 50% -20%, color-mix(in oklab, var(--ss-status-light) 16%, transparent), transparent 70%),
						repeating-linear-gradient(0deg, transparent 0px, transparent 2px, color-mix(in oklab, var(--foreground) 1.4%, transparent) 2px, color-mix(in oklab, var(--foreground) 1.4%, transparent) 3px)
					`,
					}}
				/>

				<motion.div
					initial="from"
					animate="enter"
					transition={ease.quint(0.7).out}
					variants={{
						enter: { opacity: 1, y: 0 },
						from: { opacity: 0, y: 10 },
					}}
					className="relative z-10 flex w-full max-w-[22.5rem] flex-col items-center"
				>
					<div className="mb-8 w-[19rem] sm:mb-10 sm:w-[20rem]">
						<Logo />
					</div>

					<Card
						className={cn(
							"w-full shadow-[0_18px_48px_-24px_rgba(0,0,0,0.65)]",
							error ? "ring-destructive/50" : "ring-accent/30",
						)}
					>
						<CardHeader className="space-y-2 pb-4 sm:pb-5">
							<CardTitle className="text-2xl tracking-tight">
								{m.login_title()}
							</CardTitle>
							<CardDescription className="text-sm leading-relaxed text-muted-foreground">
								{m.login_subtitle()}{" "}
								<a
									href="https://github.com/vindeckyy/slipstream/blob/main/docs-site/content/docs/forgot-password.md"
									target="_blank"
									rel="noreferrer"
									className="font-medium text-foreground/90 underline decoration-foreground/25 underline-offset-4 transition-colors hover:text-foreground hover:decoration-foreground/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-card"
								>
									{m.login_docs_link()}
								</a>
							</CardDescription>
						</CardHeader>

						<CardContent>
							<form
								onSubmit={(e) => {
									e.preventDefault();
									onSubmit(password);
								}}
								className="flex flex-col gap-5"
								aria-busy={busy || undefined}
							>
								<div className="flex flex-col gap-2">
									<OptionLabel
										htmlFor="pw"
										label={m.login_password()}
										help="The management password created during first-time setup. Signs you into this console only."
									/>
									<Input
										id="pw"
										name="password"
										type="password"
										autoFocus
										autoComplete="current-password"
										value={password}
										disabled={busy}
										aria-invalid={error || undefined}
										aria-describedby={error ? ERROR_ID : undefined}
										onChange={(e) => setPassword(e.target.value)}
										className={
											error
												? "border-destructive/70 focus-visible:ring-destructive/40"
												: undefined
										}
									/>
									{error && (
										<p
											id={ERROR_ID}
											role="alert"
											className="text-sm font-medium text-destructive"
										>
											{m.login_error()}
										</p>
									)}
								</div>

								{/* The one commit action on this surface: safety orange. */}
								<Button
									type="submit"
									className="mt-1 w-full bg-[var(--ss-action)] text-white hover:bg-[var(--ss-action-light)]"
									disabled={!canSubmit}
									aria-busy={busy || undefined}
								>
									{busy ? m.login_signing_in() : m.login_submit()}
								</Button>
							</form>
						</CardContent>
					</Card>
				</motion.div>
			</div>
		</TooltipProvider>
	);
};
