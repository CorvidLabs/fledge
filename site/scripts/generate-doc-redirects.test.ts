import { describe, test, expect } from 'bun:test'
import { redirectHtml, computeRedirects } from './generate-doc-redirects'
import { HUB_DOCS } from '../src/data/hub'

describe('redirectHtml', () => {
  test('emits meta-refresh + canonical', () => {
    const html = redirectHtml(HUB_DOCS)
    expect(html).toContain(`<meta http-equiv="refresh" content="0; url=${HUB_DOCS}">`)
    expect(html).toContain('canonical')
    expect(html).toContain('location.replace')
  })

  test('includes a visible "moved to CorvidLabs" link for no-JS users', () => {
    const html = redirectHtml(HUB_DOCS)
    expect(html).toContain('This site has moved to CorvidLabs')
    expect(html).toContain(`href="${HUB_DOCS}"`)
  })
})

describe('computeRedirects', () => {
  test('maps every legacy mdBook page to the hub docs index', () => {
    const mapped = computeRedirects(['lanes.md', 'pillars.md', 'getting-started/installation.md'])
    expect(mapped['lanes.html']).toBe(HUB_DOCS)
    expect(mapped['pillars.html']).toBe(HUB_DOCS)
    expect(mapped['getting-started/installation.html']).toBe(HUB_DOCS)
  })

  test('skips top-level .md whose stem collides with an Astro page dir', () => {
    // plugins.md would emit public/plugins.html, which on GitHub Pages
    // shadows the Astro-built plugins/index.html. Skip it.
    const mapped = computeRedirects(['plugins.md', 'lanes.md'], new Set(['plugins']))
    expect(mapped['plugins.html']).toBeUndefined()
    expect(mapped['lanes.html']).toBe(HUB_DOCS)
  })

  test('skip only applies to top-level files, not nested ones', () => {
    // getting-started/plugins.md emits getting-started/plugins.html and can't
    // collide with the top-level plugins/ dir, so it stays.
    const mapped = computeRedirects(['getting-started/plugins.md'], new Set(['plugins']))
    expect(mapped['getting-started/plugins.html']).toBe(HUB_DOCS)
  })
})
