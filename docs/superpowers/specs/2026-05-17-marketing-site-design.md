# Marketing site rebuild — design spec

**Status:** approved, ready for plan
**Author:** brainstormed with @0xLeif via Claude on 2026-05-17
**Replaces:** `docs/` (mdBook flow) and `.github/workflows/docs.yml`

## Goal

Replace the current mdBook-only GitHub Pages site at `https://corvidlabs.github.io/fledge/` with a marketing site that has parity with the Merlin site (`CorvidLabs/merlin`) plus a first-class plugin registry.

The site must do four jobs:

1. **Convert** first-time visitors into installers (hero, CTA, install snippets).
2. **Showcase** the plugin ecosystem (browsable, searchable registry — the most novel piece).
3. **Educate** with end-to-end walkthroughs and the migrated docs.
4. **Communicate** ongoing work (blog: releases, plugin spotlights, workflow posts, announcements).

## Non-goals

- Custom domain. Stays on `corvidlabs.github.io/fledge/` for v1. Astro `base: '/fledge/'`.
- Multi-version docs. v1 ships docs for the current `main` only.
- Search infrastructure beyond client-side. No Algolia / Pagefind in v1 — the docs sidebar is the navigation, an in-browser search UI for plugins is client-side over a baked JSON file.
- i18n / multi-language.
- User accounts, comments, analytics dashboards. (Plain GitHub Pages analytics only.)
- Preserving the existing GitHub Pages analytics setup. (We do still add new redirect rules for old top-level docs URLs — see Success criteria — but we are not migrating the existing analytics integration.)

## Information architecture

```
/                       index.astro          — Landing page (hero, terminal demo, pillars, plugins teaser, CTA)
/plugins                plugins/index.astro  — Plugin registry (search, filter, grid)
/plugins/{slug}         plugins/[slug].astro — One per plugin (optional v1; default to repo deep-link)
/examples               examples/index.astro — Walkthrough index
/examples/{slug}        examples/[slug].astro — Individual walkthrough
/docs                   docs/index.astro     — Docs landing (introduction)
/docs/{section}/{page}  docs/[...slug].astro — All migrated mdBook content
/blog                   blog/index.astro     — Post index with category filter
/blog/{slug}            blog/[slug].astro    — One post
/404                    404.astro            — Static 404
```

Top nav (in every header): **Plugins · Examples · Docs · Blog** plus a primary `Install` CTA and a GitHub link.

## Visual identity

**Family resemblance with Merlin, differentiated by color and structural rhythm.**

Shared with Merlin:
- Dark theme. Sticky nav with blur. Italic-serif accents inside sans body. Terminal block. Stat row. Footer pattern.

