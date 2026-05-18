import { describe, test, expect } from 'bun:test'
import { resolveLink, buildDocIndex } from './remark-resolve-doc-links'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const DOCS_DIR = join(__dirname, '..', 'src', 'content', 'docs')

describe('buildDocIndex', () => {
  const idx = buildDocIndex(DOCS_DIR)

  test('indexes top-level docs by stem', () => {
    expect(idx.byStem.get('lanes')).toBe('lanes')
    expect(idx.byStem.get('plugins')).toBe('plugins')
    expect(idx.byStem.get('spec')).toBe('spec')
  })

  test('indexes nested docs by stem', () => {
    expect(idx.byStem.get('fledge-toml')).toBe('reference/fledge-toml')
    expect(idx.byStem.get('cli-reference')).toBe('reference/cli-reference')
    expect(idx.byStem.get('changelog')).toBe('resources/changelog')
  })

  test('relStem set contains full relative paths without extension', () => {
    expect(idx.byRelStem.has('lanes')).toBe(true)
    expect(idx.byRelStem.has('reference/fledge-toml')).toBe(true)
    expect(idx.byRelStem.has('getting-started/quick-start')).toBe(true)
  })
})

describe('resolveLink', () => {
  const idx = buildDocIndex(DOCS_DIR)

  test('strips .md from same-dir relative link that resolves cleanly', () => {
    // lanes.md -> ./plugins.md (both at docs root)
    expect(resolveLink('./plugins.md', 'lanes.md', idx)).toBe('/fledge/docs/plugins')
  })

  test('redirects wrong-folder .md link to the actual location', () => {
    // lanes.md says ./fledge-toml.md but the file lives in reference/
    expect(resolveLink('./fledge-toml.md', 'lanes.md', idx)).toBe('/fledge/docs/reference/fledge-toml')
    expect(resolveLink('./configuration.md', 'lanes.md', idx)).toBe('/fledge/docs/reference/configuration')
    expect(resolveLink('./cli-reference.md', 'lanes.md', idx)).toBe('/fledge/docs/reference/cli-reference')
  })

  test('preserves anchors', () => {
    expect(resolveLink('./cli-reference.md#ai-ask-and-review', 'ai.md', idx)).toBe(
      '/fledge/docs/reference/cli-reference#ai-ask-and-review',
    )
  })

  test('handles ../ links', () => {
    // getting-started/quick-start.md -> ../templates.md
    expect(resolveLink('../templates.md', 'getting-started/quick-start.md', idx)).toBe(
      '/fledge/docs/templates',
    )
  })

  test('handles nested-to-nested via basename fallback', () => {
    // reference/fledge-toml.md says ./plugins.md but plugins is at root
    expect(resolveLink('./plugins.md', 'reference/fledge-toml.md', idx)).toBe('/fledge/docs/plugins')
  })

  test('handles same-folder nested link via direct relative match', () => {
    // resources/github-integration.md -> ./ship.md (ship is at root, not in resources/)
    expect(resolveLink('./ship.md', 'resources/github-integration.md', idx)).toBe('/fledge/docs/ship')
  })

  test('leaves external URLs alone', () => {
    expect(resolveLink('https://example.com/foo.md', 'lanes.md', idx)).toBeNull()
    expect(resolveLink('mailto:hi@example.com', 'lanes.md', idx)).toBeNull()
  })

  test('leaves already-absolute URLs alone', () => {
    expect(resolveLink('/fledge/docs/plugins', 'lanes.md', idx)).toBeNull()
  })

  test('leaves non-.md links alone', () => {
    expect(resolveLink('./plugins', 'lanes.md', idx)).toBeNull()
    expect(resolveLink('#section', 'lanes.md', idx)).toBeNull()
  })

  test('custom base URL', () => {
    expect(resolveLink('./plugins.md', 'lanes.md', idx, '/other/')).toBe('/other/docs/plugins')
  })

  test('last-resort branch emits a console.warn and returns stripped URL', () => {
    const warnings: string[] = []
    const orig = console.warn
    console.warn = (m: string) => warnings.push(m)
    let result: string | null
    try {
      // 'nonexistent-file.md' has no match in byStem or byRelStem so it hits the last-resort path.
      result = resolveLink('./nonexistent-file.md', 'lanes.md', idx, '/fledge/', 'lanes.md')
    } finally {
      console.warn = orig
    }
    expect(result).toBe('/fledge/docs/nonexistent-file')
    expect(warnings.length).toBe(1)
    expect(warnings[0]).toContain('nonexistent-file.md')
    expect(warnings[0]).toContain('lanes.md')
  })
})
