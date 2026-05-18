import { defineCollection, z } from 'astro:content'

const examples = defineCollection({
  type: 'content',
  schema: z.object({
    title: z.string(),
    tag: z.enum(['Rust CLI', 'TS + Bun', 'Python', 'Go', 'Plugins', 'AI', 'CI / CD', 'Monorepo', 'Templates']),
    steps: z.number().int().positive(),
    minutes: z.number().int().positive(),
    pillars: z.array(z.string()),
    description: z.string(),
    featured: z.boolean().default(false),
    draft: z.boolean().default(false),
    order: z.number().optional(),
  }),
})

export const collections = { examples }
