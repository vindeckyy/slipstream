import { sitePath } from '@/lib/paths'

/** Full Slipstream lockup from the product logo. */
export default function Wordmark({ className = '' }: { className?: string }) {
  return (
    <img
      src={sitePath('/slipstream-logo.png')}
      alt="Slipstream"
      title="Slipstream"
      className={`block w-auto ${className}`}
      draggable={false}
    />
  )
}
