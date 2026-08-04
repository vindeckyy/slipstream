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

const ERROR_ID = "setup-password-error";

export type SetupError = "too-short" | "mismatch" | "unavailable";

export const SetupView: FC<{
	onSubmit: (password: string, confirmation: string) => void;
	error: SetupError | null;
	busy: boolean;
}> = ({ onSubmit, error, busy }) => {
	const [password, setPassword] = useState("");
	const [confirmation, setConfirmation] = useState("");
	const canSubmit = password.length > 0 && confirmation.length > 0 && !busy;
	const errorMessage =
		error === "too-short"
			? m.setup_error_too_short()
			: error === "mismatch"
				? m.setup_error_mismatch()
				: error === "unavailable"
					? m.setup_error_unavailable()
					: null;

	return (
		<TooltipProvider>
			<div className="relative flex min-h-svh flex-col items-center justify-center overflow-hidden bg-background px-4 py-10 sm:px-6 sm:py-16">
				<div
					aria-hidden
					className="pointer-events-none absolute inset-0"
					style={{
						background: `
						radial-gradient(ellipse 80% 50% at 50% -20%, color-mix(in oklab, var(--ss-brand) 18%, transparent), transparent 70%),
						radial-gradient(ellipse 60% 40% at 50% 120%, color-mix(in oklab, var(--ss-brand-light) 8%, transparent), transparent 65%)
					`,
					}}
				/>
				<div
					aria-hidden
					className="pointer-events-none absolute inset-0 opacity-[0.35]"
					style={{
						backgroundImage:
							"radial-gradient(color-mix(in oklab, var(--foreground) 6%, transparent) 1px, transparent 1px)",
						backgroundSize: "24px 24px",
						maskImage:
							"radial-gradient(ellipse 70% 60% at 50% 40%, black 20%, transparent 75%)",
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
								{m.setup_title()}
							</CardTitle>
							<CardDescription className="text-sm leading-relaxed text-muted-foreground">
								{m.setup_subtitle()}
							</CardDescription>
							<p className="text-sm leading-relaxed text-muted-foreground">
								{m.setup_pairing_note()}
							</p>
						</CardHeader>

						<CardContent>
							<form
								onSubmit={(event) => {
									event.preventDefault();
									onSubmit(password, confirmation);
								}}
								className="flex flex-col gap-5"
								aria-busy={busy || undefined}
							>
								<div className="flex flex-col gap-2">
									<OptionLabel
										htmlFor="setup-password"
										label={m.setup_password()}
										help={m.setup_password_help()}
										recommended={m.setup_password_recommended()}
									/>
									<Input
										id="setup-password"
										name="password"
										type="password"
										autoFocus
										autoComplete="new-password"
										value={password}
										disabled={busy}
										aria-invalid={error ? true : undefined}
										aria-describedby={error ? ERROR_ID : undefined}
										onChange={(event) => setPassword(event.target.value)}
									/>
								</div>

								<div className="flex flex-col gap-2">
									<OptionLabel
										htmlFor="setup-confirmation"
										label={m.setup_confirm()}
										help={m.setup_confirm_help()}
										recommended={m.setup_confirm_recommended()}
									/>
									<Input
										id="setup-confirmation"
										name="confirmation"
										type="password"
										autoComplete="new-password"
										value={confirmation}
										disabled={busy}
										aria-invalid={error ? true : undefined}
										aria-describedby={error ? ERROR_ID : undefined}
										onChange={(event) => setConfirmation(event.target.value)}
									/>
									{errorMessage && (
										<p
											id={ERROR_ID}
											role="alert"
											className="text-sm font-medium text-destructive"
										>
											{errorMessage}
										</p>
									)}
								</div>

								{/* Same safety-orange commit action as Login. */}
								<Button
									type="submit"
									className="mt-1 w-full bg-[var(--ss-action)] text-white hover:bg-[var(--ss-action-light)]"
									disabled={!canSubmit}
									aria-busy={busy || undefined}
								>
									{busy ? m.setup_creating() : m.setup_submit()}
								</Button>
							</form>
						</CardContent>
					</Card>
				</motion.div>
			</div>
		</TooltipProvider>
	);
};
