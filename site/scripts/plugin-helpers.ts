export type Language = 'rust' | 'ts' | 'js' | 'go' | 'python' | 'shell' | 'other'
export type TrustTier = 'official' | 'community' | 'experimental'

export function slugFromName(name: string): string {
  if (!name) throw new Error('slugFromName: name is required')
  const PREFIX = 'fledge-plugin-'
  return name.startsWith(PREFIX) ? name.slice(PREFIX.length) : name
}

export function inferLanguage(repoFiles: readonly string[]): Language {
  const has = (f: string) => repoFiles.includes(f)
  if (has('Cargo.toml')) return 'rust'
  if (has('package.json')) return has('tsconfig.json') ? 'ts' : 'js'
  if (has('go.mod')) return 'go'
  if (has('pyproject.toml') || has('setup.py')) return 'python'
  if (repoFiles.some(f => f.endsWith('.sh'))) return 'shell'
  return 'other'
}

export function inferTrustTier(owner: string, topics: readonly string[]): TrustTier {
  if (topics.includes('fledge-plugin-experimental')) return 'experimental'
  if (owner === 'CorvidLabs') return 'official'
  return 'community'
}
