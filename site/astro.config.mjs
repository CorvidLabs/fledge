import { defineConfig } from 'astro/config'
import mdx from '@astrojs/mdx'
import sitemap from '@astrojs/sitemap'
import remarkResolveDocLinks from './scripts/remark-resolve-doc-links.ts'

export default defineConfig({
  site: 'https://corvidlabs.github.io',
  base: '/fledge/',
  trailingSlash: 'never',
  integrations: [mdx(), sitemap()],
  markdown: {
    // Rewrites relative `.md` links in docs/ to absolute Astro URLs so
    // cross-page references resolve at runtime instead of 404'ing.
    remarkPlugins: [remarkResolveDocLinks],
    shikiConfig: {
      // github-dark-high-contrast passes WCAG AA for all token colors
      // (#6A737D comment color in github-dark fails 3.05:1 on its #24292e bg)
      theme: 'github-dark-high-contrast',
    },
  },
})
