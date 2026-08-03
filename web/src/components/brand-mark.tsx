import { cn } from "@/lib/utils";

// The Slipstream brand mark — a clean blocky ASCII "S" (smblock style),
// echoing the three-stroke swoosh of the logo. Terminal-cyan with a soft glow.
const MARK = ["▞▀▖", "▚▄ ", "▖ ▌", "▝▀ "];

/** Slipstream brand mark — the ASCII "S" glyph. Size via font size (e.g. `text-base`). */
export function BrandMark({ className }: { className?: string }) {
	return (
		<div
			role="img"
			aria-label="Slipstream"
			className={cn(
				"font-mono leading-[1.1] select-none whitespace-pre",
				"text-[--ss-brand-light] [text-shadow:0_0_12px_rgba(34,211,238,0.5)]",
				className,
			)}
		>
			{MARK.join("\n")}
		</div>
	);
}

export default BrandMark;
