import { readdirSync, statSync, mkdirSync, writeFileSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { HUB_DOCS } from '../src/data/hub'

const __dirname = dirname(fileURLToPath(import.meta.url))
const PUBLIC_DIR = join(__dirname, '..', 'public')

// The standalone site is retired: every legacy mdBook `.html` path now
// redirects straight to the CorvidLabs hub docs index (no internal hop).
const TARGET = HUB_DOCS

export function redirectHtml(target: string): string {
  return `<!DOCTYPE html>
<html lang="en"><head>
<meta charset="utf-8">
<title>Moved to CorvidLabs</title>
<link rel="canonical" href="${target}">
<meta http-equiv="refresh" content="0; url=${target}">
<meta name="robots" content="noindex">
<script>location.replace(${JSON.stringify(target)})</script>
</head><body>
<p>This page has moved. <a href="${target}">This site has moved to CorvidLabs →</a></p>
</body></html>
`
}

export function computeRedirects(
  mdFiles: string[],
  skipStems: Set<string> = new Set(),
): Record<string, string> {
  const out: Record<string, string> = {}
  for (const f of mdFiles) {
    if (f === 'SUMMARY.md') continue
    // Only top-level files can shadow a sibling Astro pages/ directory at the
    // site root, so the skip check is scoped to those.
    const isTopLevel = !f.includes('/')
    const stem = f.replace(/\.md$/, '')
    if (isTopLevel && skipStems.has(stem)) continue
    const html = f.replace(/\.md$/, '.html')
    out[html] = TARGET
  }
  return out
}

function walk(dir: string, prefix = ''): string[] {
  const out: string[] = []
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry)
    const rel = prefix ? `${prefix}/${entry}` : entry
    if (statSync(full).isDirectory()) out.push(...walk(full, rel))
    else if (entry.endsWith('.md')) out.push(rel)
  }
  return out
}

function topLevelPageDirs(): Set<string> {
  // GitHub Pages serves /foo by checking foo.html before foo/index.html, so a
  // redirect we write at public/foo.html would shadow an Astro-built page at
  // pages/foo/index.html. Skip any redirect whose stem matches one of these.
  const pagesDir = join(__dirname, '..', 'src', 'pages')
  return new Set(
    readdirSync(pagesDir, { withFileTypes: true })
      .filter((d) => d.isDirectory())
      .map((d) => d.name),
  )
}

function main() {
  // Source-of-truth = the migrated docs/ tree under site/src/content/docs
  const docsSrc = join(__dirname, '..', 'src', 'content', 'docs')
  const mdFiles = walk(docsSrc)
  // index.md would emit public/index.html and shadow the root redirect page
  // (src/pages/index.astro → the hub marketing URL). Skip it so the root keeps
  // pointing at marketing rather than the docs index.
  const skip = topLevelPageDirs().add('index')
  const mapped = computeRedirects(mdFiles, skip)
  for (const [oldPath, newPath] of Object.entries(mapped)) {
    const dest = join(PUBLIC_DIR, oldPath)
    mkdirSync(dirname(dest), { recursive: true })
    writeFileSync(dest, redirectHtml(newPath))
  }
  const skipped = mdFiles.filter((f) => !f.includes('/') && skip.has(f.replace(/\.md$/, '')))
  if (skipped.length > 0) {
    console.log(`[generate-doc-redirects] skipped ${skipped.length} colliding: ${skipped.join(', ')}`)
  }
  console.log(`[generate-doc-redirects] wrote ${Object.keys(mapped).length} redirect files`)
}

if (import.meta.main) main()
