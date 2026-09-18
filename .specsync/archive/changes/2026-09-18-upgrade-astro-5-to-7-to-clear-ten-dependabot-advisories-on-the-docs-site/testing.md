---
change: upgrade-astro-5-to-7-to-clear-ten-dependabot-advisories-on-the-docs-site
artifact: testing
---

# Testing

## How equivalence was established

A full `bun run build` was captured **before** any dependency change and its emitted file
set recorded (109 files, 102 HTML pages). Every subsequent build was diffed against it.

| Check | Baseline | After upgrade |
|---|---|---|
| `bun run build` | exit 0, 81 pages | exit 0, 81 pages |
| Emitted file set | 109 files | 109 files, `diff` empty |
| HTML pages carrying the hub redirect | 102 / 102 | 102 / 102 |
| `bun test` | 49 pass | 49 pass |
| `astro check` | 0 errors | 0 errors, 0 warnings |

The file-set diff is the load-bearing check. It is what caught the `.mdx` glob miss that a
green build hid.

## Rejection signals

- Any HTML page in `dist/` that does not contain `location.replace` — a retired URL that
  stopped redirecting is the one user-visible failure this site can have.
- A page count other than 81, or any file present in one build's output and not the
  other's.
- `astro` resolving below 7.2.8, which would leave at least the Critical advisory open.

## Not covered

- No runtime/browser test of the redirect; the assertion is on emitted HTML.
- `site/src/data/plugins.json` is regenerated from the GitHub API by `prebuild` and was
  deliberately reverted — an unauthenticated build hit the rate limit and produced
  degraded entries (`"version": "unknown"`, `"language": "other"`). It is not part of this
  change.
