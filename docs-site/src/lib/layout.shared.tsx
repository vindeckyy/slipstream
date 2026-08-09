import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared'
import BrandMark from '@/components/BrandMark'
import Wordmark from '@/components/Wordmark'

// Shared chrome for the docs and home layouts. Primary links answer the jobs
// people arrive with; community and source stay secondary.
export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: (
        <div className="flex items-center gap-2.5">
          <BrandMark className="size-6 rounded-md" />
          <Wordmark className="text-[0.72rem] sm:text-sm" />
          <span className="hidden border-l border-fd-border pl-2.5 text-xs font-medium text-fd-muted-foreground sm:inline">
            Docs
          </span>
        </div>
      ),
    },
    githubUrl: 'https://github.com/vindeckyy/slipstream',
    links: [
      { text: 'Quick Start', url: '/docs/quickstart' },
      { text: 'Play', url: '/docs/play' },
      { text: 'Work', url: '/docs/desktop-at-work' },
      { text: 'Install', url: '/docs/install' },
      { text: 'Clients', url: '/docs/clients' },
      { text: 'API', url: '/api' },
      { text: 'GitHub Issues', url: 'https://github.com/vindeckyy/slipstream/issues' },
    ],
  }
}
