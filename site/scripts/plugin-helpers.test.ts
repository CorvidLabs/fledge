import { describe, test, expect } from 'bun:test'
import { slugFromName, inferLanguage, inferTrustTier } from './plugin-helpers'

describe('slugFromName', () => {
  test('strips fledge-plugin- prefix', () => {
    expect(slugFromName('fledge-plugin-sql')).toBe('sql')
  })
  test('handles multi-word names', () => {
    expect(slugFromName('fledge-plugin-todo-scan')).toBe('todo-scan')
  })
  test('returns the name unchanged if no prefix', () => {
    expect(slugFromName('fledge-deploy')).toBe('fledge-deploy')
  })
  test('throws on empty input', () => {
    expect(() => slugFromName('')).toThrow()
  })
})

describe('inferLanguage', () => {
  test('Cargo.toml → rust', () => {
    expect(inferLanguage(['README.md', 'Cargo.toml', 'src/main.rs'])).toBe('rust')
  })
  test('package.json → ts when tsconfig present', () => {
    expect(inferLanguage(['README.md', 'package.json', 'tsconfig.json'])).toBe('ts')
  })
  test('package.json without tsconfig → js', () => {
    expect(inferLanguage(['README.md', 'package.json'])).toBe('js')
  })
  test('go.mod → go', () => {
    expect(inferLanguage(['README.md', 'go.mod'])).toBe('go')
  })
  test('pyproject.toml → python', () => {
    expect(inferLanguage(['README.md', 'pyproject.toml'])).toBe('python')
  })
  test('only shell files → shell', () => {
    expect(inferLanguage(['README.md', 'install.sh'])).toBe('shell')
  })
  test('unknown → other', () => {
    expect(inferLanguage(['README.md'])).toBe('other')
  })
})

describe('inferTrustTier', () => {
  test('CorvidLabs owner → official', () => {
    expect(inferTrustTier('CorvidLabs', [])).toBe('official')
  })
  test('non-CorvidLabs with no experimental topic → community', () => {
    expect(inferTrustTier('alice', ['cli', 'rust'])).toBe('community')
  })
  test('experimental topic → experimental regardless of owner', () => {
    expect(inferTrustTier('CorvidLabs', ['fledge-plugin-experimental'])).toBe('experimental')
    expect(inferTrustTier('bob', ['fledge-plugin-experimental'])).toBe('experimental')
  })
})
