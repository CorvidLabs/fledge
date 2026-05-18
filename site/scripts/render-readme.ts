import { marked } from 'marked'
import DOMPurify from 'isomorphic-dompurify'

export interface RenderOptions {
  /** GitHub repo URL, e.g. https://github.com/CorvidLabs/fledge-plugin-foo */
  repoUrl?: string
  /** Default branch, e.g. "main" */
  defaultBranch?: string
}

function isExternal(url: string): boolean {
  return /^[a-z][a-z0-9+.-]*:/i.test(url) || url.startsWith('//') || url.startsWith('mailto:')
}

/**
 * Rewrite relative links/images in plugin READMEs to point at GitHub so they
 * resolve instead of 404'ing under the site's plugin route.
 *
 * Why: marked emits hrefs verbatim. A README `[LICENSE](./LICENSE)` becomes
 * `<a href="./LICENSE">` and lands at `/fledge/plugins/<slug>/LICENSE`, which
 * doesn't exist. Rewriting to the GitHub blob URL keeps the link useful.
 */
function rewriteRelative(html: string, repoUrl: string, branch: string): string {
  // Strip trailing slash from repo URL so blob path joins cleanly.
  const repo = repoUrl.replace(/\/$/, '')
  const blobBase = `${repo}/blob/${branch}/`
  const rawBase = repo.replace(/^https:\/\/github\.com\//, 'https://raw.githubusercontent.com/') + `/${branch}/`

  // Rewrite href= and src= for relative URLs only (skip absolute, mailto, fragments).
  return html.replace(/\b(href|src)="([^"]+)"/g, (match, attr, url) => {
    if (!url || url.startsWith('#')) return match
    if (isExternal(url)) return match
    if (url.startsWith('/')) return match // absolute path, leave alone
    // Normalize a leading ./ for cleaner URLs
    const cleaned = url.startsWith('./') ? url.slice(2) : url
    const base = attr === 'src' ? rawBase : blobBase
    return `${attr}="${base}${cleaned}"`
  })
}

export function renderReadme(
  markdown: string | null | undefined,
  opts: RenderOptions = {},
): string {
  if (!markdown) return ''
  const rawHtml = marked.parse(markdown, { async: false })
  let cleaned = DOMPurify.sanitize(rawHtml, {
    USE_PROFILES: { html: true },
    FORBID_TAGS: ['style'],
    FORBID_ATTR: ['style'],
  })
  if (opts.repoUrl && opts.defaultBranch) {
    cleaned = rewriteRelative(cleaned, opts.repoUrl, opts.defaultBranch)
  }
  cleaned = rewriteLegacyDocLinks(cleaned)
  return cleaned
}

/**
 * Several plugin READMEs hardcode the old mdBook URL
 * `https://corvidlabs.github.io/fledge/plugin-protocol.html` for the plugin
 * protocol page. That page doesn't exist on the Astro site; the equivalent
 * lives at `/fledge/docs/plugins#plugin-protocol-fledge-v1`. Until the plugin
 * READMEs are updated upstream, rewrite at render time.
 */
function rewriteLegacyDocLinks(html: string): string {
  return html.replace(
    /https:\/\/corvidlabs\.github\.io\/fledge\/plugin-protocol\.html/g,
    'https://corvidlabs.github.io/fledge/docs/plugins#plugin-protocol-fledge-v1',
  )
}
