import { existsSync, readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const output = join(root, 'dist', 'client')
const docs = join(root, 'content', 'docs')
const failures = []

function requireFile(path) {
  if (!existsSync(path)) failures.push(path)
}

requireFile(join(output, 'index.html'))
requireFile(join(output, '404.html'))
requireFile(join(output, 'api', 'index.html'))
requireFile(join(output, 'openapi.json'))
requireFile(join(output, 'search-index.json'))

for (const file of readdirSync(docs)) {
  if (!file.endsWith('.md') && !file.endsWith('.mdx')) continue
  const slug = file.replace(/\.(md|mdx)$/, '')
  if (slug === 'meta') continue
  requireFile(slug === 'index' ? join(output, 'docs', 'index.html') : join(output, 'docs', slug, 'index.html'))
}

const htmlFiles = []
function collect(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name)
    if (entry.isDirectory()) collect(path)
    else if (entry.name.endsWith('.html')) htmlFiles.push(path)
  }
}
collect(output)

for (const path of htmlFiles) {
  const html = readFileSync(path, 'utf8')
  if (html.includes('/_serverFn')) failures.push(path + ': contains server function runtime')
  if (html.includes('/api/search')) failures.push(path + ': contains runtime search endpoint')
  if (html.includes('/slipstream/slipstream/')) failures.push(path + ': contains a duplicated Pages base path')
  if (html.includes('href="/openapi.json"')) failures.push(path + ': contains root-relative OpenAPI link')
  if (html.includes('href="/favicon.svg"')) failures.push(path + ': contains root-relative favicon link')
}

if (failures.length) {
  console.error(failures.join('\n'))
  process.exitCode = 1
} else {
  console.log('Pages artifact contains all prerendered routes and no server-only references.')
}
