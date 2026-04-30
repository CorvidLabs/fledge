# Using fledge with AI agents

fledge is designed for humans *and* AI agents to use the same CLI. Human-facing docs (the rest of this book) cover *why* and *how-to*. This page covers what an agent specifically needs to know.

The canonical short-form entrypoint for agents lives at [AGENTS.md](https://github.com/CorvidLabs/fledge/blob/main/AGENTS.md) in the repository root. This page is its long-form companion. Point agents to either. For how specs work and why they matter, see [Spec: Spec-sync](./spec.md).

## Design principles

fledge follows four rules that make it agent-usable:

1. **Structured output is first-class.** Most commands accept `--json`. Never require an agent to screen-scrape.
2. **Non-interactive bypass exists.** Every command that prompts has a `--yes` flag. Silence isn't success, it's a hung process.
3. **Specs are machine-readable.** `fledge spec list --json` and `fledge spec show <name> --json` let agents discover the codebase's intent without filesystem spelunking.
4. **The whole CLI introspects.** `fledge introspect --json` dumps the entire command tree (incl. plugin commands). One call teaches an agent the surface.

## First-time agent setup

```bash
export FLEDGE_NON_INTERACTIVE=1               # silence every prompt
fledge plugins install --defaults             # github + deps + metrics
fledge introspect --json                      # full command tree (incl. plugin commands)
fledge spec list --json                       # semantic project map
```

After those four lines, every command and flag is discoverable as data.

## The agent-facing surface

### Core commands with `--json`

Every `--json` output is `{schema_version: 1, ...}`. Two patterns coexist:

- Pillar list/query commands use `{schema_version: 1, <resource>: [...]}`.
- Cross-cutting commands use `{schema_version: 1, action: "<verb>", ...}`.

| Command | Payload shape |
|---------|---------------|
| `fledge introspect --json` | `{schema_version: 1, name, about, aliases, args, subcommands}` recursively |
| `fledge spec list --json` | `{schema_version: 1, action: "spec_list", specs: [...]}` |
| `fledge spec show <name> --json` | `{schema_version: 1, action: "spec_show", spec: {...}}` |
| `fledge spec check --json` | `{schema_version: 1, action: "spec_check", specs, totals, strict}` |
| `fledge ai status --json` | `{schema_version: 1, action: "ai_status", provider, model, host, *_source}`. What's active and the source of each value |
| `fledge ai models --provider {claude,ollama} --json` | `{schema_version: 1, action: "ai_models", provider, models: [...]}` |
| `fledge ask "..." --json` | `{schema_version: 1, action: "ask", question, answer, provider, model}` |
| `fledge review --json` | `{schema_version: 1, action: "review", base, file, diff_stats, spec_context, reviews: [...], review?, provider?, model?}`. Top-level `review`/`provider`/`model` only when panel size is 1 |
| `fledge doctor --json` | `{schema_version: 1, action: "doctor", sections: [...], passed, failed}`. Four sections (`fledge`, `Git`, `AI`, `Toolchains`). `Toolchains` is informational, missing tools render dimmed and aren't counted toward `failed` |
| `fledge changelog --json` | `{schema_version: 1, action: "changelog", releases: [{tag, date, sections}]}` |
| `fledge run --list --json` | `{schema_version: 1, action: "run_list", auto_detected, tasks: [...]}` |
| `fledge run <task> --json` | `{schema_version: 1, action: "run_task", task, command, exit_code, success, stdout, stderr}` |
| `fledge templates list --json` | `{schema_version: 1, templates: [...]}` |
| `fledge templates search --json` | `{schema_version: 1, results: [...]}` |
| `fledge templates validate --json` | `{schema_version: 1, reports: [...]}` |
| `fledge plugins list --json` | `{schema_version: 1, plugins: [...]}` |
| `fledge plugins audit --json` | `{schema_version: 1, audit: [...]}` |
| `fledge plugins search --json` | `{schema_version: 1, results: [...]}` |
| `fledge plugins validate --json` | `{schema_version: 1, path, plugin_name, errors, warnings}` |
| `fledge lanes list --json` | `{schema_version: 1, lanes: [...]}` |
| `fledge lanes search --json` | `{schema_version: 1, results: [...]}` |
| `fledge lanes run <name> --json` | `{schema_version: 1, lane, success, duration_ms, fail_fast, steps, failures}` |
| `fledge lanes validate --json` | `{schema_version: 1, path, lane_count, errors, warnings}` |
| `fledge work start <name> --json` | `{schema_version: 1, action: "work_start", branch, base, type, prefix, issue}` |
| `fledge work pr --json` | `{schema_version: 1, action: "work_pr", url, number, title, head, base, draft}` |
| `fledge work status --json` | `{schema_version: 1, action: "work_status", branch, default, ahead, behind, pr?}` |

### Plugin commands with `--json` (after `plugins install --defaults`)

| Command | Plugin |
|---------|--------|
| `fledge checks --json` | `fledge-plugin-github`. Raw GitHub API `check-runs` response |
| `fledge issues --json` / `issues view <n> --json` | `fledge-plugin-github` |
| `fledge prs --json` / `prs view <n> --json` | `fledge-plugin-github` |
| `fledge deps --json` | `fledge-plugin-deps`. Ecosystem tool's native output |
| `fledge metrics --json` / `--churn --json` / `--tests --json` | `fledge-plugin-metrics`. LOC summary (tokei linked as a library), per-file churn, test/source ratio |

### Non-interactive mode (one switch)

> **Important:** Without `FLEDGE_NON_INTERACTIVE=1`, any command with a prompt will hang in a headless environment.

Set `FLEDGE_NON_INTERACTIVE=1` in your environment, or pass `--non-interactive` (alias `--ni`) per invocation. Both flip a global flag that every prompt site observes. Every `--yes`/`--force` is auto-promoted, and prompts that need user input bail cleanly instead of hanging.

Commands covered: `fledge templates init`, `fledge templates create`, `fledge templates publish`, `fledge work pr` (preview/confirm), `fledge ai use`, `fledge plugins install`, `fledge plugins publish`, `fledge plugins create`, `fledge lanes publish`.

### AI-powered commands

`fledge ai`, `fledge ask`, and `fledge review` route through a provider abstraction. Two providers ship in core:

| Provider | Transport | Auth | Use case |
|----------|-----------|------|----------|
| `claude` (default) | `claude` CLI shell-out | Whatever `claude` is already authenticated with | Best-in-class reasoning, paid |
| `ollama` | HTTP to `<host>/api/generate` | Optional Bearer token | Local-only, offline, cloud alternatives, self-hosted |

Select via `fledge ai use <provider> [model]` (writes to config), `FLEDGE_AI_PROVIDER=ollama`, or `--provider ollama` per invocation. `fledge ai status` reports the active triplet and the source of each value.

#### `fledge ask` is spec-aware

By default, every `fledge ask` invocation prepends a compact one-line-per-module index of every spec into the prompt. Pass `--with-specs <names>` to include the *full* spec + companion files for one or more modules.

#### `fledge review` is multi-model-capable

Pass `--with-model <provider[:model]>` (repeatable, comma-separated) to run multiple models in parallel against the same diff and spec context. The JSON output's `reviews[]` array has one entry per slot. Per-slot failures don't abort the panel.

```bash
fledge review --json
fledge review --with-model ollama --json
fledge review --no-active --with-model claude:sonnet,ollama --json
```

## Typical agent workflow

```bash
# 1. Orient
fledge introspect --json
fledge spec list --json

# 2. Branch
fledge work start fix-issue-42 --branch-type fix --issue 42 --json

# 3. Code, test
fledge run test
fledge lanes run pre-commit

# 4. Review (multi-model for high-confidence findings)
fledge review --with-model ollama --json

# 5. Open PR with AI-drafted body
fledge work pr --ai --yes --json

# 6. Wait for CI
fledge checks --json    # via fledge-plugin-github
```
