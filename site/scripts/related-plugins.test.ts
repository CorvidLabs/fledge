import { describe, test, expect } from 'bun:test'
import { relatedSlugs } from './related-plugins'

type Mini = { slug: string; language: string; topics: string[] }

const universe: Mini[] = [
  { slug: 'sql',      language: 'rust', topics: ['database', 'postgres'] },
  { slug: 'coverage', language: 'rust', topics: ['testing', 'coverage'] },
  { slug: 'bench',    language: 'rust', topics: ['testing', 'benchmarks'] },
  { slug: 'todo',     language: 'rust', topics: ['triage', 'codebase'] },
  { slug: 'deps',     language: 'ts',   topics: ['dependencies'] },
]

describe('relatedSlugs', () => {
  test('prefers shared topics', () => {
    const result = relatedSlugs('coverage', universe, 3)
    expect(result[0]).toBe('bench')   // shares "testing"
  })

  test('falls back to same-language when no topic overlap', () => {
    const result = relatedSlugs('todo', universe, 3)
    expect(result.length).toBe(3)
    expect(result).not.toContain('todo')          // never include self
    expect(result.every(s => s !== 'deps')).toBe(true)  // prefer same-language
  })

  test('returns at most `limit` entries', () => {
    expect(relatedSlugs('sql', universe, 2).length).toBe(2)
  })

  test('returns empty when only one plugin exists', () => {
    expect(relatedSlugs('sql', [universe[0]], 3)).toEqual([])
  })
})
