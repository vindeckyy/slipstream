import { cn } from "@/lib/utils";

/** Full Slipstream lockup (mark + wordmark) from the product logo. */
export function Logo({ className }: { className?: string }) {
	return (
		<img
			src="/slipstream-logo.png"
			alt="Slipstream"
			className={cn("block h-auto w-full", className)}
			draggable={false}
		/>
	);
}

export default Logo;
