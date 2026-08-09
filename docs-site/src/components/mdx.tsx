import defaultMdxComponents from 'fumadocs-ui/mdx'
import { Tab, Tabs } from 'fumadocs-ui/components/tabs'
import type { ComponentProps } from 'react'
import type { MDXComponents } from 'mdx/types'
import BitrateCalculator from '@/components/BitrateCalculator'
import { sitePath } from '@/lib/paths'

function MDXLink({ href, ...props }: ComponentProps<'a'>) {
  const resolvedHref = href?.startsWith('/') ? sitePath(href) : href
  return <a href={resolvedHref} {...props} />
}

export function getMDXComponents(components?: MDXComponents) {
  return {
    ...defaultMdxComponents,
    // Custom components usable in any .md/.mdx without a per-file import.
    BitrateCalculator,
    a: MDXLink,
    // Per-platform instructions: <Tabs items={['Ubuntu', 'Fedora']}><Tab value="Ubuntu">...
    Tabs,
    Tab,
    ...components,
  } satisfies MDXComponents
}

export const useMDXComponents = getMDXComponents

declare global {
  type MDXProvidedComponents = ReturnType<typeof getMDXComponents>
}
