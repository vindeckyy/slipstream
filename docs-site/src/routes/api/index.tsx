import { useEffect, useMemo, useState } from 'react'
import { createFileRoute, Link } from '@tanstack/react-router'
import { ApiReferenceReact } from '@scalar/api-reference-react'
// @scalar/api-reference-react@0.9.47's entry does NOT import its own stylesheet
// (and doesn't inject it at runtime), so we must ship it ourselves or the
// reference renders unstyled. Load it as a route-scoped <link> (same pattern as
// the root app.css), so it's present for SSR + the client-side Vue mount.
import scalarCss from '@scalar/api-reference-react/style.css?url'
import BrandMark from '@/components/BrandMark'
import Wordmark from '@/components/Wordmark'
import { sitePath } from '@/lib/paths'

export const Route = createFileRoute('/api/')({
  component: ApiReference,
  head: () => ({
    meta: [
      { title: 'Slipstream API reference' },
      {
        name: 'description',
        content: 'Interactive OpenAPI reference for the Slipstream host API.',
      },
    ],
    links: [{ rel: 'stylesheet', href: scalarCss }],
  }),
})

// Keep Scalar on the browser console's cool-slate surfaces and cyan accent.
//
// Scalar toggles `.light-mode` / `.dark-mode` on `document.body`, and it renders
// our `customCss` before its built-in theme preset in the same style tag. A bare
// `.dark-mode` rule has equal specificity to the later preset and loses. Scoping
// these rules to `body.dark-mode` / `body.light-mode` wins over both the linked
// base sheet and the in-component preset. Scalar ignores unknown custom-property
// names, so this stays forward-safe.
const SCALAR_CSS = `
body.light-mode,
body.dark-mode {
  --scalar-font: 'Geist Variable', ui-sans-serif, system-ui, sans-serif;
  --scalar-font-code: ui-monospace, 'SFMono-Regular', Menlo, Consolas, monospace;
  --scalar-radius: 0.5rem;
  --scalar-radius-lg: 0.75rem;
  --scalar-radius-xl: 0.875rem;
}

/* Dark console surfaces. */
body.dark-mode {
  --scalar-background-1: #060a0f;
  --scalar-background-2: #0d141c;
  --scalar-background-3: #121a24;
  --scalar-background-accent: #22d3ee1f;
  --scalar-border-color: #526472;

  --scalar-color-1: #e8eef2;
  --scalar-color-2: #93a7b5;
  --scalar-color-3: #6f8592;
  --scalar-color-accent: #22d3ee;

  --scalar-link-color: #22d3ee;
  --scalar-link-color-hover: #a5f3fc;

  --scalar-button-1: #22d3ee;
  --scalar-button-1-color: #060a0f;
  --scalar-button-1-hover: #22d3ee;

  --scalar-sidebar-background-1: #0d141c;
  --scalar-sidebar-color-1: #e8eef2;
  --scalar-sidebar-color-2: #93a7b5;
  --scalar-sidebar-color-active: #22d3ee;
  --scalar-sidebar-item-hover-background: #22d3ee1f;
  --scalar-sidebar-item-hover-color: #e8eef2;
  --scalar-sidebar-item-active-background: #22d3ee2e;
  --scalar-sidebar-border-color: #526472;
  --scalar-sidebar-search-background: #111820;
  --scalar-sidebar-search-border-color: #526472;
  --scalar-sidebar-search-color: #93a7b5;
  --scalar-sidebar-indent-border: #526472;
  --scalar-sidebar-indent-border-active: #22d3ee;
  --scalar-sidebar-indent-border-hover: #93a7b5;

  --scalar-header-background-1: #060a0f;
  --scalar-header-color-1: #e8eef2;
  --scalar-header-border-color: #526472;

  --scalar-scrollbar-color: #526472;
  --scalar-scrollbar-color-active: #93a7b5;

  --scalar-color-green: #4ade80;
  --scalar-color-red: #f87171;
  --scalar-color-yellow: #fbbf24;
  --scalar-color-blue: #60a5fa;
  --scalar-color-orange: #fb923c;
  --scalar-color-purple: #22d3ee;
}

/* Light console surfaces. */
body.light-mode {
  --scalar-background-1: #eef3f6;
  --scalar-background-2: #ffffff;
  --scalar-background-3: #e2ebf0;
  --scalar-background-accent: #0891b21a;
  --scalar-border-color: #849098;

  --scalar-color-1: #0a1620;
  --scalar-color-2: #4d6672;
  --scalar-color-3: #647b87;
  --scalar-color-accent: #0891b2;

  --scalar-link-color: #0891b2;
  --scalar-link-color-hover: #0e7490;

  --scalar-button-1: #0891b2;
  --scalar-button-1-color: #ffffff;
  --scalar-button-1-hover: #0e7490;

  --scalar-sidebar-background-1: #ffffff;
  --scalar-sidebar-color-1: #0a1620;
  --scalar-sidebar-color-2: #4d6672;
  --scalar-sidebar-color-active: #0e7490;
  --scalar-sidebar-item-hover-background: #0891b214;
  --scalar-sidebar-item-hover-color: #0a1620;
  --scalar-sidebar-item-active-background: #0891b222;
  --scalar-sidebar-border-color: #849098;
  --scalar-sidebar-search-background: #ffffff;
  --scalar-sidebar-search-border-color: #849098;
  --scalar-sidebar-search-color: #4d6672;
  --scalar-sidebar-indent-border: #849098;
  --scalar-sidebar-indent-border-active: #0891b2;
  --scalar-sidebar-indent-border-hover: #4d6672;

  --scalar-header-background-1: #ffffff;
  --scalar-header-color-1: #0a1620;
  --scalar-header-border-color: #849098;

  --scalar-scrollbar-color: #849098;
  --scalar-scrollbar-color-active: #4d6672;

  --scalar-color-green: #16a34a;
  --scalar-color-red: #dc2626;
  --scalar-color-yellow: #d97706;
  --scalar-color-blue: #2563eb;
  --scalar-color-orange: #ea580c;
  --scalar-color-purple: #0891b2;
}
`