Differentiated:
- **Palette**: warm rust / orange instead of Merlin's cooler accent.
  - `--bg: #0c0a09` (stone-950, warmer than Merlin's slate)
  - `--bg-raised: #18120e`
  - `--border: #2a201a`
  - `--text: #f5f5f4`
  - `--text-muted: #c2b9b1` (raised from a8a29e for a11y)
  - `--text-dim: #9c948d` (raised from 78716c for a11y)
  - `--accent: #ea580c` (orange-600)
  - `--accent-bright: #fdba74` (orange-300, gradient highlight + focus ring)
  - `--accent-deep: #9a3412` (orange-800)
- **Structural**: split-grid hero (text + terminal side-by-side rather than stacked); 2×3 unified pillars grid with hairline dividers (rather than a 3-card row); large italic-serif numerals on cards.
- **Type**: SF / system sans for body; **italic serif (Iowan Old Style / Apple Garamond / Baskerville / Georgia)** for the gradient/accent words — `fly`, `nothing`, `shipped`, `real`. Mono is JetBrains Mono / SF Mono.

Logo: a 28×28 gradient tile with the letter "f" — no separate brand mark in v1 (fledge has no existing logo asset; the wordmark + tile is the wordmark).

## Component vocabulary

Reusable Astro components (in `site/src/components/`):

- `Header.astro` — nav, version pill, primary CTA, docs search (only on `/docs/*`)
- `Footer.astro` — brand block + 3 link columns
- `Button.astro` — `variant: primary | secondary | ghost`, accessible focus ring, min 44px tap target
- `Badge.astro` — pill badge (used for "v1.4 shipping", "Latest", etc.)
- `Terminal.astro` — macOS-style terminal frame with `term-prompt / term-out / term-good / term-comment` classes
- `Pillar.astro` — single pillar cell (i. — vi.)
- `PluginCard.astro` — name, version, tier tag, language tag, description, install snippet, star count
- `ExampleCard.astro` — number, tag, title, blurb, meta (steps · time · pillars)
- `PostCard.astro` — category tag, date, title, dek, author, read time
- `CategoryTag.astro` — color-coded badge (announce/plugin/release/workflow/tutorial)
- `Callout.astro` — `type: note | warn | tip`

A11y baseline for every component:
- All interactive elements have `:focus-visible` with a 2px `--accent-bright` ring at 3px offset.
- Body text ≥ 16px, secondary text ≥ 14px. No text below 12px.
- Headings in source order. Landmarks (`<header role="banner">`, `<main id="main">`, `<nav aria-label>`, `<footer role="contentinfo">`). Skip-to-main link.
- Animations wrapped in `@media (prefers-reduced-motion: no-preference)`.
- Color contrast against `--bg`: text 18.5:1, text-muted 11.4:1, text-dim 6.4:1.

## Page-by-page content

### `/` — landing

Sections, top-to-bottom:
1. **Sticky nav** with logo + version chip, primary nav, GitHub link, `Install` CTA.
2. **Hero (split grid)**: status-style badge ("31 plugins shipping in v1.4"); H1 with italic-serif gradient on the word `fly`; subtitle; two CTA buttons (`Get started`, `Browse 31 plugins`); fine-print install command. Right column: terminal block showing `templates init → lanes init → lanes run ci` and a blinking cursor.
3. **Stats row**: 4 big numerals — `31 plugins · 6 pillars · ∞ languages · 1 binary`.
4. **Six Pillars** in a 2×3 unified grid (Scaffold · Run · Spec · AI · Ship · Extend). Each cell has roman numeral, name, blurb, and an example command.
5. **Plugin spotlight strip**: 4-up preview cards with a "Browse the registry →" link.
6. **Examples teaser**: section-head + 3 cards (Rust CLI / TS+Bun / Plugins) with `tag` + `title` + `dek` + meta.
7. **CTA banner**: H2 with italic-serif "flight"; install snippet with copy button; secondary actions.
8. **Footer**.

### `/plugins` — registry

Sections:
1. Page-head: eyebrow, H1, lede, quick stats (31 plugins · 18 official · 13 community · weekly refresh).
2. Controls bar: search box (⌘K hint), tier filter chips with counts (All / Official / Community), language chips (Any / Rust / TS / Shell), sort dropdown (Most popular / Recently updated / Name / Newest).
3. Results meta row: count + grid/list layout toggle.
4. 3-column responsive grid of plugin cards. Each card: name (mono), version, **tier tag** (color-coded: green=Official, blue=Community, amber=Experimental), language tag, description, install snippet, star count.
5. Pagination.
6. Submit-CTA panel: "Built a plugin? Share it." → authoring guide.

**Data**: bake `site/src/data/plugins.json` at prebuild via `site/scripts/build-plugin-registry.ts`. Sources:
1. GitHub search for `org:CorvidLabs+fledge-plugin-*` (all official plugins).
2. An allowlist of additional owners (`site/scripts/community-allowlist.json` — array of GitHub user/org logins) whose `*-fledge-plugin-*` repos are also pulled in.

Each entry contains:

```jsonc
{
  "name": "fledge-plugin-sql",
  "slug": "sql",                       // strip "fledge-plugin-" prefix
  "version": "0.3.0",                  // from latest GitHub release tag, fall back to Cargo.toml / package.json
  "description": "...",                // from repo description, fall back to README opening paragraph
  "language": "rust",                  // detected from manifest type
  "trust_tier": "official",            // "official" if owned by CorvidLabs, "community" otherwise; "experimental" if topic includes "fledge-plugin-experimental"
  "install": "fledge plugins install CorvidLabs/fledge-plugin-sql",
  "repo": "https://github.com/CorvidLabs/fledge-plugin-sql",
  "topics": ["database", "postgres"],  // GitHub topics
  "stars": 142,
  "updated_at": "2026-05-08T...",      // ISO timestamp of last release or last commit to default branch
  "default_branch": "main"
}
```

Search/filter is client-side over this baked JSON (~30 entries × ~300 bytes = trivially small). Free-text matches name, description, topics.

Resilience: if the GitHub fetch fails or hits rate limit during a build, the script reuses the previous `plugins.json` from the cache step and emits a CI warning. Build never fails because GitHub is flaky.

### `/plugins/{slug}` (optional v1)

Default to **deep-linking to the GitHub repo** in v1 (cards link to `repo`). Per-plugin pages are a follow-up unless we have a clear need (e.g., a curated description / screenshots). Decision flagged in the implementation plan.

### `/examples` — walkthrough index

1. Page-head: eyebrow, H1, lede.
2. **Featured walkthrough** (full-width card): big text left + mini-terminal preview right showing all numbered steps. "Featured" badge in the corner.
3. Topic filter chips: All / Scaffold / Lanes / Ship / Plugins / AI / CI/CD.
4. 2-column grid of cards with a big italic-serif numeral, tag, title, blurb, meta (steps · time · pillars). Coming-soon items rendered dimmed with a dashed "Coming soon" pill.
5. Submit-CTA panel: "Have a workflow worth showing?" → walkthrough style guide.

Each walkthrough lives in `site/src/content/examples/{slug}.mdx` with frontmatter (`title`, `tag`, `steps`, `minutes`, `pillars`, `featured?`, `draft?`).

### `/examples/{slug}` — single walkthrough

Standard MDX content page with the docs article styling (max-width 800px), but no left sidebar; right-rail TOC; previous/next pager keyed to walkthrough order in frontmatter.

### `/docs` and `/docs/{...}` — docs

Three-column layout: left sidebar (sticky, scrollable) — main article (max-width 800px) — right TOC (sticky, scrollable, hides below 1100px).

Sidebar groups (mirrors current mdBook `SUMMARY.md`):
- Getting started — Introduction · Installation · Quick start · Existing projects
- The six pillars — Scaffold · Run · Spec · AI · Ship · Extend
- Reference — CLI reference · fledge.toml · Configuration · Plugin protocol · Doctor · AGENTS.md
- Resources — Template authoring · Changelog · GitHub integration

Article features:
- Breadcrumb above H1.
- Article H1, then a "lede" paragraph styled larger.
- H2 with bottom border; H3 plain.
- Inline `<code>` accent-colored. Code blocks with a language label and Copy button.
- Callout style for notes/tips/warns.
- Article footer: "Edit this page on GitHub" link + Previous/Next pager.
- Right TOC built from H2/H3 with scrollspy.

**Docs search**: ships in v1 as in-page browser search; integrated Pagefind index is a v1.1 candidate (post-launch). Header reserves the search box slot so the upgrade is purely additive.

**Content migration**: every file in `docs/src/*.md` and `docs/src/getting-started/*.md` moves to `site/src/content/docs/...md` with frontmatter (`title`, `description`, `order`). The mdBook `SUMMARY.md` is replaced by a typed Astro content collection schema.

### `/blog` and `/blog/{slug}` — blog

1. Page-head: eyebrow, H1 ("Updates, plugins, and *field notes*"), lede, RSS + Mastodon links.
2. **Category filter strip**: All / Releases / Plugins / Workflows / Tutorials / Announcements. Each chip carries a color swatch that matches the post tags. (Colors: Release=orange, Plugin=green, Workflow=blue, Tutorial=purple, Announcement=red.)
3. **Featured post** card: text left + custom visual right. "Latest" badge.
4. **3-column post grid**: category tag, date, title, dek, author avatar/name, read time.
5. Pagination.
6. **Subscribe strip**: "Get posts in your inbox" email signup. ⚠️ See open question below — we need to pick a mailing-list backend before v1 ships, else this becomes a `mailto:` placeholder.

Each post lives in `site/src/content/blog/{slug}.mdx` with frontmatter (`title`, `category`, `date`, `author`, `dek`, `readTime`, `featured?`, `draft?`).

A single post page (`/blog/{slug}`) reuses the `examples/{slug}` article layout (800px max-width, no left sidebar, right TOC, prev/next).

## Repo layout

```
site/
  astro.config.mjs        site: 'https://corvidlabs.github.io', base: '/fledge/'
  package.json            Bun-driven; scripts: dev, prebuild, build, preview, fmt, lint
  bun.lock
  tsconfig.json
  public/                 favicon, og:image, robots.txt
  scripts/
    build-plugin-registry.ts    GH API → site/src/data/plugins.json
  src/
    components/           See "Component vocabulary"
    layouts/
      BaseLayout.astro    nav + footer + skip-link + meta
      ArticleLayout.astro article column + (optional) TOC + prev/next pager
      DocsLayout.astro    BaseLayout + sidebar + ArticleLayout
    content/
      config.ts           content-collection schemas (docs, blog, examples)
      docs/               migrated mdBook content (*.md)
      blog/               *.mdx
      examples/           *.mdx
    data/
      plugins.json        baked at prebuild
    pages/
      index.astro
      404.astro
      plugins/
        index.astro
        [slug].astro      (deferred — default behavior is to redirect to repo)
      examples/
        index.astro
        [slug].astro
      docs/
        index.astro
        [...slug].astro
      blog/
        index.astro
        [slug].astro
    styles/
      globals.css         CSS custom properties (the rust palette), reset, focus-visible rules
```

`docs/` (current mdBook tree) is **deleted** after the migration commit lands. `.github/workflows/docs.yml` is deleted in the same commit and replaced with `.github/workflows/pages.yml`.

## Build & deploy

`.github/workflows/pages.yml`:
- Triggers on push to `main` when `site/**` changes, on `workflow_dispatch`, **and** on a weekly schedule (`cron: '0 8 * * 1'`) so the plugin registry refreshes without a code change.
- Jobs: `build` (checkout → setup Bun → `bun install --frozen-lockfile` → `bun run prebuild` with `GH_TOKEN` from `secrets.GITHUB_TOKEN` to dodge the 60/hr unauth rate limit → `bun run build` → upload `site/dist`); `deploy` (GitHub Pages OIDC).
- Pages cache step keeps the previous `plugins.json` around so the prebuild can fall back on it during a GitHub outage.
- Runner: `ubuntu-latest`. Merlin uses self-hosted; fledge defaults to GitHub-hosted unless that becomes a problem.

## Open questions (must answer before plan)

1. **Mailing list provider**: Buttondown, Beehiiv, ConvertKit, or a `mailto:` placeholder until v1.1? — affects whether the subscribe form is functional at launch.
2. **Per-plugin pages**: ship `/plugins/{slug}` in v1 (more work, lets us add curated content per plugin), or defer until a real need surfaces? Default: defer; cards link to repo.
3. **Docs search**: ship in v1 with Pagefind (~minor extra build step), or ship browser-only search and add later? Default: defer.
4. **Authentication for plugin registry fetch**: should the workflow create a GH App token for higher rate limits, or is `${{ secrets.GITHUB_TOKEN }}` (5000/hr) enough for ~30 plugins? Default: use the workflow token; revisit if we cross ~500 plugins.

## Success criteria

- All four nav pages live at `/`, `/plugins`, `/examples`, `/docs`, `/blog`.
- Plugin registry rebuilds weekly without manual intervention; surfaces every `CorvidLabs/fledge-plugin-*` repo automatically.
- Lighthouse score ≥ 95 on Performance, Accessibility, Best Practices, SEO (mobile + desktop) for `/`, `/plugins`, and a representative `/docs/{page}`.
- Migrated docs load with the same content as today's mdBook site. Old top-level URLs (`/fledge/getting-started.html`, etc.) return either matching content at the new path or a 1-step redirect (via a generated static HTML redirect file in `site/public/`), **not** a 404.
- `cargo install fledge` snippet copies to clipboard in one click from the home and CTA.
- `prefers-reduced-motion: reduce` produces a static (no-motion) page.
- Site survives a GitHub API outage during build (uses cached `plugins.json`).
