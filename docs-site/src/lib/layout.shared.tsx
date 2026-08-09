import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared'
import BrandMark from '@/components/BrandMark'
import Wordmark from '@/components/Wordmark'
import { sitePath } from '@/lib/paths'

// Shared chrome for the docs and home layouts. Primary links answer the jobs
// people arrive with; community and source stay secondary.
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
    githubUrl: 'https://github.com/vindeckyy/slipstream',
    links: [
      { text: 'Quick Start', url: sitePath('/docs/quickstart') },
      { text: 'Play', url: sitePath('/docs/play') },
      { text: 'Work', url: sitePath('/docs/desktop-at-work') },
      { text: 'Install', url: sitePath('/docs/install') },
      { text: 'Clients', url: sitePath('/docs/clients') },
      { text: 'API', url: sitePath('/api') },
      {
        type: 'menu',
        text: 'Community',
        items: [
          { text: 'Discord', url: 'https://discord.gg/kaPNvzMuGU' },
          { text: 'Reddit', url: 'https://www.reddit.com/r/Slipstream/' },
          { text: 'Support on Ko-fi', url: 'https://ko-fi.com/slipstream' },
        ],
      },
    ],
  }
}
