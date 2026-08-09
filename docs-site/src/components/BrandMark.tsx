import { sitePath } from '@/lib/paths'

/** Slipstream brand mark from the product logo. */
export default function BrandMark({ className }: { className?: string }) {
  return (
    <img
      src={sitePath('/slipstream-mark.png')}
      alt="Slipstream"
      title="Slipstream"
      className={`block object-contain ${className ?? ''}`}
      draggable={false}
    />
  )
}
