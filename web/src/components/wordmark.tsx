import { cn } from "@/lib/utils";

// The Slipstream wordmark in clean blocky ASCII (smblock) — "SLIPSTREAM" as a
// sharp terminal signature: monospace, cyan, soft glow. Size via font size
// (e.g. `text-lg`); the mono grid keeps the aspect.
const WORDMARK = [
	"▞▀▖▌  ▜▘▛▀▖▞▀▖▀▛▘▛▀▖▛▀▘▞▀▖▙▗▌",
	"▚▄ ▌  ▐ ▙▄▘▚▄  ▌ ▙▄▘▙▄ ▙▄▌▌▘▌",
	"▖ ▌▌  ▐ ▌  ▖ ▌ ▌ ▌▚ ▌  ▌ ▌▌ ▌",
	"▝▀ ▀▀▘▀▘▘  ▝▀  ▘ ▘ ▘▀▀▘▘ ▘▘ ▘",
];

/** The Slipstream wordmark as ASCII text. Size via font size (e.g. `text-lg`). */
export function Wordmark({ className }: { className?: string }) {
	return (
		<div
			role="img"
			aria-label="Slipstream"
			title="Slipstream"
			className={cn(
				"font-mono leading-[1.15] select-none whitespace-pre",
				"text-[--ss-brand-light] [text-shadow:0_0_14px_rgba(34,211,238,0.45)]",
				className,
			)}
		>
			{WORDMARK.join("\n")}
		</div>
	);
}

export default Wordmark;
