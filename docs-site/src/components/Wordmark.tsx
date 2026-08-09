/** Full Slipstream wordmark. */
export default function Wordmark({ className = '' }: { className?: string }) {
  return (
    <span
      role="img"
      aria-label="Slipstream"
      title="Slipstream"
      className={`inline-block whitespace-nowrap font-sans font-bold italic leading-none tracking-[-0.06em] text-[#69cdf4] ${className}`}
    >
      Slipstream
    </span>
  )
}
