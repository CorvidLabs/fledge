# AGENTS.md — fledge for AI agents

This doc is for AI agents (Claude Code, GPT-based coding agents, OpenHands, etc.) using the `fledge` CLI alongside a human. Humans get `README.md` and `docs/`; agents get this one page.

## What fledge is, in one paragraph

`fledge` is a single-binary dev-lifecycle CLI (Rust). **Six pillars: scaffold (`templates`), run (`run`/`lanes`/`watch`), spec (`spec`), AI (`ai`/`ask`/`review`), ship (`work`/`release`/`changelog`), extend (`plugins`/`config`/`introspect`/`completions`/`doctor`).** Anything else — GitHub-specific browsing, polyglot dep audits, code metrics, toolchain probes — is a plugin. Run `fledge plugins install --defaults` once to get the curated plugin set; you'll get back to feature parity with pre-v0.15 fledge.

If you are about to run `npm`, `cargo`, `make`, `git checkout -b`, or `gh pr create`, check first whether fledge has a wrapper — it usually does, and the wrapper often has `--json` and guardrails you want.

## First-time setup for an agent

```bash
export FLEDGE_NON_INTERACTIVE=1               # silence every prompt
fledge plugins install --defaults             # curated plugin set: github, deps, metrics
fledge introspect --json                       # dump the full command tree (incl. plugin commands)
fledge spec list --json                        # orient to the codebase via specs
```

After those four lines, every command and every flag is discoverable as data.

## Golden rules

1. **Prefer fledge subcommands over raw tools** when wrapped. E.g. use `fledge work start` instead of `git checkout -b`, `fledge run test` instead of guessing `cargo test` vs `npm test`.
2. **Always add `--json` when a command supports it** (see list below). Parse the JSON; do not screen-scrape pretty output.
3. **Set `FLEDGE_NON_INTERACTIVE=1` once** in your shell, or pass `--non-interactive` (alias `--ni`) per invocation. Every command then treats confirmation prompts as `--yes`; prompts with no default bail with a clear error instead of blocking forever.
4. **Check exit codes.** Non-zero means something failed, even if stdout looks fine.
5. **Don't mutate without running `fledge lanes run pre-commit` first** — it's the project-defined quality gate.

## Discover what fledge can do

```bash
fledge introspect --json             # full command tree — every subcommand, every flag
fledge introspect                    # same, human-readable indented listing
fledge --help                        # top-level command list (human text)
fledge <cmd> --help                  # per-command flags
fledge spec list --json              # all specs as a JSON array
fledge spec show <name> --json       # one spec's structure as JSON
fledge plugins list --json           # what extensions are active
```

`fledge introspect --json` is the right starting point for an agent that has never seen fledge before — it reveals the entire CLI surface (including plugin-installed commands) in one call.

Specs (`specs/<name>/*.spec.md` and companion files) are the source of truth for *why* a module exists. When you need context beyond what the code shows, read the spec — particularly `specs/<name>/context.md` (design decisions) and `specs/<name>/requirements.md` (user stories).

## Machine-readable surface (`--json`)

### Core commands

