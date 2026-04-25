# Using fledge with AI Agents

fledge is designed for humans *and* AI agents to use the same CLI. Human-facing docs (the rest of this book) cover *why* and *how-to*; this page covers what an agent specifically needs to know.

The canonical short-form entrypoint for agents lives at [AGENTS.md](https://github.com/CorvidLabs/fledge/blob/main/AGENTS.md) in the repository root. This page is its long-form companion — point agents to either.

## Design principles

fledge follows four rules that make it agent-usable:

1. **Structured output is first-class.** Most commands accept `--json`. Never require an agent to screen-scrape.
2. **Non-interactive bypass exists.** Every command that prompts has a `--yes` flag. Silence isn't success — it's a hung process.
3. **Specs are machine-readable.** `fledge spec list --json` and `fledge spec show <name> --json` let agents discover the codebase's intent without filesystem spelunking.
4. **The whole CLI introspects.** `fledge introspect --json` dumps the entire command tree (incl. plugin commands) — one call teaches an agent the surface.

## First-time agent setup

```bash
export FLEDGE_NON_INTERACTIVE=1               # silence every prompt
fledge plugins install --defaults             # github + deps + metrics + templates-remote + doctor
fledge introspect --json                       # full command tree (incl. plugin commands)
fledge spec list --json                        # semantic project map
```

After those four lines, every command and flag is discoverable as data.

## The agent-facing surface

### Core commands with `--json`

| Command | Payload shape |
|---------|---------------|
| `fledge introspect --json` | Full command tree: `{name, about, aliases, args, subcommands}` recursively |
| `fledge spec list --json` | Array of `{name, version, status, path, files, section_count, required_sections, companions, missing_companions}` |
| `fledge spec show <name> --json` | `{name, version, status, path, files, sections, companions, missing_companions}` |
| `fledge spec check --json` | `{specs: [{name, version, status, file_count, section_count, required_count, errors, warnings}], totals: {checked, errors, warnings}, strict}` |
| `fledge ai status --json` | `{provider, model, host, *_source}` — what's active and the source of each value |
| `fledge ai models --provider {claude,ollama} --json` | Live model list |
| `fledge ask "..." --json` | `{question, answer, provider, model}` |
| `fledge review --json` | Single-model: `{base, file, diff_stats, spec_context, review, provider, model, reviews:[...]}`. With `--with-model`: `reviews:[{provider, model, elapsed_seconds, review|error}, ...]` |
| `fledge doctor --json` | `{sections:[{name, checks:[...]}], passed, failed}` |
| `fledge changelog --json` | Structured changelog |
| `fledge plugins list --json` | Installed plugins |
| `fledge lanes run <name> --json` | Lane execution results |
| `fledge work start <name> --json` | `{branch, base, type, prefix, issue}` |
| `fledge work pr --json` | `{url, number, title, head, base, draft}` (suppresses preview/confirm) |
| `fledge work status --json` | `{branch, default, ahead, behind, pr}` |

### Plugin commands with `--json` (after `plugins install --defaults`)

| Command | Plugin |
|---------|--------|
| `fledge checks --json` | `fledge-plugin-github` — raw GitHub API `check-runs` response |
| `fledge issues --json` / `issues view <n> --json` | `fledge-plugin-github` |
| `fledge prs --json` / `prs view <n> --json` | `fledge-plugin-github` |
| `fledge deps --json` | `fledge-plugin-deps` — ecosystem tool's native output |
| `fledge metrics --json` / `--churn --json` / `--tests --json` | `fledge-plugin-metrics` |
| `fledge templates-search --json` | `fledge-plugin-templates-remote` |
| `fledge doctor-tools --json` | `fledge-plugin-doctor` — `[{tool, group, status, version}, ...]` |

### Non-interactive mode (one switch)

Set `FLEDGE_NON_INTERACTIVE=1` in your environment, or pass `--non-interactive` (alias `--ni`) per invocation. Both flip a global flag that every prompt site observes: every `--yes`/`--force` is auto-promoted, and prompts that need user input bail cleanly instead of hanging.

Commands covered: `fledge templates init`, `fledge templates create`, `fledge work pr` (preview/confirm), `fledge ai use`, `fledge plugins install`, `fledge plugins publish`, `fledge plugins create`, `fledge lanes publish`, `fledge templates-publish` (plugin).

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
fledge review --with-model ollama:gpt-oss:120b-cloud --with-model ollama:qwen3-coder:480b-cloud --json
fledge review --no-active --with-model claude:opus-4.7,ollama:gpt-oss:120b-cloud --json
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
fledge review --with-model ollama:gpt-oss:120b-cloud --with-model ollama:qwen3-coder:480b-cloud --json

# 5. Open PR with AI-drafted body
fledge work pr --ai --yes --json

# 6. Wait for CI
fledge checks --json    # via fledge-plugin-github
```
