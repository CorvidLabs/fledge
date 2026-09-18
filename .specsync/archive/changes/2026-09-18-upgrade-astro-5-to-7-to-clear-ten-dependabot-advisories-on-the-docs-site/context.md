---
change: upgrade-astro-5-to-7-to-clear-ten-dependabot-advisories-on-the-docs-site
artifact: context
---

# Context

## What led here

Ten open Dependabot advisories against `astro` in `site/package.json`, one Critical
(RCE through AVIF image optimization), two High, five Moderate, two Low. The site was
pinned at `astro@^5.0.0`, resolving to 5.18.1.

The advisories' own `first_patched_version` fields span 6.1.6 through 7.2.8, so clearing
all ten requires **7.2.8 or later** — a two-major jump, plus `@astrojs/mdx` 4 to 8.

## Exposure, assessed before deciding scope

The fledge site is a **retired redirect stub**. All eleven routes render
`src/components/Redirect.astro` with a build-time constant hub URL; the content
collections exist only to enumerate retired URLs so each still builds a redirect page.
Output is static (no `output: server`).

On that footing, none of the ten are meaningfully exploitable:

- **#9 Critical, AVIF RCE** — unreachable. No `<Image`, `getImage`, or `astro:assets`
  anywhere in `site/src`.
- **#4 High, Host-header SSRF in prerendered error page fetch** — unreachable. Static
  output, no server runtime.
- **#2 Low, server island replay** — unreachable. No `server:defer`.
- **#6/#7, View Transition XSS** — unreachable. Every `transition:` occurrence is a CSS
  property in a `<style>` block, not an Astro `transition:*` directive.
- **#10, authorization bypass when stripping the configured base** — no authorization
  exists on a static site.
- **#1/#3/#5/#8, reflected XSS** — a static site has no request to reflect. The one
  `define:vars` use passes a hardcoded hub URL.

The upgrade was done anyway, because carrying ten open advisories is its own cost and the
site still builds and deploys. But the Critical label did not justify rushing: the
assessment came first, and it is recorded here so the next person does not redo it.

## Breaking changes actually hit

1. **`unified` is no longer bundled.** Astro 7 ships a different default Markdown
   processor, so `markdown.remarkPlugins` needs `@astrojs/markdown-remark` installed
   explicitly. A stale transitive 6.3.11 was being resolved, which failed as
   `unified is not a function`. Fixed by adding `@astrojs/markdown-remark@^7.3.1`.
2. **Legacy content config removed.** `src/content/config.ts` with `type: 'content'`
   moved to `src/content.config.ts` with a `glob()` loader per collection.
3. **`entry.slug` removed** in favour of `entry.id` under the Content Layer API.

## The trap worth remembering

The first loader pass used `pattern: '**/*.md'` for all three collections and the build
went green — at **76 pages instead of 81**. `blog/` and `examples/` hold `.mdx`, not
`.md`, so five pages silently vanished. A green build was not evidence of correctness;
only diffing the emitted file set against a pre-upgrade baseline caught it. The pattern
is now `'**/*.{md,mdx}'` for those two collections.
