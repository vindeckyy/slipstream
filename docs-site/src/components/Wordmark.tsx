/** Full Slipstream lockup from the product logo. */
export default function Wordmark({ className = '' }: { className?: string }) {
  return (
    <img
      src="/slipstream-logo.png"
      alt="Slipstream"
      title="Slipstream"
      className={`block w-auto ${className}`}
      draggable={false}
    />
  )
}