| Command | What you get | Use when |
|---------|-------------|----------|
| `fledge introspect --json` | Full command tree: every subcommand, every flag, every arg | First contact with fledge |
| `fledge spec list --json` | Array of spec summaries (module, version, status, sections, companions) | Orienting to a new codebase |
| `fledge spec show <name> --json` | Single spec detail (frontmatter + section list + companion status) | Need structured view of one module |
| `fledge spec check --json` | `{specs: [{name, version, status, errors, warnings, ...}], totals, strict}` | Spec-sync validation as data |
| `fledge ai status --json` | `{provider, model, host, *_source}` — what's active and where each value came from | Verifying provider config before invoking the LLM |
| `fledge ai models --provider {claude,ollama} --json` | Live model list (Ollama hits `/api/tags`; Claude returns curated aliases) | Picking a specific model |
| `fledge ask "..." --json` | `{question, answer, provider, model}` from the active LLM provider over the codebase | Answering a question about the code |
| `fledge review --json` | Single-model: `{base, file, diff_stats, spec_context, review, provider, model, reviews:[...]}` | Before opening a PR |
| `fledge review --with-model <ref> --json` | Multi-model panel: `reviews:[{provider, model, elapsed_seconds, review|error}, ...]` | Comparing models on the same diff |
| `fledge doctor --json` | `{sections:[{name, checks:[...], informational}], passed, failed}` — four sections (`fledge`, `Git`, `AI`, `Toolchains`). `Toolchains` is informational; missing tools don't count toward `failed`. | Debugging a broken setup |
| `fledge changelog --json` | Structured changelog from tags | Generating release notes |
| `fledge plugins list --json` | `{schema_version: 1, plugins: [{name, version, source, trust_tier, ...}]}` | Auditing plugin state |
| `fledge plugins audit --json` | `{schema_version: 1, audit: [{name, version, trust_tier, capabilities, has_lifecycle_hooks, ...}]}` | Capability/hook audit |
| `fledge plugins search --json` | `{schema_version: 1, results: [{name, full_name, stars, trust_tier, ...}]}` | GitHub search for `fledge-plugin`-tagged repos |
| `fledge plugins validate --json` | `{schema_version: 1, path, plugin_name, errors, warnings}` | CI gate before publish |
| `fledge lanes list --json` | `{schema_version: 1, lanes: [{name, description, steps, fail_fast, source?, trust_tier}]}` | Discovering lanes available to run |
| `fledge lanes search --json` | `{schema_version: 1, results: [...]}` (same shape as plugins search) | GitHub search for `fledge-lane`-tagged repos |
| `fledge lanes run <name> --json` | `{schema_version: 1, lane, success, duration_ms, fail_fast, steps: [...], failures: [...]}` | Running the project's own CI pipeline |
| `fledge lanes validate --json` | `{schema_version: 1, path, lane_count, errors, warnings}` | CI gate before publish |
| `fledge templates list --json` | `{schema_version: 1, templates: [{name, description, source, source_ref, path}]}` | Listing available templates |
| `fledge templates search --json` | `{schema_version: 1, results: [...]}` (same shape as plugins search) | GitHub search for `fledge-template`-tagged repos |
| `fledge templates validate --json` | `{schema_version: 1, reports: [{path, template, errors, warnings}]}` | CI gate before publish |
| `fledge work start <name> --json` | `{branch, base, type, prefix, issue}` — branch name the agent just created | Branch scripting |
| `fledge work pr --json` | `{url, number, title, head, base, draft}` — PR URL to report back | After agent finishes a task |
| `fledge work status --json` | `{branch, default, ahead, behind, pr?}` — current state of the branch | Pre-action sanity check |

### Plugin commands (after `plugins install --defaults`)

| Command | Plugin | What you get |
|---------|--------|-------------|
| `fledge checks --json` | `fledge-plugin-github` | Raw GitHub API response of CI check-runs for a branch |
| `fledge issues --json` / `fledge issues view <n> --json` | `fledge-plugin-github` | GitHub issues — list or one |
| `fledge prs --json` / `fledge prs view <n> --json` | `fledge-plugin-github` | GitHub PRs — list or one |
| `fledge deps --json` | `fledge-plugin-deps` | Dependency report from the ecosystem tool (`cargo outdated`, `npm audit`, …) |
| `fledge metrics --json` / `--churn --json` / `--tests --json` | `fledge-plugin-metrics` | LOC summary (tokei), per-file churn, test/source ratio |

Commands **without** `--json` (pretty output only): `init`, `spec init`, `spec new`, `run`, `watch`, `release`, `ai use`. If you need structured output from one of these, add it via a spec + PR — it's an accepted pattern.

**Envelope contract.** Every `--json` output in the three pillars (plugins/lanes/templates) is shaped as `{schema_version: 1, <resource>: [...]}` or `{schema_version: 1, action: "<verb>", ...}` for mutating commands. Top-level `schema_version` is the version contract: new fields are additive within v1; field removal/retyping requires a new schema_version. **Always read `<resource>` (or named keys) — never assume the top level is an array.** Pre-1.0 outputs that returned bare arrays were wrapped in tier C of the 1.0 readiness work; pinning to fledge ≥ that release means you can rely on the envelope.

## Non-interactive mode (the one-switch answer)

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
| `fledge ai use` | Errors with a clear "pass provider+model" message — no hang |
| `fledge work pr` | Skip the preview/confirm prompt (treat as --yes) |
| `fledge plugins install` | Skip trust-tier and capability-grant prompts |
| `fledge plugins publish` | Skip confirmations |
| `fledge plugins create` | Skip scaffolding prompts |
| `fledge lanes publish` | Skip description prompt |
| `fledge templates publish` | Skip the confirmation prompt |

Prompts that have **no sensible default** (e.g. `fledge ai use` being asked to pick a provider when none was specified) fail fast with a clear error naming the flag to pass instead. No silent hangs.

You can still pass `--yes`/`--force` per command if you prefer — they and `FLEDGE_NON_INTERACTIVE` compose.

## AI commands

`fledge ai`, `fledge ask`, and `fledge review` go through a provider abstraction. Two providers ship in core:

- **Claude** (default) — shells out to the `claude` CLI, which must be installed and authenticated on the host.
- **Ollama** — HTTP to any Ollama-speaking endpoint: local daemon (`http://localhost:11434`), Ollama Cloud / Turbo, or self-hosted. Supports a Bearer API key.

### Picking a provider — three ways

```bash
# 1. fledge ai use (writes to ~/.config/fledge/config.toml — persists)
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

**Every `fledge ask` invocation automatically prepends a compact index of all specs** (one line per module: name, version, status, files, first-paragraph purpose). The model can then cite specific specs in its answer even when the user didn't mention them.

```bash
fledge ask "how does work build branch names?" --json
fledge ask --with-specs work,trust "how do these modules interact?"
fledge ask --with-specs all "which modules touch GitHub?"
fledge ask --no-spec-index "quick Rust syntax question"
```

### `fledge review` — single or multi-model

`fledge review` auto-detects which modules a diff touches (via each spec's frontmatter `files:` and any edits under `specs/<name>/`) and includes their full spec + companion files as context.

```bash
# Single model (active config)
fledge review --json
fledge review --base HEAD~3 --json
fledge review --with-specs plugin --json
fledge review --no-auto-specs --json
fledge review --model opus --format checklist --json

# Multi-model panel — same diff, parallel critiques
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

### Open a PR with an AI-drafted body
```bash
# fledge work pr will:
#   1. Generate the body from commits (or via --ai for an LLM-drafted one)
#   2. Show a preview block (title, head→base, draft, body)
#   3. Prompt y/n  (skipped under FLEDGE_NON_INTERACTIVE or --yes)
#   4. Push and call gh pr create

fledge work pr --yes --json                                # heuristic body, scripted
fledge work pr --ai --yes --json                           # LLM-drafted body, scripted
fledge work pr --ai --provider ollama --model gpt-oss:120b-cloud --yes
```

### Before reporting a task done
```bash
fledge lanes run pre-commit                                 # fmt + lint + test + spec-check
# or the project-specific full lane:
fledge lanes run ci
```

### Verify CI is green (requires `fledge-plugin-github`)
```bash
fledge checks --json | jq '.check_runs[] | {name, conclusion}'
```

### Inspect a spec's deeper context as part of planning
```bash
cat specs/<name>/context.md      # design decisions
cat specs/<name>/requirements.md # user stories / acceptance
cat specs/<name>/tasks.md        # what's done, what's gapped
cat specs/<name>/testing.md      # test plan
```

## Exit codes

- `0` — success
- `1` — user-facing error (bad input, missing file, validation failure, prompt required in non-TTY)
- Non-zero exit also fires on `fledge spec check` errors, `fledge lanes run` failures, and `fledge review` errors

## Project-specific quality gate

This repo defines its own lanes in `fledge.toml`. The key ones for agents:

- `fledge lanes run pre-commit` — fmt + lint + test + spec-check (required before opening a PR)
- `fledge lanes run ci` — full CI pipeline locally
- `fledge lanes run check` — quick parallel fmt+lint then test
- `fledge spec check` — always run if you touched `src/` or `specs/`

## When things go wrong

- **A command hung**: you probably skipped `--yes` or `--force`. Cancel, re-run with the bypass flag (or set `FLEDGE_NON_INTERACTIVE=1` once).
- **`fledge ask` / `review` errored with auth**: the host's `claude` CLI isn't set up, or your Ollama config is wrong. Run `fledge ai status` to see what fledge thinks is active, then `fledge doctor` to verify the provider is reachable.
- **`fledge spec check` fails**: read the error — almost always a missing section, missing source file, or unknown status. Don't "fix" it by editing the validator.
- **`fledge work pr` fails on push**: you're probably not on a remote-tracking branch. Re-run after `git push -u origin HEAD` or debug with `fledge work status`.
- **`fledge checks` (or any command) says "unrecognized subcommand"**: the corresponding plugin isn't installed. Run `fledge plugins install --defaults` to get the curated set in one shot.
- **A multi-model `fledge review` panel had one slot fail**: that slot's `error` field has the cause; the other slots' reviews are still valid. `--with-model` is fault-tolerant by design.

## Extending fledge for better agent support

If a command you want doesn't expose `--json`, or a workflow isn't automatable, the right fix is:
1. Open an issue tagged `agent-surface`
2. Update the corresponding spec (`specs/<module>/<module>.spec.md`) — bump the version, add the new flag to Public API + Behavioral Examples
3. Implement + `fledge lanes run pre-commit` + PR

The project explicitly welcomes agent-surface improvements.