function ApiReference() {
  // Follow the docs' own light/dark switch and hide Scalar's own toggle, so the
  // Fumadocs toggle stays the single source of truth. Fumadocs drives next-themes
  // with `attribute: "class"`, which writes the resolved theme as a class on
  // <html>. Read that class directly rather than next-themes' useTheme(). The
  // class includes system resolution and cannot desync from the docs toggle when
  // bridging into Scalar's separate Vue app. Default to dark (the docs default)
  // so SSR and the first client render agree. The observer then tracks the live
  // class, including OS changes while system mode is active.
  const [isDark, setIsDark] = useState(true)
  useEffect(() => {
    const root = document.documentElement
    const sync = () => setIsDark(root.classList.contains('dark'))
    sync()
    const observer = new MutationObserver(sync)
    observer.observe(root, { attributes: true, attributeFilter: ['class'] })
    return () => observer.disconnect()
  }, [])

  // Scalar pollutes global scope and never cleans up: it appends a persistent
  // <style id="scalar-style"> to <head> that includes a *global*
  // `body { background-color: var(--scalar-background-1) }`, adds its #scalar-refs
  // teleport target, and toggles .dark-mode/.light-mode on <body>. After client
  // navigation, that residue bleeds into the next page. Strip it when /api
  // unmounts so leaving the page restores a fresh-load DOM; Scalar re-injects a
  // fresh instance on re-entry.
  useEffect(
    () => () => {
      document.getElementById('scalar-style')?.remove()
      document.getElementById('scalar-refs')?.remove()
      document.body.classList.remove('dark-mode', 'light-mode')
    },
    [],
  )

  // A fresh object on each theme flip so the React wrapper's
  // `updateConfiguration` effect fires and Scalar swaps the body mode class.
  const configuration = useMemo(
    () => ({
      url: sitePath('/openapi.json'),
      darkMode: isDark,
      hideDarkModeToggle: true,
      agent: { disabled: true },
      metaData: { title: 'Slipstream API reference' },
      hideDownloadButton: false,
      customCss: SCALAR_CSS,
    }),
    [isDark],
  )

  return (
    <div className="flex min-h-screen flex-col">
      <header className="flex h-14 shrink-0 items-center justify-between border-b border-fd-border px-4 md:px-6">
        <h1
          aria-label="Slipstream API reference"
          className="flex min-w-0 items-center text-sm font-medium"
        >
          <Link
            to="/docs/$"
            params={{ _splat: '' }}
            aria-label="Slipstream documentation"
            className="flex items-center gap-2 no-underline"
          >
            <BrandMark className="size-6" />
            <Wordmark className="text-sm" />
          </Link>
          <span className="ml-2 hidden text-sm text-fd-muted-foreground sm:inline">
            API reference
          </span>
        </h1>
        <nav
          aria-label="API reference navigation"
          className="flex items-center gap-4 text-sm"
        >
          <Link
            to="/docs/$"
            params={{ _splat: '' }}
            className="text-fd-muted-foreground transition-colors hover:text-fd-foreground"
          >
            Docs
          </Link>
          <a
            href={sitePath("/openapi.json")}
            className="text-fd-muted-foreground transition-colors hover:text-fd-foreground"
          >
            OpenAPI JSON
          </a>
        </nav>
      </header>

      <main className="min-h-0 flex flex-1 flex-col" aria-label="API reference">
        {/* Scalar mounts a Vue app client-side in a useEffect. The server keeps
            this container empty, and the browser mounts the reference into it. */}
        <div className="min-h-0 flex-1">
          <ApiReferenceReact configuration={configuration} />
        </div>
      </main>
    </div>
  )
}
