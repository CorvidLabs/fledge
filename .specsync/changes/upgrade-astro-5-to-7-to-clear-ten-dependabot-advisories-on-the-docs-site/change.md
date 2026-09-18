---
id: upgrade-astro-5-to-7-to-clear-ten-dependabot-advisories-on-the-docs-site
state: accepted
type: bug_fix
base_commit: d3d710b10501c5576d946556fd10071edb3adafe
---

# Upgrade Astro 5 to 7 to clear ten Dependabot advisories on the docs site

## Intent

Upgrade Astro 5 to 7 to clear ten Dependabot advisories on the docs site

## Affected Canonical Specs

- None

## Acceptance Criteria

- astro resolves to >= 7.2.8, the version that clears all ten open Dependabot advisories; bun run build emits a byte-identical file set to the pre-upgrade baseline (109 files, 102 html pages, all containing the hub redirect); bun test passes 49/49; astro check reports 0 errors and 0 warnings

## No-spec Rationale

site/ is outside source_dirs (src, templates), so no canonical spec covers the docs site; this is a dependency upgrade with no change to fledge's own contracts
