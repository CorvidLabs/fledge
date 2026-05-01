# AGENTS.md

This page is for AI agents (Claude Code, GPT-based coding agents, OpenHands, etc.) using `fledge` alongside a human. Humans get `README.md` and `docs/`. Agents get this one page.

## What fledge is, in one paragraph

`fledge` is a dev-lifecycle CLI. One tool for the dev loop, any language. Scaffold (`templates`), run (`run`/`lanes`/`watch`), spec (`spec`), AI (`ai`/`ask`/`review`), ship (`work`/`release`/`changelog`), extend (`plugins`/`config`/`introspect`/`completions`/`doctor`). Anything ecosystem-specific is a plugin. Run `fledge plugins install --defaults` once for the curated set.

If you're about to run `npm`, `cargo`, `make`, or `git checkout -b`, check first whether fledge has a wrapper. It usually does, and the wrapper has `--json` and guardrails. For PRs, use `fledge github prs create` from `fledge-plugin-github`.

## First-time setup

```bash
export FLEDGE_NON_INTERACTIVE=1               # silence every prompt
fledge plugins install --defaults             # curated plugin set: github, deps, metrics
fledge introspect --json                      # full command tree (incl. plugin commands)
fledge spec list --json                       # orient to the codebase via specs
```

After those four lines every command and every flag is discoverable as data.

## Golden rules

1. **Prefer fledge subcommands over raw tools** when wrapped. Use `fledge work start` instead of `git checkout -b`, `fledge run test` instead of guessing `cargo test` vs `npm test`.
2. **Always add `--json` when a command supports it** (see list below). Parse the JSON. Do not screen-scrape pretty output.
3. **Set `FLEDGE_NON_INTERACTIVE=1` once** in your shell, or pass `--non-interactive` (alias `--ni`) per invocation. Confirmation prompts then behave as `--yes`. Prompts with no default bail with a clear error instead of blocking forever.
4. **Check exit codes.** Non-zero means failed, even if stdout looks fine.
5. **Run `fledge lanes run pre-commit` before mutating shared state.** It's the project-defined quality gate.

## Discover what fledge can do

```bash
fledge introspect --json             # full command tree, every subcommand and flag
fledge introspect                    # same, human-readable indented listing
fledge --help                        # top-level command list (human text)
fledge <cmd> --help                  # per-command flags
fledge spec list --json              # all specs as JSON
fledge spec show <name> --json       # one spec's structure as JSON
fledge plugins list --json           # what extensions are active
```

`fledge introspect --json` is the right starting point for an agent that has never seen fledge. It reveals the entire CLI surface (including plugin-installed commands) in one call.

Specs (`specs/<name>/*.spec.md` and companion files) are the source of truth for *why* a module exists. When you need context beyond what the code shows, read the spec. Particularly `specs/<name>/context.md` (design decisions) and `specs/<name>/requirements.md` (user stories).

## Machine-readable surface (`--json`)

### Core commands

