import { describe, test, expect } from 'bun:test'
import { repoToEntry } from './build-plugin-registry'

const sampleRepo = {
  name: 'fledge-plugin-sql',
  owner: { login: 'CorvidLabs' },
  description: 'Postgres + SQLite migrations',
  html_url: 'https://github.com/CorvidLabs/fledge-plugin-sql',
  default_branch: 'main',
  stargazers_count: 142,
  topics: ['database', 'postgres'],
  pushed_at: '2026-04-01T12:00:00Z',
  license: { spdx_id: 'MIT' },
  open_issues_count: 3,
}

describe('repoToEntry', () => {
  test('produces a registry entry from a GitHub repo + manifest data', () => {
    const entry = repoToEntry(sampleRepo, {
      files: ['Cargo.toml', 'README.md', 'src/main.rs'],
      version: '0.3.0',
    })
    expect(entry.name).toBe('fledge-plugin-sql')
    expect(entry.slug).toBe('sql')
    expect(entry.version).toBe('0.3.0')
    expect(entry.description).toBe('Postgres + SQLite migrations')
    expect(entry.language).toBe('rust')
    expect(entry.trust_tier).toBe('official')
    expect(entry.install).toBe('fledge plugins install CorvidLabs/fledge-plugin-sql')
    expect(entry.repo).toBe('https://github.com/CorvidLabs/fledge-plugin-sql')
    expect(entry.topics).toEqual(['database', 'postgres'])
    expect(entry.stars).toBe(142)
    expect(entry.updated_at).toBe('2026-04-01T12:00:00Z')
    expect(entry.default_branch).toBe('main')
  })

  test('falls back to "unknown" version when no manifest version is given', () => {
    const entry = repoToEntry(sampleRepo, { files: ['Cargo.toml'], version: null })
    expect(entry.version).toBe('unknown')
  })
})
