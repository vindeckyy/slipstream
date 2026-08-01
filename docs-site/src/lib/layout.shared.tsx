import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared'
import BrandMark from '@/components/BrandMark'
import Wordmark from '@/components/Wordmark'

// Shared chrome for the docs and home layouts. Keep the product identity visible
// while the links answer the questions people arrive with.
export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: (
        <div className="flex items-center gap-2.5">
          <BrandMark className="size-6 rounded-md" />
          <Wordmark className="h-4" />
          <span className="hidden border-l border-fd-border pl-2.5 text-xs font-medium text-fd-muted-foreground sm:inline">
            Docs
          </span>
        </div>
      ),
    },
    links: [
      { text: 'Start here', url: '/docs/quickstart' },
      { text: 'Install host', url: '/docs/install' },
      { text: 'Connect a client', url: '/docs/clients' },
      { text: 'Browser console', url: '/docs/web-console' },
      { text: 'API', url: '/api' },
      { text: 'Website', url: 'https://slipstream.unom.io' },
      { text: 'Support', url: 'https://ko-fi.com/slipstream' },
      { text: 'Source code', url: 'https://github.com/vindeckyy/slipstream/slipstream' },
      { text: 'Discord', url: 'https://discord.gg/kaPNvzMuGU' },
      { text: 'Reddit', url: 'https://www.reddit.com/r/Slipstream/' },
    ],
  }
}
