/** Slipstream brand mark from the product logo. */
export default function BrandMark({ className }: { className?: string }) {
  return (
    <img
      src="/slipstream-mark.png"
      alt="Slipstream"
      title="Slipstream"
      className={className}
      draggable={false}
    />
  )
}
