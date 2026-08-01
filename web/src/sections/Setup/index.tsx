import { type FC, useState } from "react";
import { useLocale } from "@/lib/i18n";
import { type SetupError, SetupView } from "./view";

export const SectionSetup: FC<{ next?: string }> = ({ next }) => {
	useLocale();
	const [error, setError] = useState<SetupError | null>(null);
	const [busy, setBusy] = useState(false);

	const onSubmit = async (password: string, confirmation: string) => {
		setBusy(true);
		setError(null);
		try {
			const res = await fetch("/_auth/setup", {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ password, confirmation }),
			});
			if (!res.ok) {
				const body = (await res.json().catch(() => null)) as {
					statusMessage?: string;
				} | null;
				const message = body?.statusMessage?.toLowerCase() ?? "";
				if (res.status === 409) {
					window.location.href = "/login";
					return;
				}
				setError(
					message.includes("at least")
						? "too-short"
						: message.includes("match")
							? "mismatch"
							: "unavailable",
				);
				setBusy(false);
				return;
			}
			window.location.href = safeNextPath(next);
		} catch {
			setError("unavailable");
			setBusy(false);
		}
	};

	return <SetupView onSubmit={onSubmit} error={error} busy={busy} />;
};

function safeNextPath(next: string | undefined): string {
	if (!next) return "/";
	try {
		const target = new URL(next, window.location.origin);
		return target.origin === window.location.origin
			? target.pathname + target.search + target.hash
			: "/";
	} catch {
		return "/";
	}
}
