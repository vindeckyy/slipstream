import { cn } from "@/lib/utils";
import BrandMark from "./brand-mark";
import Wordmark from "./wordmark";

// Full Slipstream lockup in the ASCII console style: the wordmark + the S
// glyph beneath it. Used on the Setup/Login splash.
export function Logo({ className }: { className?: string }) {
	return (
		<div
			className={cn(
				"flex flex-col items-center gap-2 select-none",
				className,
			)}
		>
			<Wordmark className="text-[13px] sm:text-[15px]" />
			<BrandMark className="text-[11px] opacity-70" />
		</div>
	);
}

export default Logo;
