import { getRouteApi } from '@tanstack/react-router'
import { FooterView } from '@unom/app-ui/footer'
import BrandMark from '@/components/BrandMark'
import Wordmark from '@/components/Wordmark'

const rootApi = getRouteApi('__root__')

const SITE_URL = 'https://github.com/vindeckyy/slipstream'
const resolveHref = (to: string) =>
  to.startsWith('/') ? new URL(to, SITE_URL).toString() : to

const footerLinks = [
  { label: 'Quick Start', href: '/docs/quickstart' },
  { label: 'Play', href: '/docs/play' },
  { label: 'Work', href: '/docs/desktop-at-work' },
  { label: 'Install', href: '/docs/install' },
  { label: 'Network & VPN', href: '/docs/network-and-vpn' },
  { label: 'Troubleshooting', href: '/docs/troubleshooting' },
  { label: 'API', href: '/api' },
  { label: 'Discord', href: 'https://discord.gg/kaPNvzMuGU' },
] as const

export default function Footer() {
  const { footer } = rootApi.useLoaderData()

  return (
    <div className="border-t border-fd-border">
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
                Work from the office.
              </p>
            </div>
          </div>

          <nav aria-label="Docs footer" className="flex flex-wrap gap-x-5 gap-y-2 md:max-w-md md:justify-end">
            {footerLinks.map((link) => (
              <a
                key={link.label}
                href={link.href}
                className="text-sm text-fd-muted-foreground transition-colors hover:text-fd-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
              >
                {link.label}
              </a>
            ))}
          </nav>
        </div>
      </div>
      <FooterView
        sections={footer?.sections}
        tagline={footer?.tagline}
        socials={footer?.socials}
        socialsLabel="Socials"
        resolveHref={resolveHref}
        className="border-t border-fd-border/70"
      />
    </div>
  )
}
