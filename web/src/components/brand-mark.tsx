/** Slipstream brand mark from the product logo (cyan stream "S"). */
export function BrandMark({ className }: { className?: string }) {
	return (
		<img
			src="/slipstream-mark.png"
			alt="Slipstream"
			className={className}
			draggable={false}
		/>
	);
}

export default BrandMark;
