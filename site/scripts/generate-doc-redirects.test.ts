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
})
