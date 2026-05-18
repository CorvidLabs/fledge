import { writeFileSync, mkdirSync, readFileSync, existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  slugFromName,
  inferLanguage,
  inferTrustTier,
  type Language,
  type TrustTier,
} from './plugin-helpers'
import { renderReadme } from './render-readme'
import { relatedSlugs } from './related-plugins'
import allowlist from './community-allowlist.json' with { type: 'json' }

const __dirname = dirname(fileURLToPath(import.meta.url))
const DATA_DIR = join(__dirname, '..', 'src', 'data')
const PER_PLUGIN_DIR = join(DATA_DIR, 'plugins')
const INDEX_PATH = join(DATA_DIR, 'plugins.json')
const TOKEN = process.env.GITHUB_TOKEN

function rawHeaders(): Record<string, string> {
  const headers: Record<string, string> = { 'User-Agent': 'fledge-site-builder' }
  if (TOKEN) headers.Authorization = `Bearer ${TOKEN}`
  return headers
}

export interface RegistryEntry {
  name: string
  slug: string
  version: string
  description: string
  language: Language
  trust_tier: TrustTier
  install: string
  repo: string
  topics: string[]
  stars: number
  updated_at: string
  default_branch: string
}

export interface FullEntry extends RegistryEntry {
  readme_html: string
  license: string | null
  open_issues: number
  related_slugs: string[]
}

interface GhRepo {
  name: string
  owner: { login: string }
  description: string | null
  html_url: string
  default_branch: string
  stargazers_count: number
  topics: string[] | null
  pushed_at: string
  license: { spdx_id: string } | null
  open_issues_count: number
}

interface ManifestInfo {
  files: string[]
  version: string | null
}

export function repoToEntry(repo: GhRepo, info: ManifestInfo): RegistryEntry {
  return {
    name: repo.name,
    slug: slugFromName(repo.name),
    version: info.version ?? 'unknown',
    description: repo.description ?? '',
    language: inferLanguage(info.files),
    trust_tier: inferTrustTier(repo.owner.login, repo.topics ?? []),
    install: `fledge plugins install ${repo.owner.login}/${repo.name}`,
    repo: repo.html_url,
    topics: repo.topics ?? [],
    stars: repo.stargazers_count,
    updated_at: repo.pushed_at,
    default_branch: repo.default_branch,
  }
}

async function gh<T>(url: string): Promise<T> {
  const headers: Record<string, string> = {
    Accept: 'application/vnd.github+json',
    'User-Agent': 'fledge-site-builder',
  }
  if (TOKEN) headers.Authorization = `Bearer ${TOKEN}`
  const res = await fetch(url, { headers })
  if (!res.ok) throw new Error(`GH ${res.status} ${res.statusText} on ${url}`)
  return res.json() as Promise<T>
}

async function listFledgePluginRepos(): Promise<GhRepo[]> {
  const owners = ['CorvidLabs', ...(allowlist as string[])]
  const all: GhRepo[] = []
  for (const owner of owners) {
    const q = encodeURIComponent(`org:${owner} fledge-plugin- in:name`)
    const url = `https://api.github.com/search/repositories?q=${q}&per_page=100`
    const result = await gh<{ items: GhRepo[] }>(url)
    for (const item of result.items) {
      if (item.name.startsWith('fledge-plugin-')) all.push(item)
    }
  }
  return all
}

async function fetchManifest(
  owner: string,
  repo: string,
  branch: string,
): Promise<ManifestInfo> {
  const listing = await gh<{ tree: { path: string; type: string }[] }>(
    `https://api.github.com/repos/${owner}/${repo}/git/trees/${branch}?recursive=0`,
  )
  const files = listing.tree.filter(n => n.type === 'blob').map(n => n.path)
  let version: string | null = null
  if (files.includes('Cargo.toml')) {
    const cargoToml = await fetch(
      `https://raw.githubusercontent.com/${owner}/${repo}/${branch}/Cargo.toml`,
      { headers: rawHeaders() },
    )
      .then(r => r.text())
      .catch(() => '')
    const m = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)
    version = m?.[1] ?? null
  } else if (files.includes('package.json')) {
    const pkg = await fetch(
      `https://raw.githubusercontent.com/${owner}/${repo}/${branch}/package.json`,
      { headers: rawHeaders() },
    )
      .then(r => r.json())
      .catch(() => ({}))
    version = (pkg as { version?: string }).version ?? null
  }
  return { files, version }
}

async function fetchReadme(owner: string, repo: string, branch: string): Promise<string> {
  for (const name of ['README.md', 'readme.md', 'README.MD', 'README']) {
    const res = await fetch(
      `https://raw.githubusercontent.com/${owner}/${repo}/${branch}/${name}`,
      { headers: rawHeaders() },
    )
    if (res.ok) return res.text()
  }
  return ''
}

function loadCachedIndex(): RegistryEntry[] | null {
  if (!existsSync(INDEX_PATH)) return null
  try {
    return JSON.parse(readFileSync(INDEX_PATH, 'utf-8')) as RegistryEntry[]
  } catch {
    return null
  }
}

async function main() {
  if (!TOKEN) {
    console.warn('[build-plugin-registry] GITHUB_TOKEN not set — using unauthenticated rate limits (60/hr). Set GITHUB_TOKEN to avoid throttling.')
  }

  mkdirSync(PER_PLUGIN_DIR, { recursive: true })

  let repos: GhRepo[]
  try {
    repos = await listFledgePluginRepos()
  } catch (e) {
    console.warn(`[build-plugin-registry] GH fetch failed: ${(e as Error).message}`)
    const cached = loadCachedIndex()
    if (cached) {
      console.warn('[build-plugin-registry] using cached plugins.json from previous build')
      return
    }
    console.error('[build-plugin-registry] no cache available, writing empty index')
    writeFileSync(INDEX_PATH, '[]')
    return
  }

  const enrich = await Promise.all(
    repos.map(async r => {
      const info = await fetchManifest(r.owner.login, r.name, r.default_branch).catch(
        () => ({ files: [], version: null }),
      )
      const readme = await fetchReadme(r.owner.login, r.name, r.default_branch).catch(() => '')
      return { repo: r, info, readme }
    }),
  )

  const entries = enrich.map(({ repo, info, readme }) => ({
    entry: repoToEntry(repo, info),
    repo,
    readme,
  }))
  const index: RegistryEntry[] = entries.map(e => e.entry)
  const miniUniverse = index.map(e => ({ slug: e.slug, language: e.language, topics: e.topics }))

  writeFileSync(INDEX_PATH, JSON.stringify(index, null, 2))
  console.log(`[build-plugin-registry] wrote ${index.length} entries to plugins.json`)

  for (const { entry, repo, readme } of entries) {
    const full: FullEntry = {
      ...entry,
      readme_html: renderReadme(readme, { repoUrl: repo.html_url, defaultBranch: repo.default_branch }),
      license: repo.license?.spdx_id ?? null,
      open_issues: repo.open_issues_count,
      related_slugs: relatedSlugs(entry.slug, miniUniverse, 3),
    }
    writeFileSync(join(PER_PLUGIN_DIR, `${entry.slug}.json`), JSON.stringify(full, null, 2))
  }
  console.log(`[build-plugin-registry] wrote ${entries.length} per-plugin files`)
}

// Only run when invoked as a script, not when imported by tests.
if (import.meta.main) {
  main().catch(err => {
    console.error(err)
    process.exit(1)
  })
}