| Command | What you get | Use when |
|---------|-------------|----------|
| `fledge introspect --json` | `{schema_version: 1, name, about, aliases, args: [...], subcommands: [...]}`. Each subcommand recursively has the same shape; each arg has `name, long?, short?, aliases, help, required, takes_value, value_name, global?` | First contact with fledge |
| `fledge spec list --json` | `{schema_version: 1, action: "spec_list", specs: [{name, version, status, sections, companions, ...}]}` | Orienting to a new codebase |
| `fledge spec show <name> --json` | `{schema_version: 1, action: "spec_show", spec: {name, version, status, sections, companions, ...}}` | Need structured view of one module |
| `fledge spec check --json` | `{schema_version: 1, action: "spec_check", specs: [...], totals, strict}` | Spec-sync validation as data |
| `fledge ai status --json` | `{schema_version: 1, action: "ai_status", provider, provider_source, model, model_source, host, host_source}`. All six keys always present. `provider_source` is `"env" \| "config" \| "default"`. `model`/`host` and their `*_source` may be `null` (e.g. Claude has no host) | Verifying provider config before invoking the LLM |
| `fledge ai models --provider {claude,ollama} --json` | `{schema_version: 1, action: "ai_models", provider, models: [...]}`. Ollama hits `/api/tags`, Claude returns curated aliases | Picking a specific model |
| `fledge ask "..." --json` | `{schema_version: 1, action: "ask", question, answer, provider, model}` from the active LLM | Answering a question about the code |
| `fledge review --json` | Single-model: `{schema_version: 1, action: "review", base, file, diff_stats, spec_context, reviews: [...], review \| error, provider, model}`. Top-level `provider`/`model` always present when panel size is 1; `review` (success) or `error` (slot failed) is the discriminator. Per-slot `reviews[]` items follow the same `review \| error` pattern | Before opening a PR |
| `fledge review --with-model <ref> --json` | Multi-model panel: `{schema_version: 1, action: "review", base, ..., reviews: [{provider, model, elapsed_seconds, review|error}, ...]}` | Comparing models on the same diff |
| `fledge doctor --json` | `{schema_version: 1, action: "doctor", sections: [{name, checks: [...], informational}], passed, failed}`. Four sections (`fledge`, `Git`, `AI`, `Toolchains`). `Toolchains` is informational, missing tools don't count toward `failed` | Debugging a broken setup |
| `fledge changelog --json` | `{schema_version: 1, action: "changelog", releases: [{tag, date, sections}]}` | Generating release notes |
| `fledge run --list --json` | `{schema_version: 1, action: "run_list", auto_detected, tasks: [...]}` | Discovering tasks defined in `fledge.toml` (or auto-detected) |
| `fledge run <task> --json` | `{schema_version: 1, action: "run_task", task, command, exit_code, success, stdout, stderr}` | Running a task and capturing its output |
| `fledge run --init --json` | `{schema_version: 1, action: "run_init", file, project_type, files_created}` | Scaffolding a `fledge.toml` |
| `fledge release --dry-run --json` | `{schema_version: 1, action: "release", dry_run: true, version, no_bump, files_to_bump, will_changelog, will_tag, will_push, tag}` | Preview what a release would do |
| `fledge release --json` | `{schema_version: 1, action: "release", dry_run: false, version, old_version, files_bumped, changelog_updated, commit_created, tag_created, tag, pushed}` | After a real release completes |
| `fledge lanes run <name> --dry-run --json` | `{schema_version: 1, lane, description, total_steps, fail_fast, dry_run: true, steps: [{step, kind, name}]}` | Preview lane steps without executing |
| `fledge plugins list --json` | `{schema_version: 1, plugins: [{name, version, source, installed, commands, pinned_ref, trust_tier}]}` | Auditing plugin state |
| `fledge plugins audit --json` | `{schema_version: 1, audit: [{name, version, source, trust_tier, capabilities: {exec, store, metadata}, commands, has_lifecycle_hooks}]}` | Capability/hook audit |
| `fledge plugins search --json` | `{schema_version: 1, results: [{name, full_name, description, stars, url, trust_tier}]}` | GitHub search for `fledge-plugin`-tagged repos |
| `fledge plugins validate --json` | `{schema_version: 1, path, plugin_name, errors, warnings}` | CI gate before publish |
| `fledge plugins install <src> --json` | `{schema_version: 1, action: "install", scope: "single" \| "defaults", installed: [{name, source, version, trust_tier, commands, pinned_ref, capabilities}], failed: [{source, error}], summary: {total, installed, failed}}` | Programmatic install / bulk default install |
| `fledge plugins update [name] --json` | `{schema_version: 1, action: "update", scope: "single" \| "all" \| "defaults", results: [{name, status: "updated" \| "skipped" \| "failed", version?, commands?, pinned_ref?, latest_tag?, detail?}], summary: {total, updated, skipped, failed}}`. Conditional fields depend on `status` | Bulk update |
| `fledge plugins remove <name> --json` | `{schema_version: 1, action: "remove", removed: {name, source, version, commands}}` | Programmatic uninstall |
| `fledge plugins publish --json` | `{schema_version: 1, action: "publish", cancelled, repo: {owner, name, url, created, private}, plugin: {name, version, description}, topic, install_hint}`. Same key set on success and cancelled paths; `cancelled: true` when the user declines | Publishing a plugin repo |
| `fledge plugins create --json` | `{schema_version: 1, action: "create", path, name, description, files_created}` | Scaffolding a new plugin |
| `fledge lanes list --json` | `{schema_version: 1, lanes: [{name, description, step_count, fail_fast, source?, trust_tier}]}`. `step_count` is an integer; full step detail lives in `lanes run --dry-run --json` | Discovering lanes available to run |
| `fledge lanes search --json` | `{schema_version: 1, results: [{owner, name, description, stars, url, topics, trust_tier}]}` | GitHub search for `fledge-lane`-tagged repos |
| `fledge lanes run <name> --json` | `{schema_version: 1, lane, description, total_steps, success, duration_ms, fail_fast, steps: [{step, name, success, duration_ms, error}], failures: [...]}` | Running the project's own CI pipeline |
| `fledge lanes validate --json` | `{schema_version: 1, path, lane_count, errors, warnings}` | CI gate before publish |
| `fledge lanes init --json` | `{schema_version: 1, action: "init", file, project_type, lanes_added}` | Adding default lanes to fledge.toml |
| `fledge lanes import <src> --json` | `{schema_version: 1, action: "import", source, trust_tier, imported, skipped, file, written}`. `file` is always the computed `.fledge/lanes/<safe>.toml` path; `written: false` when every lane was skipped | Importing community lanes |
| `fledge lanes publish --json` | `{schema_version: 1, action: "publish", cancelled, repo: {owner, name, url, created, private}, lanes_published, topic, import_hint}`. Same key set on success and cancelled paths; `cancelled: true` when the user declines | Publishing a lane repo |
| `fledge lanes create --json` | `{schema_version: 1, action: "create", path, name, description, files_created}` | Scaffolding a new lane repo |
| `fledge templates list --json` | `{schema_version: 1, templates: [{name, description, source, source_ref, path}]}` | Listing available templates |
| `fledge templates search --json` | `{schema_version: 1, results: [{owner, name, description, stars, url, topics, trust_tier}]}` | GitHub search for `fledge-template`-tagged repos |
| `fledge templates validate --json` | `{schema_version: 1, reports: [{path, template, errors, warnings}]}` | CI gate before publish |
| `fledge templates init <template> --json` | `{schema_version: 1, action: "init", project: {name, path}, template: {name, source, version}, variables_used, files_created, git_initialized, hooks_run}` | Scaffolding a new project |
| `fledge templates create --json` | `{schema_version: 1, action: "create", path, name, description, render_patterns, include_hooks, include_prompts, files_created}` | Creating a new template skeleton |
| `fledge templates publish --json` | `{schema_version: 1, action: "publish", cancelled, repo: {owner, name, url, created, private}, template: {description}, topic, use_hint}`. Same key set on success and cancelled paths; `cancelled: true` when the user declines a confirmation | Publishing a template repo |
| `fledge work start <name> --json` | `{schema_version: 1, action: "work_start", branch, base, type, prefix, issue}`. `issue` is `null` when no `--issue` flag was passed; all other fields always present | Branch scripting |
| `fledge work commit --json` | `{schema_version: 1, action: "work_commit", hash, message, branch}`. Commit hash to report back | After writing code |
| `fledge work push --json` | `{schema_version: 1, action: "work_push", branch, remote, force}`. Confirms the push | After committing |
| `fledge work status --json` | `{schema_version: 2, action: "work_status", branch, default, ahead, behind, dirty}`. `dirty` is uncommitted file count; no PR field | Pre-action sanity check |

