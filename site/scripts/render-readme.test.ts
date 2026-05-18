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

  test('rewrites relative links to GitHub blob URL when repo opts given', () => {
    const html = renderReadme('See [LICENSE](./LICENSE) and [docs](docs/api.md).', {
      repoUrl: 'https://github.com/CorvidLabs/fledge-plugin-x',
      defaultBranch: 'main',
    })
    expect(html).toContain('href="https://github.com/CorvidLabs/fledge-plugin-x/blob/main/LICENSE"')
    expect(html).toContain('href="https://github.com/CorvidLabs/fledge-plugin-x/blob/main/docs/api.md"')
  })

  test('rewrites relative image src to raw.githubusercontent.com', () => {
    const html = renderReadme('![logo](./logo.png)', {
      repoUrl: 'https://github.com/CorvidLabs/fledge-plugin-x',
      defaultBranch: 'main',
    })
    expect(html).toContain(
      'src="https://raw.githubusercontent.com/CorvidLabs/fledge-plugin-x/main/logo.png"',
    )
  })

  test('leaves absolute and external links alone when rewriting', () => {
    const html = renderReadme(
      '[ext](https://example.com/x) [root](/about) [hash](#section)',
      {
        repoUrl: 'https://github.com/CorvidLabs/fledge-plugin-x',
        defaultBranch: 'main',
      },
    )
    expect(html).toContain('href="https://example.com/x"')
    expect(html).toContain('href="/about"')
    expect(html).toContain('href="#section"')
  })

  test('does not rewrite when opts missing (back-compat)', () => {
    const html = renderReadme('[LICENSE](./LICENSE)')
    expect(html).toContain('href="./LICENSE"')
  })

  test('rewrites legacy mdBook plugin-protocol URL to the Astro docs anchor', () => {
    const html = renderReadme(
      'Uses the [fledge-v1 protocol](https://corvidlabs.github.io/fledge/plugin-protocol.html).',
    )
    expect(html).toContain('href="https://corvidlabs.github.io/fledge/docs/plugins#plugin-protocol-fledge-v1"')
    expect(html).not.toContain('plugin-protocol.html')
  })
})
