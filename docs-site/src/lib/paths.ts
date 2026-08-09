const baseUrl = import.meta.env.BASE_URL || '/'
const normalizedBase = baseUrl.endsWith('/') ? baseUrl : baseUrl + '/'

export function sitePath(path: string): string {
  const normalizedPath = path.startsWith('/') ? path.slice(1) : path
  return normalizedBase + normalizedPath
}

export function routerBasePath(): string {
  if (normalizedBase === '/') return '/'
  return normalizedBase.slice(0, -1)
}
