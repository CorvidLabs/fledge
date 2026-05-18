export interface MiniPlugin {
  slug: string
  language: string
  topics: readonly string[]
}

export function relatedSlugs<T extends MiniPlugin>(
  slug: string,
  universe: readonly T[],
  limit: number,
): string[] {
  const self = universe.find(p => p.slug === slug)
  if (!self) return []
  const others = universe.filter(p => p.slug !== slug)
  if (others.length === 0) return []

  const score = (p: T): number => {
    const sharedTopics = p.topics.filter(t => self.topics.includes(t)).length
    const langBonus = p.language === self.language ? 0.5 : 0
    return sharedTopics + langBonus
  }

  return others
    .map(p => ({ p, s: score(p) }))
    .sort((a, b) => b.s - a.s || a.p.slug.localeCompare(b.p.slug))
    .slice(0, limit)
    .map(x => x.p.slug)
}
