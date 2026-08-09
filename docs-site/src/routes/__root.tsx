/// <reference types="vite/client" />
import { createRootRoute, HeadContent, Outlet, Scripts } from '@tanstack/react-router'
import { RootProvider } from 'fumadocs-ui/provider/tanstack'
import * as React from 'react'
import '@fontsource-variable/geist'
import Footer from '@/components/Footer'
import { sitePath } from '@/lib/paths'
import appCss from '@/styles/app.css?url'

const siteDescription =
  'Private, low-latency desktop and game streaming from a Linux host to Android, Steam Deck, and compatible Moonlight clients.'
const siteOrigin = 'https://vindeckyy.github.io/slipstream'

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: 'utf-8' },
      { name: 'viewport', content: 'width=device-width, initial-scale=1' },
      { name: 'color-scheme', content: 'dark light' },
      { title: 'Slipstream documentation' },
      { name: 'description', content: siteDescription },
      { name: 'robots', content: 'index,follow' },
      { property: 'og:title', content: 'Slipstream documentation' },
      { property: 'og:description', content: siteDescription },
      { property: 'og:type', content: 'website' },
      { property: 'og:url', content: `${siteOrigin}/` },
      { property: 'og:image', content: `${siteOrigin}/slipstream-logo.png` },
      { property: 'og:image:alt', content: 'Slipstream logo' },
      { name: 'twitter:card', content: 'summary_large_image' },
      { name: 'twitter:title', content: 'Slipstream documentation' },
      { name: 'twitter:description', content: siteDescription },
      { name: 'twitter:image', content: `${siteOrigin}/slipstream-logo.png` },
    ],
    links: [
      { rel: 'stylesheet', href: appCss },
      { rel: 'icon', type: 'image/svg+xml', href: sitePath('/favicon.svg') },
      { rel: 'manifest', href: sitePath('/manifest.webmanifest') },
      { rel: 'canonical', href: `${siteOrigin}/` },
    ],
  }),
  component: RootComponent,
})

function RootComponent() {
  return (
    <RootDocument>
      <Outlet />
    </RootDocument>
  )
}

function RootDocument({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <HeadContent />
      </head>
      <body className="flex min-h-screen flex-col">
        <RootProvider
          search={{
            options: { type: 'static', api: sitePath('/search-index.json') },
            links: [
              ['Quick Start', sitePath('/docs/quickstart')],
              ['Play', sitePath('/docs/play')],
              ['Desktop at work', sitePath('/docs/desktop-at-work')],
              ['Install the host', sitePath('/docs/install')],
              ['Troubleshooting', sitePath('/docs/troubleshooting')],
              ['API reference', sitePath('/api')],
            ],
          }}
        >
          {children}
          <Footer />
        </RootProvider>
        <Scripts />
      </body>
    </html>
  )
}