### Plugin commands (after `plugins install --defaults`)

| Command | Plugin | What you get |
|---------|--------|-------------|
| `fledge github checks --json` | `fledge-plugin-github` | Raw GitHub API response of CI check-runs for a branch |
| `fledge github issues --json` / `fledge github issues view <n> --json` | `fledge-plugin-github` | GitHub issues, list or one |
| `fledge github issues create --title "..." --json` | `fledge-plugin-github` | Create an issue; returns `{number, url, title}` |
| `fledge github prs --json` / `fledge github prs view <n> --json` | `fledge-plugin-github` | GitHub PRs, list or one |
| `fledge github prs create --fill --json` | `fledge-plugin-github` | Create PR (infer from commits); returns `{number, url, title}` |
| `fledge deps --json` | `fledge-plugin-deps` | Dependency report from the ecosystem tool (`cargo outdated`, `npm audit`, ...) |
| `fledge metrics --json` / `--churn --json` / `--tests --json` | `fledge-plugin-metrics` | LOC summary (tokei), per-file churn, test/source ratio |

Commands **without** `--json` (pretty output only): `spec init`, `spec new`, `watch`, `ai use`, `config *` (including `config edit`, an interactive editor), `completions`. If you need structured output from one of these, add it via a spec + PR. It's an accepted pattern.

**Envelope contract.** Every `--json` output is `{schema_version: 1, ...}`. Two patterns coexist:

