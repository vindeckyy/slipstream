import { readdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'
import tsConfigPaths from 'vite-tsconfig-paths'
import tailwindcss from '@tailwindcss/vite'
import mdx from 'fumadocs-mdx/vite'
import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import { nitroV2Plugin } from '@tanstack/nitro-v2-vite-plugin'
import viteReact from '@vitejs/plugin-react'

const pagesBase = process.env.PAGES_BASE_PATH || '/'
const staticBuild = Boolean(process.env.PAGES_BASE_PATH)
const routerBase = pagesBase === '/' ? '/' : pagesBase.slice(0, -1)
const docsDirectory = fileURLToPath(new URL('./content/docs', import.meta.url))
const docsPages = readdirSync(docsDirectory)
  .filter((file) => /\.(md|mdx)$/.test(file))
  .map((file) => file.replace(/\.(md|mdx)$/, ''))
  .filter((slug) => slug !== 'meta')
  .map((slug) => ({ path: slug === 'index' ? '/docs' : '/docs/' + slug }))

const staticPages = [
  { path: '/' },
  { path: '/api' },
  { path: '/api/search', prerender: { outputPath: '/search-index.json' } },
  ...docsPages,
]

export default defineConfig({
  base: pagesBase,
  server: { port: 3001 },
  plugins: [
    mdx(),
    tsConfigPaths({ projects: ['./tsconfig.json'] }),
    tailwindcss(),
    tanstackStart({
      router: { basepath: routerBase },
      ...(staticBuild
        ? {
            pages: staticPages,
            prerender: {
              enabled: true,
              autoSubfolderIndex: true,
              autoStaticPathsDiscovery: false,
              crawlLinks: false,
              failOnError: true,
              concurrency: 4,
            },
          }
        : {}),
    }),
    ...(staticBuild
      ? []
      : nitroV2Plugin({
          preset: 'bun',
          compatibilityDate: '2026-06-12',
        })),
    viteReact(),
  ],
})
