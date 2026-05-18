import { describe, test, expect } from 'bun:test'
import { renderReadme } from './render-readme'

describe('renderReadme', () => {
  test('renders headings and paragraphs', () => {
    const html = renderReadme('# Title\n\nHello *world*.')
    expect(html).toContain('<h1>Title</h1>')
    expect(html).toContain('<em>world</em>')
  })

  test('strips inline script tags', () => {
    const html = renderReadme('<script>alert(1)</script>\nHello.')
    expect(html).not.toContain('<script')
    expect(html).toContain('Hello.')
  })

  test('strips on* event handlers', () => {
    const html = renderReadme('<a href="#" onclick="alert(1)">x</a>')
    expect(html).not.toContain('onclick')
  })

  test('keeps code fences with language', () => {
    const html = renderReadme('```rust\nfn main() {}\n```')
    expect(html).toContain('<pre>')
    expect(html).toContain('<code')
    expect(html).toContain('fn main() {}')
  })

  test('returns empty string for null/empty input', () => {
    expect(renderReadme('')).toBe('')
    expect(renderReadme(null as unknown as string)).toBe('')
  })
})
