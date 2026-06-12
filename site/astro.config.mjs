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
      // The 'css-variables' theme emits --astro-code-* custom properties
      // instead of baked-in colors, so highlighting resolves to the brand
      // --code-* tokens (mapped in src/styles/globals.css) and themes with
      // light/dark for free. WCAG AA holds in both modes.
      theme: 'css-variables',
    },
  },
})