- Pillar list/query commands (`plugins list`, `lanes list/run/search`, `templates list/search`) use `{schema_version: 1, <resource>: [...]}`. The resource key (`plugins`, `lanes`, `results`, `templates`) acts as the discriminator.
- Cross-cutting commands (`doctor`, `run`, `ai`, `ask`, `changelog`, `work`, `spec`, `review`) use `{schema_version: 1, action: "<verb>", ...}`. The `action` string discriminates between commands sharing similar shapes.

Top-level `schema_version` is the version contract, **scoped per command**. Each `--json`-emitting command has its own version that bumps only when *that command's* shape changes incompatibly. Two commands both emitting `schema_version: 1` does not mean their shapes are linked — they're tracked independently. New fields are additive within a given command's version; field removal/retyping bumps that command's version. **Always read `<resource>` (or `action` + named keys). Never assume the top level is an array.** Pre-1.0 outputs that returned bare arrays were wrapped in tier C/D of the 1.0 readiness work. Pinning to fledge ≥ 1.0 means you can rely on the envelope.

**Error output.** Errors always go to stderr as plain text, even when `--json` is active. Check the exit code first — non-zero means failure and stderr carries the human-readable error. Do not parse stderr as JSON. Stdout may still contain a partial or final envelope before some failures (e.g. `lanes run --json` emits the lane envelope with `success: false` then bails), so don't treat "stdout has JSON" as a success signal — the exit code is the contract.

## Non-interactive mode

Set this once at the top of your shell session and forget about it:

```bash
export FLEDGE_NON_INTERACTIVE=1
```

Or pass the flag per invocation: `fledge --non-interactive <cmd>` (alias `--ni`). Both are equivalent.

When non-interactive mode is active, every command that would otherwise prompt behaves **as if `--yes` / `--force` were passed**:

| Command | Effect |
|---------|--------|
| `fledge templates init` | Skip template-variable prompts (uses detected defaults) |
| `fledge templates create` | Skip name/description/type prompts |
| `fledge ai use` | Errors with a clear "pass provider+model" message. No hang |
| `fledge work commit` | Skip the interactive message prompt (requires `-m` or `--ai`) |
| `fledge plugins install` | Skip trust-tier and capability-grant prompts |
| `fledge plugins publish` | Skip confirmations |
| `fledge plugins create` | Skip scaffolding prompts |
| `fledge lanes publish` | Skip description prompt |
| `fledge templates publish` | Skip the confirmation prompt |

Prompts with no sensible default (`fledge ai use` being asked to pick a provider when none was specified) fail fast with a clear error naming the flag to pass instead. No silent hangs.

You can still pass `--yes`/`--force` per command if you prefer. They and `FLEDGE_NON_INTERACTIVE` compose.

## AI commands

`fledge ai`, `fledge ask`, and `fledge review` go through a provider abstraction. Two providers ship in core:

- **Claude** (default). Shells out to the `claude` CLI, which must be installed and authenticated on the host.
- **Ollama**. HTTP to any Ollama-speaking endpoint: local daemon (`http://localhost:11434`), Ollama Cloud / Turbo, or self-hosted. Supports a Bearer API key.

### Picking a provider, three ways

```bash
# 1. fledge ai use (writes to ~/.config/fledge/config.toml, persists)
fledge ai use ollama qwen3-coder:480b-cloud
fledge ai use claude opus-4.7
fledge ai status                           # show the active triplet + source of each value

# 2. Env vars (per-shell-session)
export FLEDGE_AI_PROVIDER=ollama
export OLLAMA_HOST=https://ollama.com
export OLLAMA_API_KEY=sk-...
export FLEDGE_AI_MODEL=gpt-oss:120b-cloud
export FLEDGE_AI_TIMEOUT=600                # seconds, Ollama only

# 3. Per-invocation (highest precedence)
fledge ask --provider ollama --model qwen3-coder:480b-cloud "..."
fledge review --provider claude --model opus-4.7
```

Precedence: CLI flag > env var > config > default (`claude`).

### `fledge ask` is spec-aware by default

Every `fledge ask` invocation automatically prepends a compact index of all specs (one line per module: name, version, status, files, first-paragraph purpose). The model can then cite specific specs in its answer even when the user didn't mention them.

```bash
fledge ask "how does work build branch names?" --json
fledge ask --with-specs work,trust "how do these modules interact?"
fledge ask --with-specs all "which modules touch GitHub?"
fledge ask --no-spec-index "quick Rust syntax question"
```

