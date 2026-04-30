# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.17.0] - 2026-04-29

### Chores

- scope schema_version per-command via named constants (#297) (faeaefe)
- update Homebrew formula to v0.16.0 (#288) (8d8bd56)

### Documentation

- update framing to lead with what fledge does, not how it's built (#306) (4f5fa17)
- update CLAUDE.md architecture section for post-refactor module structure (#304) (8ecab3a)
- update documentation for v0.16.0 changes (#295) (11b99fd)

### Features

- add interactive `fledge config edit` command (d21e372)

### Fixes

- update spec files to reference new module paths post-refactor (#305) (fe2924a)
- pre-1.0 security hardening (#296) (c9562d6)
- resolve nested specs + organize change logs (closes #291) (#294) (36b7c49)
- normalize Ollama host to include scheme (#293) (5847846)
- security hardening for 1.0 — hook gates + output caps (#289) (bc3be9a)

### Refactoring

- split plugin.rs (105KB) into plugin/ folder module (#298) (af8b490)
- split lanes.rs (87KB) into lanes/ folder module (#299) (92f246d)
- split release.rs (57KB) into release/ folder module (#303) (596fcd3)
- split protocol.rs (65KB) into protocol/ folder module (#301) (aa3f60f)
- split spec.rs (62KB) into spec/ folder module (#302) (89e0ec9)
- extract CLI types and handlers into sibling modules (#300) (3cd93d7)

## [v0.16.0] - 2026-04-26

### Chores

- lock 1.0 contracts (introspect schema, plugin protocol, lanes, llm, trust) (#270) (1360fc3)
- update Homebrew formula to v0.15.3 (#269) (f4d95b3)

### Documentation

- close AGENTS.md envelope coverage gaps (#286) (24ef2fe)
- sync README/AGENTS/docs to current state, drop em-dashes (#280) (1d7ac1c)
- sync AGENTS.md + spec behavioral examples to envelope shape (#279) (52805a5)

### Features

- add --json flag (#281) (e30a6a3)
- tier-D schema_version envelope on cross-cutting --json paths (#278) (0291aa8)
- wrap --json outputs in schema_version envelope (tier C, BREAKING) (#274) (71f237a)
- --json coverage across plugins, lanes, templates (tier A + B) (#273) (296064f)

### Fixes

- finalize envelope shapes before tagging (#285) (d5be3da)
- silence pre-release lane on --json (single-envelope contract) (#284) (0dce714)
- propagate --json to pre-release lane, document release JSON shapes (#283) (4c5b0bb)
- address 1.0 readiness blockers from multi-model review (#282) (6b1234b)
- honor --json on dry-run path (1.0 contract) (#277) (b781561)
- tighten lanes import + templates list envelopes (#271 followups) (#276) (3d9799d)

## [Unreleased]

### Added

- **Schema-versioned `--json` envelope** across every public-surface JSON output in plugins, lanes, and templates. Each output is now a JSON object with a top-level `schema_version: 1` field. Tier B (#271) added `--json` to commands that didn't have it; this entry tracks tier C (#272), the breaking migration for the ones that did.

### Changed (BREAKING)

- **`--json` outputs in plugins/lanes/templates wrapped in `schema_version` envelope** (#272). The following commands previously returned a top-level JSON array; they now return a JSON object with the array under a named key. **Update any `jq` paths in your scripts.**

  | Command | Before | After |
  |---|---|---|
  | `fledge plugins list --json` | `[…]` | `{schema_version: 1, plugins: […]}` |
  | `fledge plugins audit --json` | `[…]` | `{schema_version: 1, audit: […]}` |
  | `fledge plugins search --json` | `[…]` | `{schema_version: 1, results: […]}` |
  | `fledge plugins validate --json` | `{path, plugin_name, errors, warnings}` | adds `schema_version: 1` (additive) |
  | `fledge lanes list --json` | `[…]` | `{schema_version: 1, lanes: […]}` |
  | `fledge lanes search --json` | `[…]` | `{schema_version: 1, results: […]}` |
  | `fledge lanes run --json` | `{lane, success, …}` | adds `schema_version: 1` (additive) |
  | `fledge lanes validate --json` | `{path, lane_count, errors, warnings}` | adds `schema_version: 1` (additive) |
  | `fledge templates search --json` | `[…]` | `{schema_version: 1, results: […]}` |
  | `fledge templates validate --json` | `[{report}, …]` | `{schema_version: 1, reports: […]}` |

  **Migration:** scripts using `jq '.[]'` against any of the above need to update to `jq '.<resource>[]'` — `.plugins[]`, `.audit[]`, `.results[]`, `.lanes[]`, `.reports[]`. The named key is fully discoverable from the command-to-resource mapping above.

  **Why now.** `schema_version` cannot be added to a top-level array additively. Doing this in 0.x is the *last* time it's free; once 1.0 ships, the top-level shape is frozen and a future migration would require a major version bump. The matching `schema_version: 1` field on already-object outputs (validate / lane run) is purely additive — no script breakage there.

  **Future evolution.** Within `schema_version: 1`, new fields are additive — consumers must ignore unknown keys. Removing or retyping a field bumps `schema_version` to `2`.

### Spec bumps

- `plugin` v14 → v15
- `lanes` v11 → v12
- `main` v7 → v8
- `validate` v1 → v2

## [v0.15.3] - 2026-04-25

**Dogfooding-driven patch.** Two real bugs hit while shipping `fledge-plugin-github` v0.2.0 — both fixed in this release. Plus a sweep of the docs that had drifted past v0.15.2 and a per-spec test-file split.

### Added

- **`fledge release` recognizes `plugin.toml`** (#264, #265) — `[plugin].version` is now a first-class version source. Plugin authors no longer have to hand-bump `plugin.toml` and pass an explicit version: `fledge release minor` Just Works inside a plugin repo. The bumper is section-scoped (a `version` key inside `[[commands]]` or any other table is left alone). Rust plugins with both `Cargo.toml` and `plugin.toml` get both bumped together.
- **`--no-bump` flag** for `fledge release` — tag-only release, useful when the canonical version lives outside the working tree (e.g. the GitHub Release tag itself).
- **`fledge plugins validate`** now flags `plugin.version` strings that don't parse as semver. The existing empty-version error is unchanged.
- **`FLEDGE_PLUGIN_DIR` environment variable** (#266, #267) — exported by fledge before exec'ing any plugin binary, lifecycle hook, or fledge-v1 protocol plugin. Set to the plugin's source directory (canonicalized, absolute). Multi-file shell plugins should reach sibling helpers via `"$FLEDGE_PLUGIN_DIR/bin/<helper>"` instead of `dirname "$0"` (which resolves to the shared `plugins/bin/` symlink dir, not the plugin's source). Closes a quiet contract that bit `fledge-plugin-github` v0.2.0 and that every multi-file shell plugin author would otherwise rediscover.
- **`fledge plugins create` scaffold** updated — generated entry-point script now uses `${FLEDGE_PLUGIN_DIR:?...}` and ships a commented dispatcher example. New plugins get the right pattern by default.

### Changed

- **`tests/integration.rs` (2749 LOC monolith) split into nine per-spec files** matching the `specs/` layout: `tests/{templates,config,run,lanes,doctor,validate,spec,changelog,main}.rs`. Shared helpers extracted to `tests/common/mod.rs`. All 157 integration tests pass under their new file homes.
- Two dummy `tests/*.test.ts` files (placeholder bun tests with no actual integration) deleted — fledge has no bun harness, the files served no purpose.

### Fixed

- **Documentation accuracy across the whole tree** (#263). Every reference to `templates-search` / `templates-publish` (hyphenated, plugin-attributed) updated to `templates search` / `publish` (core subcommands as of v0.15.2). Every "five plugins" claim corrected to "three". Every `doctor-tools` plugin reference removed (re-absorbed into core's `Toolchains` section). `fledge doctor --json` shape annotated with the new `informational: bool` per-section field. Per-spec drift fixed in `doctor`, `search`, `publish`, `templates`, `main`, `spinner`, `plugin`, `work`. Includes typo cleanup: `pluginss` (×6), duplicate-numbered invariant 9, and a stale `fledge --version → fledge 0.8.0` example.
- **CHANGELOG itself** had been silently drifting (entries in semver-disorder, three tags missing entirely, `v` prefix inconsistent). Restored to strict descending order; backfilled v0.2.0, v0.2.1, v0.6.1; removed a stale `[Unreleased]` placeholder; normalized formatting.

### Spec bumps

- `release` v1 → v2 (plugin.toml support, `--no-bump`)
- `plugin` v11 → v12 (`FLEDGE_PLUGIN_DIR` runtime contract; `plugin.version` semver validation)

## [v0.15.2] - 2026-04-25

**Default-plugin slim-down + distribution sync.** Two of the v0.15 default plugins (`templates-remote`, `doctor`) were doing redundant work and are now back in core. `DEFAULT_PLUGINS` shrinks from 5 to 3. The Nix flake and Homebrew formula were six versions behind; both are now in sync and wired into the release flow so they stay that way.

### Added

- **`fledge templates search`** and **`fledge templates publish`** — re-absorbed from `fledge-plugin-templates-remote`. Same flags (`--author`, `--limit`, `--json` for search; `--org`, `--private`, `--description`, `--yes` for publish) and identical JSON shapes. The plugin was a shell wrapper around `src/search.rs`/`src/publish.rs` modules that fledge already used internally — re-exposing the templates flavor in core eliminates the duplicate implementation. (#260)
- **`fledge doctor` Toolchains section** — re-absorbed from `fledge-plugin-doctor`. Probes 16 toolchains across rust/node/python/go/ruby/swift/JVM. Marked **informational**: missing entries render dimmed (`· tool (not installed)`) and don't pollute the pass/fail totals (a Python project shouldn't fail because Swift is absent). (#260)
- **Post-release formula workflow** (`.github/workflows/post-release-formula.yml`) — runs after the `Release` workflow finishes uploading binaries + their `.sha256` sidecars. Fetches the real shas, rewrites `Formula/fledge.rb` with the new version + shas, and opens a PR. This is the only correct moment to bump the formula — at `fledge release` time the new version's binaries don't exist yet, so any pre-build sha would be a lie. (#261)

### Changed

- **`DEFAULT_PLUGINS` is now 3 entries**: `github`, `deps`, `metrics`. `fledge plugins install --defaults` no longer pulls `templates-remote` or `doctor` (their commands are in core). Existing users who previously installed the dropped plugins can remove them with `fledge plugins remove`. (#260)
- **`fledge-plugin-metrics` rewritten in Rust** — links `tokei` as a library (no separate `cargo install tokei`), uses the `ignore` crate for gitignore-aware walking, and emits stable plugin-owned JSON shapes instead of pass-through `tokei --output json`. Auto-detected build via `Cargo.toml`. (CorvidLabs/fledge-plugin-metrics#1)
- **`fledge.toml` gains `[release].files`** — `flake.nix` now bumps alongside `Cargo.toml` on every release. (`Formula/fledge.rb` is intentionally excluded; see the post-release workflow above.) (#259)
- **`Section.informational: bool`** added to the doctor report; informational sections appear in `--json` output but are excluded from the passed/failed counts. (#260)

### Fixed

- **Nix flake version**: 0.9.1 → 0.15.1 → 0.15.2. Cosmetic (the lockfile drives the actual build), but the label was misleading. (#259)
- **Homebrew formula**: 0.9.0 (with `sha256 "PLACEHOLDER"` ×3 — never actually installed since the formula was added) → 0.15.1 with real sha256 hashes pulled from the v0.15.1 release artifacts. The formula will move to v0.15.2 automatically once the post-release workflow runs against the v0.15.2 tag and opens a PR. (#259, #261)
- **`CLAUDE.md` src/ list** refreshed: dropped 6 files removed in the v0.15 tight-core refactor (`checks.rs`, `deps.rs`, `metrics.rs`, `issues.rs`, `prs.rs`, `update.rs`); added 9 real files that were missing (`ai.rs`, `introspect.rs`, `llm.rs`, `meta.rs`, `protocol.rs`, `spinner.rs`, `trust.rs`, `utils.rs`, `watch.rs`). (#259)

### Deprecated

- **`CorvidLabs/fledge-plugin-templates-remote`** and **`CorvidLabs/fledge-plugin-doctor`** are archived. Their READMEs now point at the corresponding core commands. The repos still exist (`fledge plugins install` still works against them) but are no longer maintained.

### Spec bumps

- `search` v3, `publish` v4, `doctor` v6 (full rewrite for the Toolchains section + the informational Section field), `plugin` v11 (DEFAULT_PLUGINS shrink + 2 export rows added to satisfy spec-sync `--strict`).

## [v0.15.1] - 2026-04-25

**Patch release — symmetric `update --defaults`.**

### Added

- **`fledge plugins update --defaults`** (#257) — symmetric with the v0.15 `install --defaults` flag. Updates only the installed plugins from the curated `DEFAULT_PLUGINS` set, leaving community plugins untouched. Source matching is tolerant of all three forms a plugin's stored `source` can take: `owner/repo` shorthand, normalized URL, and URL without `.git`. If none of the defaults are installed, the command suggests `fledge plugins install --defaults` and exits 0.

### Spec bumps

- `plugin` v9 → v10

## [v0.15.0] - 2026-04-24

**The tight-core release.** v0.15 keeps the load-bearing pillars — templates, lanes, plugins, spec-sync, AI (ask/review/multi-model), work, run, release — and removes everything else from the core binary. The signature is now: *one Rust binary, six pillars, spec-driven by default*. Anything ecosystem-specific or platform-specific (GitHub clients, language toolchain probes, lockfile parsers, vanity metrics) belongs in plugins where it can evolve independently and not bloat the binary for users who don't need it.

### Added

- **`fledge plugins install --defaults`** — one-command bulk install of fledge's curated plugin set. Pulls in the five plugins that took over commands removed from core: `fledge-plugin-github`, `fledge-plugin-deps`, `fledge-plugin-metrics`, `fledge-plugin-templates-remote`, `fledge-plugin-doctor`. Per-plugin failures don't abort the bulk install; the trailing summary lists each failure with its error. After a fresh `cargo install fledge`, this command gets you back to v0.14 feature parity in one line.
- **`fledge review --with-model <provider[:model]>`** — multi-model review panel (#255). Pass one or more `--with-model` refs (repeatable + comma-separated) and the review runs across all of them in parallel against the same diff and spec context, with cyan banner headers between slots and an `elapsed_seconds` per slot. Per-model errors are captured (not fatal). `--no-active` excludes the configured-default slot. JSON gains `reviews[]` array; legacy single-model `review`/`provider`/`model` fields preserved when panel size is 1. The first signature workflow that v0.14's `fledge ai` enables: switch providers in seconds, then turn around and use any of them — or all of them — to review your diff.

### Removed

The following commands and their modules left core. Some will return as plugins; some are gone for good. *No silent regressions* — running any removed command surfaces the standard `unrecognized subcommand` clap error.

- **`fledge update`** — bidirectional template re-application. Deleted entirely, no plugin successor planned. The complexity-trap argument: re-applying an evolved template onto an evolved scaffold creates merge conflicts that look automatic but require human judgment every time. Manage drift through git, where it belongs.
- **`fledge deps`** (~1411 LOC) — polyglot dependency health (outdated/audit/licenses across npm/cargo/pip/swift/kotlin/etc.). Lockfile parsers per ecosystem don't belong in the binary every fledge user installs. Future: `fledge-plugin-deps`.
- **`fledge checks`, `fledge issues`, `fledge prs`** — GitHub-specific CI/issues/PR browsing. Bakes "all dev happens on GitHub Actions" platform assumption. PR *creation* via `fledge work pr` stays in core (with extension hooks for non-GitHub platforms in a future release). Future: `fledge-plugin-github`.
- **`fledge metrics`** (~601 LOC) — LOC/churn/test-ratio scanner. Niche, overlapped by `tokei`/`scc`/GitHub Insights. Future: `fledge-plugin-metrics`.
- **`fledge templates search` / `fledge templates publish`** — GitHub-specific template registry browsing/publishing. Local `init`/`create`/`validate`/`list` stay in core (the actual templates pillar). Future: `fledge-plugin-templates-remote` (likely default-install).
- **`fledge templates update`** — same as the top-level `fledge update`, bidirectional re-apply.

### Changed

- **`fledge doctor`** stripped to a self-check: validates fledge config loads, checks git, probes the AI provider's reachability. Toolchain probes (rust/node/python/swift/kotlin/ruby/etc.) — previously ~250 LOC of `if-elif` over every ecosystem — are gone from core. The `Project Type` and `Toolchain`/`Dependencies` sections are no longer in the report. Future tool-specific probes will arrive as `fledge-plugin-doctor-<ecosystem>` plugins. Net: doctor went from 906 → ~480 LOC and stops noisy-checking ecosystems you don't use.
- **Core surface: 21 → 14 commands.** The signature reads cleaner: scaffold (`templates`), run (`run`/`lanes`/`watch`), spec (`spec`), AI (`ai`/`ask`/`review`), ship (`work`/`release`/`changelog`), extend (`plugins`/`config`/`introspect`/`completions`/`doctor`).
- **`.gitignore`** widened to drop the entire `.claude/` directory (was just `.claude/worktrees/`).
- **Release prep**: extracted `src/update.rs`'s template-meta utilities (`ProjectMeta`, `write_project_meta`, `compute_file_hash`) into a new `src/meta.rs` so `fledge templates init` keeps writing `.fledge/meta.toml`. The `update` command and its 700+ LOC of bidirectional-sync logic are gone; the meta-tracking surface (~150 LOC) survives as a library module.

### Migration notes

- Scripts that called `fledge deps`, `fledge metrics`, `fledge update`, `fledge checks`, `fledge issues`, `fledge prs`, `fledge templates search/update/publish` need to use the underlying tool directly until plugin replacements ship.
- `fledge doctor --json` no longer emits `project_type` at the top level. Use `sections[*].name` to identify which check section a result belongs to.
- The `spec_check` count drops from 35 to 29 — that's the deleted module specs.

### Spec bumps

- `review` v6 → v7 — multi-model panel
- `doctor` v3 → v4 — stub-and-extend (toolchain probes removed)
- New module spec: none (`meta` is library code, not a user-facing command)
- Removed specs: `update`, `deps`, `metrics`, `checks`, `issues`, `prs`

## [v0.14.0] - 2026-04-24

**The multi-model feedback release.** v0.13.0 made fledge speak any LLM. v0.14.0 makes that easy to *use*. `fledge ai status`/`models`/`use` gives you a one-line provider+model switcher with a live picker for Ollama (local or cloud), so trying the same `fledge review` or `fledge ask` across qwen3-coder, gpt-oss, deepseek, and kimi takes seconds, not env-var gymnastics. And `fledge work pr` turns that velocity into shipped work — auto-generates the body from your commits, shows you a preview with a yes/no confirm, and (with `--ai`) hands the diff to whichever model you just selected to draft a real `## Summary` + `## Test plan` for review.

### Added

- **`fledge ai`** subcommand tree — three actions for managing AI providers without editing config or typing long env exports (#251)
  - `fledge ai status [--json]` — show the active provider, model, and host with a `(from env / config / default)` source tag on each value, so you can see *why* a setting is active
  - `fledge ai models --provider {ollama,claude} [--search <q>] [--json]` — live list of available models. For Ollama, hits `<host>/api/tags` with a 5-second timeout (graceful "is the server running?" hint on failure); for Claude, returns a curated alias list with a "not authoritative" note
  - `fledge ai use [provider] [model]` — interactive picker (live model list for Ollama, with a `(custom…)` escape for not-yet-pulled models) or fully scriptable via positional args. Writes to `~/.config/fledge/config.toml`. Honors `--non-interactive` / non-TTY shells with a clear error
- **`fledge work pr` auto body + preview + confirmation** (#253)
  - When `--body` is omitted, generates `## Summary` + bullets from `git log base..branch`, stripping conventional-commit prefixes (`feat:`, `fix(scope):`) and sentence-casing each bullet
  - Styled preview block (title, `head → base`, draft tag, full body) before any push or `gh pr create` call
  - `Create this pull request? (Y/n)` confirmation prompt with default Yes; choosing "n" prints `✋ Aborted.` and exits 0 with no side effects
  - `--yes` / `-y` skips the prompt; `--json` skips it as well (agent-friendly). Non-interactive shells without `--yes`/`--json` bail with a clear message rather than hanging
- **`fledge work pr --ai`** for LLM-drafted PR bodies (#253)
  - Hands the full commit log + `git diff --stat` + a 600-line-truncated unified diff to the configured provider (`fledge ai use`-aware)
  - Generates a richer Markdown body with both `## Summary` and `## Test plan` sections — the model can reference specific files and functions because it sees the diff
  - Per-call overrides: `--provider {claude,ollama}`, `--model <name>`. `--body <text>` always wins over `--ai` (literal beats generated)
  - Spinner shows `Drafting PR body [provider (model)]:` during the call (suppressed in `--json`)
- **`ai.ollama.timeout_seconds` config key** (default `600`) plus **`FLEDGE_AI_TIMEOUT`** env var — control the per-request HTTP timeout for the Ollama provider. Useful for slow local models or long-context cloud calls (#251)

### Changed

- `fledge work pr` now always passes `--body` to `gh pr create` (even when generated heuristically), so the resulting PR is never empty (#253)

### Fixed

- `fledge doctor` now honors `FLEDGE_AI_MODEL` when displaying the active Ollama model — previously ignored the env override, mirroring the existing `OLLAMA_HOST` lookup (#251)

### Spec bumps

- `work` v6 → v8, `config` v7 → v8, `doctor` v3 → v4, `llm` v1 → v2
- New module spec: `ai` v1

## [v0.13.0] - 2026-04-23

**The agent-surface release.** fledge is now designed for humans and AI agents to drive the same CLI. Pick any LLM backend — Claude CLI or Ollama (local, cloud, or self-hosted) — and they all speak the same spec-aware `fledge ask` / `fledge review`. Set `FLEDGE_NON_INTERACTIVE=1` once, get JSON on every read command, and let the AI commands automatically include the right spec context from your repo's design docs. See the new [AGENTS.md](./AGENTS.md) for the one-page guide.

### Added

- **`AGENTS.md`** at the repo root plus `docs/src/agents.md` — canonical one-page guide for AI agents driving fledge, covering the machine-readable surface, non-interactive mode, provider selection, and typical workflows (#242)
- **LLM provider abstraction + Ollama support** — `fledge ask` and `fledge review` now route through a `LlmProvider` trait. Two implementations ship in core: Claude CLI (default, unchanged) and Ollama. The Ollama provider covers the local daemon, Ollama Cloud / Turbo (with `OLLAMA_API_KEY`), and any self-hosted Ollama-speaking endpoint in one impl. Select via `ai.provider` config, `FLEDGE_AI_PROVIDER` env, or `--provider {claude,ollama}` per invocation. (#250)
- **`fledge introspect [--json]`** — dumps the full clap command tree (every subcommand, every arg, every alias) as nested JSON or an indented listing. One call teaches an agent the entire CLI (#248)
- **`fledge spec list [--json]`** (alias `ls`) and **`fledge spec show <name> [--json]`** — enumerate and inspect specs programmatically (#242)
- **`fledge spec check --json`** — structured validation output with per-spec errors/warnings (#246)
- **`fledge ask`** is spec-aware by default: every invocation prepends a compact index of the project's specs. New `--with-specs <names>` loads full spec + companion bundles for named modules (`all` supported); `--no-spec-index` for off-topic questions. JSON output gains `provider` and `model` fields. (#244, #250)
- **`fledge review`** auto-detects relevant specs from the diff's changed-file list (matches each spec's `files:` frontmatter and the `<specs_dir>/<name>/` prefix, honoring custom `specs_dir`). New `--with-specs` to force-include; `--no-auto-specs` to disable. JSON output gains `spec_context`, `provider`, and `model` arrays. (#245, #250)
- **`fledge work start --json`**, **`fledge work pr --json`**, **`fledge work status --json`** — structured output for scripting branch and PR workflows. `status` distinguishes `behind: null` (base not fetched) from `behind: 0` (up to date) (#246)
- **Global `--non-interactive` flag** (alias `--ni`) and **`FLEDGE_NON_INTERACTIVE` env var** — one switch that treats every confirmation prompt as `--yes`/`--force` and bails with an actionable error on prompts that have no default (#247)
- **`fledge doctor` dual-provider AI section** — detects both `claude` and `ollama` binaries, reports the active provider, and probes the Ollama host's `/api/tags` with a 3-second timeout. Distinguishes "daemon down" from "not installed" from "typo in `ai.provider`" (#250)
- **New `[ai]` config section** — `ai.provider`, `ai.claude.model`, `ai.ollama.{host,api_key,model}`. Env var overrides: `FLEDGE_AI_PROVIDER`, `FLEDGE_AI_MODEL`, `OLLAMA_HOST`, `OLLAMA_API_KEY`, `FLEDGE_AI_TIMEOUT`. All follow the CLI > env > config > default precedence. (#250)
- Completed companion-file set for the `trust` spec module (#241)

### Changed

- README gains a short "Working with AI agents?" callout near the top pointing to `AGENTS.md` — mentions both Claude CLI and Ollama paths (#248, #250)
- Prompt constraints on `fledge review` explicitly tell the active provider to treat specs as context-only and review only the diff itself — no suggestions on unchanged code, no critique of the specs (#245)
- `fledge work pr` URL parsing is now robust to trailing slashes, query strings, and subpaths (`/pull/42/files`, `/pull/42?x=1`, etc.) (#246)

### Fixed

- `fledge work status --json`'s `behind` field no longer silently reports `0` when `git rev-list` can't compute it (base branch not fetched) — emits `null` instead so agents can tell "needs fetch" from "up to date" (#246)
- `fledge doctor` no longer silently falls back to Claude when `ai.provider` is set to an invalid value; it now surfaces the parse error as an Error-level check (#250)
- `OllamaProvider` distinguishes HTTP status errors (401, 404, 500) from connection failures, so users get a clean "endpoint returned HTTP 500" message instead of "decoding response" (#250)

### Spec bumps

- `ask` v2 → v4, `review` v4 → v6, `spec` v2 → v5, `work` v5 → v6, `main` v2 → v5, `config` v6 → v7, `doctor` v2 → v3
- New module specs: `introspect` v1, `llm` v1, `trust` v1 companion files

## [v0.12.1] - 2026-04-23

### Added

- Swift (Package.swift) and Kotlin (Gradle/Maven) dependency support in `fledge deps` (#239)

## [v0.12.0] - 2026-04-23

### Added

- `fledge watch` command — file-watching with automatic task/lane re-runs (#230, #231)
- `--model`, `--prompt`, and `--format` flags for `fledge review` (#232)
- Kotlin KMP and Kotlin Ktor API templates (#234)
- `--json` flag for `fledge lanes run` (#237)

### Fixed

- Watch debounce behavior — reduced duplicate re-runs on rapid saves (#236)
- `fledge doctor` now correctly detects bun, pnpm, and yarn toolchains (#235)
- Improved 404 error messages for GitHub API calls (#233)

### Changed

- CLI commands reordered alphabetically for consistency (#237)

## [v0.11.1] - 2026-04-23

### Added

- `fledge run --json` flag for structured JSON output — improves AI agent usability (#228)

## [v0.11.0] - 2026-04-23

### Added

- Plugin trust tiers and `fledge plugin audit` command — verify plugin provenance (#220)
- Trust tier badges for templates and lanes — warnings for unverified sources (#221)
- Non-TTY support for AI agents and CI environments — all interactive prompts gracefully degrade (#222)
- `uv.lock` support in `fledge deps` for Python projects (#223)
- Use cases page and enhanced review documentation (#219)

### Fixed

- Proper TOML parsing for `uv.lock` instead of fragile line-based parsing (#224)
- Plural command names (`lanes`, `plugins`) used consistently across all docs and specs (#216, #217, #218)

### Changed

- CONTRIBUTING.md fully dogfoods fledge — uses `fledge run`, `fledge lanes`, and `fledge work` instead of raw cargo commands (#223, #225, #226)

## [v0.10.0] - 2026-04-22

### Added

- `fledge lanes create` / `fledge lanes validate` — scaffold and validate lane definitions (#203)
- `fledge plugin create` / `fledge plugin validate` — scaffold and validate plugin manifests (#203)
- Plugin protocol v1 — full JSON-lines IPC with capability manifest, structured logging, and lifecycle events (#178, #179, #196, #197)
- Plugin and lane publishing — `fledge plugin publish` and `fledge lanes publish` (#176, #177)
- GitHub CLI (`gh`) token fallback — fledge uses `gh auth token` when no `GITHUB_TOKEN` is set (#201)
- Release workflow hardening — duplicate tag pre-check prevents overwriting existing releases (#214)
- 10 new release tests covering gemspec, setup.cfg, pom.xml bumping, `--no-tag` flag, and edge cases (#214)
- Cross-platform plugin protocol tests (#180)

### Fixed

- Plugin state.json locking, env filtering, key validation, and exec timeout cap (#188)
- Plugin protocol security hardening — input validation, output size limits (#187)
- Plugin audit findings from security review (#195)
- TOML serialization crashes, UTF-8 truncation panics, and remote ref parsing failures (#200)
- 6 crash and security findings from codebase audit (#199)
- Security review findings — input sanitization and error handling (#198)
- Error message config keys now reference correct `fledge config` commands (#213)
- Spec frontmatter documented as YAML (not TOML) with correct field types (#212)
- Audit round 2 — doc/spec inaccuracies, missing CLI flags, wrong command names (#211)
- Documentation and spec gaps filled (#210)
- Infra and publishing audit bug fixes (#209)
- Dev loop audit bug fixes (#208)
- Templates audit bug fixes (#206)
- Doc inaccuracies in language defaults, lane docs, and CLI reference (#214)

### Changed

- Removed TUI module — will be reimplemented as a plugin (#204)
- CLI documentation updated to match current subcommand structure (#202)

## [v0.9.1] - 2026-04-21

### Fixed

- Release workflow: use `cp` instead of `mv` in checksum step to fix artifact packaging with `download-artifact@v4` (#173)

## [v0.9.0] - 2026-04-21

### Added

- `fledge lane` — composable workflow pipelines with sequential, parallel, and inline steps
- `fledge lane --init` — auto-generate lanes for your project type
- `fledge plugin` — plugin architecture (install, remove, list, search, run) via GitHub repos
- `fledge validate-template` — validate templates for correctness with `--strict` and `--json` output
- `fledge run` zero-config mode — auto-detects project type and runs tasks without `fledge.toml`
- Community lane registry — search and import lanes from GitHub
- `fledge.toml` in the repo root — fledge now dogfoods its own CLI for development workflows
- "Using Fledge with Existing Projects" documentation guide
- Step timing for lanes — each step shows elapsed time, lane summary shows total time
- Plugin lifecycle hooks — `pre_init`, `post_work_start`, `pre_push` fire at fledge lifecycle events
- Parallel lane steps accept inline commands alongside task references
- SECURITY.md — vulnerability reporting policy and security model documentation
- CONTRIBUTING.md — development setup, workflow, code guidelines, and contribution process
- Doctor guide page in documentation (`docs/src/doctor.md`)
- Troubleshooting page in documentation (`docs/src/troubleshooting.md`)

### Fixed

- **Security**: path traversal in template rendering — malicious templates can no longer write outside the project directory
- **Security**: GitHub token no longer leaked via process table — auth passed via environment variables instead of CLI args
- **Security**: config files now enforce 0600 permissions on both new and pre-existing files
- **Security**: plugin binary path traversal hardened — both plugin dir and binary path are canonicalized before comparison
- **Security**: plugin command names validated to prevent symlink injection (rejects `/`, `\`, `.`, `-` prefix)
- **Security**: plugin install now shows security warning and requires confirmation (use `--force` to skip in CI)
- **Security**: post-create hooks always require confirmation regardless of template source (use `--yes` to skip in CI)
- **Security**: template requirement checker rejects tool names starting with `-` to prevent `which` false positives
- **Security**: replaced hand-rolled base64 with audited `base64` crate
- CLI reference examples now use correct built-in template names
- CLI Reference: added missing `--author` and `--org` flags for `fledge init`
- CLI Reference: added missing `--description`, `--render-patterns`, `--hooks`, `--prompts`, `--yes` flags for `fledge create-template`
- CLI Reference: corrected `--type` to `--branch-type` for `fledge work start` (matching actual flag name)
- CLI Reference: removed non-existent `-y, --yes` flag from `fledge update`
- CLI Reference: updated `fledge lane` to document subcommand structure (`run`, `list`, `init`, `search`, `import`)
- CLI Reference: added short flags (`-t`, `-b`) for `fledge work pr`
- Removed misplaced TUI section from plugins documentation page
- Fixed `--type` → `--branch-type` in develop guide, GitHub integration guide, and quick start
- Updated SUMMARY.md with new documentation pages

### Changed

- **Breaking**: `fledge lane` now uses subcommands — `fledge lane run <name>` replaces `fledge lane <name>`, `fledge lane list` replaces `fledge lane --list`, etc.
- **Breaking**: post-create hooks now always prompt for confirmation (pass `--yes` to auto-approve for CI/scripts)
- **Breaking**: `fledge plugin install` now requires confirmation before cloning (pass `--force` to skip for CI/scripts)
- **Breaking**: hook execution uses direct process invocation instead of shell — pipes, redirects, and shell expansions in hook commands are no longer supported; use a wrapper script instead
- Full end-to-end dev lifecycle coverage from scaffold to ship
- Homebrew formula updated to 0.9.0
- CLI commands reorganized: `fledge templates`, `fledge lanes`, `fledge plugins` with subcommands

## [v0.8.0] - 2026-04-19

### Added

- `fledge deps` - dependency health check (outdated packages, audit, license scan) for Rust, Node, Python, Go, Ruby
- `fledge metrics` - project stats (lines of code by language, test file ratio, churn analysis)
- `fledge doctor` - environment diagnostics (toolchain versions, missing dependencies, config validation)
- JSON output for all three commands (`--json`)

## [v0.7.0] - 2026-04-19

### Added

- `fledge run` — task runner with `fledge.toml` support, `--init` scaffolding, language-aware defaults (Rust, Node, Go, Python, Ruby, Java/Gradle/Maven)
- `fledge checks` — view CI/CD check status for any branch with `--json` output
- `fledge changelog` — generate changelogs from git tags and conventional commits with `--limit`, `--tag`, `--unreleased`, `--json` flags

### Fixed

- Made fledge fully language-agnostic — `.gitignore` template covers all ecosystems, upgrade message links to install docs instead of assuming `cargo install`
- Split Java detection into Gradle/Maven, reinstated `/target/` in `.gitignore`
- Removed invalid `--prompt` flag from Claude CLI calls in `fledge ask`/`fledge review`

## [v0.6.1] - 2026-04-19

### Fixed

- Add missing `version` field in `TemplateInfo` constructor — the v0.6.0 crates.io publish was built before this fix landed and didn't compile, so v0.6.1 is a republish of the same feature set with the build error resolved.

## [v0.6.0] - 2026-04-19

### Added

- Install script (`curl -fsSL .../install.sh | sh`) — detects OS/arch, downloads the right binary
- Homebrew formula (`brew install CorvidLabs/tap/fledge`)
- Nix flake (`nix run github:CorvidLabs/fledge`)
- `fledge completions --install` — auto-installs shell completions for bash, zsh, or fish
- SHA256 checksums in GitHub releases

## [v0.5.0] - 2026-04-19

### Added

- `fledge issues` — list and view GitHub issues with `--state`, `--label`, `--json` filters
- `fledge prs` — list and view GitHub pull requests with `--state`, `--json` filters
- `fledge review` — AI-powered code review of current changes via Claude CLI
- `fledge ask` — ask questions about your codebase via Claude CLI

## [v0.4.0] - 2026-04-19

### Added

- `fledge update` — re-apply source template to existing projects with `--dry-run` and `--refresh`
- `fledge spec check` — validate spec-sync specifications against source code
- `fledge spec init` — initialize spec-sync configuration
- `fledge spec new` — scaffold a new spec module
- `fledge work start` — begin a feature branch with naming conventions
- `fledge work pr` — create a PR from the current branch
- `fledge work status` — show current branch and PR status

## [v0.3.0] - 2026-04-19

### Added

- `fledge search` — template discovery via GitHub topics
- `fledge publish` — publish templates to GitHub with `fledge-template` topic
- `fledge create-template` — scaffold a new fledge template
- Template versioning and compatibility checks (`min_fledge_version`)
- Version pinning for remote templates with `@ref` syntax
- Additional built-in templates: `python-cli`, `go-cli`, `ts-node`, `static-site`

### Changed

- `fledge config` — full subcommand interface (get/set/unset/add/remove/list/path)
- mdBook documentation site on GitHub Pages

## [v0.2.1] - 2026-04-18

### Fixed

- Templates were only discoverable via filesystem paths, so `cargo install` users got "No templates found" errors. Templates are now compiled into the binary with `include_dir` and extracted to a versioned cache directory on first use. (#25)

## [v0.2.0] - 2026-04-18

### Added

- `cargo publish` step in the release workflow — tagging a release auto-publishes to crates.io. Includes a `cargo publish --dry-run` preflight to catch packaging errors before burning a version slot, and a `startsWith(github.ref, 'refs/tags/v')` guard for defense-in-depth. (#22)
- Initial `CHANGELOG.md` (this file).

## [v0.1.0] - 2026-04-18

### Added

- Core scaffolding engine with Tera template rendering
- 6 built-in templates: `rust-cli`, `ts-bun`, `python-cli`, `go-cli`, `ts-node`, `static-site`
- Remote template support via `owner/repo` GitHub syntax
- Interactive prompts with dialoguer for project configuration
- Hook system with `pre_create` and `post_create` lifecycle hooks
- Hook security: confirmation prompts, `--dry-run`, and `--yes` flags
- Shell completions for bash, zsh, fish, elvish, and PowerShell (`fledge completions`)
- Colored error output with contextual help messages
- Global configuration via `~/.config/fledge/config.toml`
- Optional TUI mode (`--features tui`)
- CI pipeline: tests (3 OS), clippy, fmt, dependency audit, spec-sync validation
- Cross-platform release builds (Linux, macOS x86/ARM, Windows)
