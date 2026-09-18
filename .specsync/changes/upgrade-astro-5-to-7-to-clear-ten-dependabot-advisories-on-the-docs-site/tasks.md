---
change: upgrade-astro-5-to-7-to-clear-ten-dependabot-advisories-on-the-docs-site
artifact: tasks
---

# Tasks

- [x] Pull `first_patched_version` for all ten advisories from the Dependabot API; confirm
      7.2.8 is the floor that clears every one
- [x] Assess reachability per advisory against the retired-stub architecture before
      committing to a two-major migration
- [x] Capture a pre-upgrade build baseline (file set, page count, redirect presence)
- [x] Upgrade `astro` 5.18.1 to 7.3.3, `@astrojs/mdx` 4 to 8, `@astrojs/sitemap` to 3.7.4
- [x] Add `@astrojs/markdown-remark@^7.3.1` — Astro 7 no longer bundles the unified pipeline
- [x] Migrate `src/content/config.ts` to `src/content.config.ts` with `glob()` loaders
- [x] Replace `entry.slug` with `entry.id` across the three dynamic routes and two components
- [x] Fix the `.md`-only glob that silently dropped five `.mdx` pages
- [x] Verify the emitted file set is identical to the baseline
- [x] `bun test` 49/49, `astro check` 0 errors / 0 warnings
- [ ] Definition approval — owner gate