### `fledge review`, single or multi-model

`fledge review` auto-detects which modules a diff touches (via each spec's frontmatter `files:` and any edits under `specs/<name>/`) and includes their full spec + companion files as context.

```bash
# Single model (active config)
fledge review --json
fledge review --base HEAD~3 --json
fledge review --with-specs plugin --json
fledge review --no-auto-specs --json
fledge review --model opus --format checklist --json

# Multi-model panel: same diff, parallel critiques
fledge review --with-model ollama:gpt-oss:120b-cloud --with-model ollama:qwen3-coder:480b-cloud --json
fledge review --no-active --with-model claude:opus-4.7,ollama:gpt-oss:120b-cloud --json
```

The JSON output's `reviews[]` array contains one entry per slot with `provider`, `model`, `elapsed_seconds`, and either `review` or `error`. Per-slot failures don't abort the panel.

The prompt is explicitly constrained: the model reviews *only the diff*, treats the specs as context-only, and must not suggest changes to code outside the diff or critique the specs themselves.

## Typical agent workflows

### Start a task
```bash
fledge spec list --json                                    # orient
fledge spec show <module> --json                           # dig into one area
fledge work start my-change -t feat                        # branch
```

### Commit and push changes
```bash
fledge work commit --ai --all --json                       # AI-drafted commit, stage everything
fledge work commit -m "feat: add search index" --json      # explicit message
fledge work push --json                                    # push to origin
```

### Open a PR
```bash
# Via fledge-plugin-github (structured output):
fledge github prs create --title "feat: add search index" --draft --json
# Or infer title/body from commits:
fledge github prs create --fill --json
```

### Before reporting a task done
```bash
fledge lanes run pre-commit                                 # fmt + lint + test + spec-check
# or the project-specific full lane:
fledge lanes run ci
```

### Verify CI is green (requires `fledge-plugin-github`)
```bash
fledge github checks --json | jq '.check_runs[] | {name, conclusion}'
```

### Inspect a spec's deeper context as part of planning
```bash
cat specs/<name>/context.md      # design decisions
cat specs/<name>/requirements.md # user stories / acceptance
cat specs/<name>/tasks.md        # what's done, what's gapped
cat specs/<name>/testing.md      # test plan
```

## Exit codes

- `0` success
- `1` user-facing error (bad input, missing file, validation failure, prompt required in non-TTY)
- Non-zero exit also fires on `fledge spec check` errors, `fledge lanes run` failures, and `fledge review` errors

## Project-specific quality gate

This repo defines its own lanes in `fledge.toml`. The key ones for agents:

- `fledge lanes run pre-commit`. fmt + lint + test + spec-check (required before opening a PR)
- `fledge lanes run ci`. Full CI pipeline locally
- `fledge lanes run check`. Quick parallel fmt+lint then test
- `fledge spec check`. Always run if you touched `src/` or `specs/`

## When things go wrong

- **A command hung**: you probably skipped `--yes` or `--force`. Cancel, re-run with the bypass flag (or set `FLEDGE_NON_INTERACTIVE=1` once).
- **`fledge ask` / `review` errored with auth**: the host's `claude` CLI isn't set up, or your Ollama config is wrong. Run `fledge ai status` to see what fledge thinks is active, then `fledge doctor` to verify the provider is reachable.
- **`fledge spec check` fails**: read the error. Almost always a missing section, missing source file, or unknown status. Don't "fix" it by editing the validator.
- **`fledge work push` fails**: check `fledge work status --json` for ahead/dirty counts. If there's nothing to push (ahead=0), commit first. If you're not on a tracking branch, `fledge work push` sets `-u origin` automatically.
- **`fledge checks` (or any command) says "unrecognized subcommand"**: the corresponding plugin isn't installed. Run `fledge plugins install --defaults` for the curated set.
- **A multi-model `fledge review` panel had one slot fail**: that slot's `error` field has the cause, the other slots' reviews are still valid. `--with-model` is fault-tolerant by design.

## Extending fledge for better agent support

If a command you want doesn't expose `--json`, or a workflow isn't automatable, the right fix is:
1. Open an issue tagged `agent-surface`
2. Update the corresponding spec (`specs/<module>/<module>.spec.md`). Bump the version, add the new flag to Public API + Behavioral Examples
3. Implement, run `fledge lanes run pre-commit`, open the PR

The project explicitly welcomes agent-surface improvements.
