import { Suspense } from 'react'
import { createFileRoute, notFound } from '@tanstack/react-router'
import { DocsLayout } from 'fumadocs-ui/layouts/docs'
import { DocsBody, DocsDescription, DocsPage, DocsTitle } from 'fumadocs-ui/layouts/docs/page'
import browserCollections from 'collections/browser'
import { useFumadocsLoader } from 'fumadocs-core/source/client'
import type { Root as PageTreeRoot } from 'fumadocs-core/page-tree'
import { baseOptions } from '@/lib/layout.shared'
import { source } from '@/lib/source'
import { useMDXComponents } from '@/components/mdx'

export const Route = createFileRoute('/docs/$')({
  head: ({ params }) => {
    const slugs = (params._splat ?? '').split('/').filter(Boolean)
    const page = source.getPage(slugs)
    const title = page?.data.title ?? 'Documentation'
    const description = page?.data.description ?? 'Slipstream documentation.'
    const path = page?.path ?? `/docs/${slugs.join('/')}`
    const canonical = `https://vindeckyy.github.io/slipstream${path}`
    return {
      meta: [
        { title: `${title} | Slipstream` },
        { name: 'description', content: description },
        { property: 'og:title', content: `${title} | Slipstream` },
        { property: 'og:description', content: description },
        { property: 'og:type', content: 'article' },
        { property: 'og:url', content: canonical },
        { name: 'twitter:card', content: 'summary_large_image' },
      ],
      links: [{ rel: 'canonical', href: canonical }],
    }
  },
  component: Page,
  loader: async ({ params }) => {
    const slugs = (params._splat ?? '').split('/').filter(Boolean)
    const page = source.getPage(slugs)
    if (!page) throw notFound()

    const data = {
      path: page.path,
      pageTree: await source.serializePageTree(source.getPageTree()),
    }
    await clientLoader.preload(data.path)
    return data
  },
})

const clientLoader = browserCollections.docs.createClientLoader({
  component({ toc, frontmatter, default: MDX }, _props: undefined) {
    return (
      <DocsPage toc={toc}>
        <DocsTitle>{frontmatter.title}</DocsTitle>
        <DocsDescription>{frontmatter.description}</DocsDescription>
        <DocsBody>
          <MDX components={useMDXComponents()} />
        </DocsBody>
      </DocsPage>
    )
  },
})

function Page() {
  const data = useFumadocsLoader(Route.useLoaderData()) as {
    path: string
    pageTree: PageTreeRoot
  }

  return (
    <DocsLayout {...baseOptions()} tree={data.pageTree}>
      <Suspense>{clientLoader.useContent(data.path)}</Suspense>
    </DocsLayout>
  )
}
