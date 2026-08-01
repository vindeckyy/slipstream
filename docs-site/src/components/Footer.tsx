import { getRouteApi } from '@tanstack/react-router'
import { FooterView } from '@unom/app-ui/footer'
import BrandMark from '@/components/BrandMark'
import Wordmark from '@/components/Wordmark'

const rootApi = getRouteApi('__root__')

// The CMS sections and social links stay shared with the marketing site. The
// small identity row gives the docs shell its own clear home before those
// shared links begin.
const SITE_URL = 'https://slipstream.unom.io'
const resolveHref = (to: string) =>
  to.startsWith('/') ? new URL(to, SITE_URL).toString() : to

export default function Footer() {
  const { footer } = rootApi.useLoaderData()

  return (
    <div className="border-t border-fd-border">
      <div className="mx-auto flex w-full max-w-6xl items-center gap-3 px-6 py-8 md:px-8">
        <BrandMark className="size-9 rounded-xl" />
        <div>
          <div className="flex items-center gap-2.5">
            <Wordmark className="h-4" />
            <span className="border-l border-fd-border pl-2.5 text-xs font-medium text-fd-muted-foreground">
              Docs
            </span>
          </div>
          <p className="mt-1 text-sm text-fd-muted-foreground">
            Guides for the Slipstream host, clients, and browser console.
          </p>
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
