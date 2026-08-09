import BrandMark from '@/components/BrandMark'
import Wordmark from '@/components/Wordmark'
import { sitePath } from '@/lib/paths'

const footerLinks = [
  { label: 'Quick Start', href: '/docs/quickstart' },
  { label: 'Play', href: '/docs/play' },
  { label: 'Work', href: '/docs/desktop-at-work' },
  { label: 'Install', href: '/docs/install' },
  { label: 'Network & VPN', href: '/docs/network-and-vpn' },
  { label: 'Troubleshooting', href: '/docs/troubleshooting' },
  { label: 'API', href: '/api' },
  { label: 'GitHub', href: 'https://github.com/vindeckyy/slipstream' },
  { label: 'Security', href: 'https://github.com/vindeckyy/slipstream/security/policy' },
] as const

export default function Footer() {
  return (
    <footer className="border-t border-fd-border">
      <div className="mx-auto w-full max-w-6xl px-6 py-10 md:px-8">
        <div className="flex flex-col gap-8 md:flex-row md:items-start md:justify-between">
          <div className="flex items-start gap-3">
            <BrandMark className="size-9 rounded-xl" />
            <div>
              <div className="flex items-center gap-2.5">
                <Wordmark className="h-4" />
                <span className="border-l border-fd-border pl-2.5 text-xs font-medium text-fd-muted-foreground">
                  Docs
                </span>
              </div>
              <p className="mt-2 max-w-sm text-sm leading-6 text-fd-muted-foreground">
                Private desktop and game streaming for Linux hosts. Play on the couch,
                work from the office.
              </p>
            </div>
          </div>

          <nav aria-label="Docs footer" className="flex flex-wrap gap-x-5 gap-y-2 md:max-w-lg md:justify-end">
            {footerLinks.map((link) => (
              <a
                key={link.label}
                href={link.href.startsWith('/') ? sitePath(link.href) : link.href}
                className="text-sm text-fd-muted-foreground transition-colors hover:text-fd-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
              >
                {link.label}
              </a>
            ))}
          </nav>
        </div>
        <p className="mt-8 border-t border-fd-border/70 pt-5 text-xs text-fd-muted-foreground">
          Slipstream is open source under the MIT OR Apache-2.0 license.
        </p>
      </div>
    </footer>
  )
}
