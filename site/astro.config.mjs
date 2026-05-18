import { defineConfig } from 'astro/config'
import mdx from '@astrojs/mdx'
import sitemap from '@astrojs/sitemap'

export default defineConfig({
  site: 'https://corvidlabs.github.io',
  base: '/fledge/',
  trailingSlash: 'never',
  integrations: [mdx(), sitemap()],
  markdown: {
    shikiConfig: {
      // github-dark-high-contrast passes WCAG AA for all token colors
      // (#6A737D comment color in github-dark fails 3.05:1 on its #24292e bg)
      theme: 'github-dark-high-contrast',
    },
  },
})
