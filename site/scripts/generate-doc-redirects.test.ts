import { describe, test, expect } from 'bun:test'
import { redirectHtml, computeRedirects } from './generate-doc-redirects'

describe('redirectHtml', () => {
  test('emits meta-refresh + canonical', () => {
    const html = redirectHtml('/fledge/docs/lanes')
    expect(html).toContain('<meta http-equiv="refresh" content="0; url=/fledge/docs/lanes">')
    expect(html).toContain('canonical')
    expect(html).toContain('location.replace')
  })
})

describe('computeRedirects', () => {
  test('maps top-level mdBook pages to new docs paths', () => {
    const mapped = computeRedirects(['lanes.md', 'pillars.md', 'getting-started/installation.md'])
    expect(mapped['lanes.html']).toBe('/fledge/docs/lanes')
    expect(mapped['pillars.html']).toBe('/fledge/docs/pillars')
    expect(mapped['getting-started/installation.html']).toBe('/fledge/docs/getting-started/installation')
  })

  test('skips top-level .md whose stem collides with an Astro page dir', () => {
    // plugins.md would emit public/plugins.html, which on GitHub Pages
    // shadows the Astro-built plugins/index.html (the registry). Skip it.
    const mapped = computeRedirects(['plugins.md', 'lanes.md'], new Set(['plugins']))
    expect(mapped['plugins.html']).toBeUndefined()
    expect(mapped['lanes.html']).toBe('/fledge/docs/lanes')
  })

  test('skip only applies to top-level files, not nested ones', () => {
    // getting-started/plugins.md emits getting-started/plugins.html and can't
    // collide with the top-level plugins/ dir, so it stays.
    const mapped = computeRedirects(['getting-started/plugins.md'], new Set(['plugins']))
    expect(mapped['getting-started/plugins.html']).toBe('/fledge/docs/getting-started/plugins')
  })
})
