import { cn } from "@/lib/utils";

// Full Slipstream lockup from the product logo. Size via height (e.g. `h-5`);
// width follows the image aspect ratio.
export function Wordmark({ className }: { className?: string }) {
	return (
		<img
			src="/slipstream-logo.png"
			alt="Slipstream"
			title="Slipstream"
			className={cn("block w-auto", className)}
			draggable={false}
		/>
	);
}

export default Wordmark;
