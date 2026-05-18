/**
 * remark plugin: resolve relative .md links in docs/ content to absolute Astro URLs.
 *
 * Why this exists:
 *   - Source docs use markdown-friendly relative links like `./plugins.md` or
 *     `../foo.md#anchor`. Astro renders these verbatim, so the browser tries to
 *     fetch a non-existent `.md` page and 404s.
 *   - Cross-folder links are also unreliable: an author writes `./plugins.md`
 *     from `reference/fledge-toml.md` even though `plugins.md` lives at the
 *     docs root. We resolve by indexing every `.md` in docs/ by basename and
 *     looking the target up there before falling back to a strip-extension fix.
 *
 * The plugin only touches links that:
 *   - end in `.md` or `.md#anchor`
 *   - are not absolute URLs (no protocol, no leading `/`)
 */
import { readdirSync, statSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { visit } from 'unist-util-visit'

const __dirname = dirname(fileURLToPath(import.meta.url))
const DOCS_DIR = join(__dirname, '..', 'src', 'content', 'docs')
const BASE = '/fledge/'

export interface Options {
  /** Override docs directory (tests). */
  docsDir?: string
  /** Override base URL (tests). */
  base?: string
}

export interface DocIndex {
  /** stem (no .md) -> path relative to docs/ without extension, e.g. "plugins" or "reference/fledge-toml" */
  byStem: Map<string, string>
  /** rel paths like "reference/fledge-toml" for direct lookup by rel-without-ext */
  byRelStem: Set<string>
}

function walk(dir: string, prefix = ''): string[] {
  const out: string[] = []
  for (const entry of readdirSync(dir).sort()) {
    const full = join(dir, entry)
    const rel = prefix ? `${prefix}/${entry}` : entry
    if (statSync(full).isDirectory()) out.push(...walk(full, rel))
    else if (entry.endsWith('.md')) out.push(rel)
  }
  return out
}

export function buildDocIndex(docsDir: string): DocIndex {
  const byStem = new Map<string, string>()
  const byRelStem = new Set<string>()
  for (const rel of walk(docsDir)) {
    if (rel === 'SUMMARY.md') continue
    const relStem = rel.replace(/\.md$/, '')
    byRelStem.add(relStem)
    const base = relStem.split('/').pop()!
    // First occurrence wins (basenames are unique in our docs tree today; if
    // that changes we'll get a clear precedence here rather than silent overwrite).
    if (!byStem.has(base)) {
      byStem.set(base, relStem)
    } else if (byStem.get(base) !== relStem) {
      console.warn(`[remark-resolve-doc-links] duplicate basename "${base}": kept ${byStem.get(base)}, ignored ${relStem}`)
    }
  }
  return { byStem, byRelStem }
}

/** Parses `./foo.md#anchor` -> { target: "./foo.md", hash: "#anchor" } */
function splitHash(url: string): { target: string; hash: string } {
  const idx = url.indexOf('#')
  if (idx === -1) return { target: url, hash: '' }
  return { target: url.slice(0, idx), hash: url.slice(idx) }
}

function isExternal(url: string): boolean {
  return /^[a-z][a-z0-9+.-]*:/i.test(url) || url.startsWith('//') || url.startsWith('mailto:')
}

/**
 * Resolve a single href against the source file's location.
 * Returns the rewritten href, or null to keep the original.
 */
export function resolveLink(
  href: string,
  sourceRelPath: string,
  index: DocIndex,
  base = BASE,
  currentFile = sourceRelPath,
): string | null {
  if (!href) return null
  if (isExternal(href)) return null
  if (href.startsWith('/')) return null // already absolute

  const { target, hash } = splitHash(href)
  if (!target.endsWith('.md')) return null

  // Resolve relative to the source file's directory.
  const srcDir = sourceRelPath.includes('/') ? sourceRelPath.split('/').slice(0, -1).join('/') : ''
  // Manual posix join + normalize so we don't pull in node:path on hot mdast paths.
  const segments = (srcDir ? `${srcDir}/${target}` : target).split('/')
  const normalized: string[] = []
  for (const seg of segments) {
    if (!seg || seg === '.') continue
    if (seg === '..') normalized.pop()
    else normalized.push(seg)
  }
  const resolvedRel = normalized.join('/').replace(/\.md$/, '')

  // If the relative resolution lands on a real file, use it.
  if (index.byRelStem.has(resolvedRel)) {
    return `${base}docs/${resolvedRel}${hash}`
  }
  // Otherwise, try basename lookup — handles `./plugins.md` from
  // reference/fledge-toml.md when plugins.md lives at the docs root.
  const baseName = target.replace(/^.*\//, '').replace(/\.md$/, '')
  const found = index.byStem.get(baseName)
  if (found) {
    return `${base}docs/${found}${hash}`
  }
  // Last resort: just strip the .md so we don't leave a guaranteed-404 link.
  // Emit a warning so authors notice during local dev / CI rather than shipping a silent 404.
  const strippedUrl = `${base}docs/${resolvedRel}${hash}`
  console.warn(`[remark-resolve-doc-links] could not resolve "${href}" from ${currentFile}; emitting stripped URL (will 404)`)
  return strippedUrl
}

/** The remark plugin. */
export default function remarkResolveDocLinks(opts: Options = {}) {
  const docsDir = opts.docsDir ?? DOCS_DIR
  const base = opts.base ?? BASE
  const index = buildDocIndex(docsDir)

  return function transformer(tree: unknown, file: { path?: string; history?: string[] }) {
    const filePath = file.path ?? (file.history && file.history[file.history.length - 1])
    if (!filePath) return
    // Path relative to docsDir, posix-normalized.
    const rel = filePath.startsWith(docsDir) ? filePath.slice(docsDir.length + 1).replace(/\\/g, '/') : ''
    if (!rel) return

    // unist-util-visit's types aren't great in JS-friendly TS configs; we
    // narrow inline rather than pulling in heavy mdast typings.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    visit(tree as any, 'link', (node: any) => {
      const rewritten = resolveLink(node.url, rel, index, base, filePath)
      if (rewritten !== null) node.url = rewritten
    })
  }
}
