# AGENTS.md — fledge for AI agents

This doc is for AI agents (Claude Code, GPT-based coding agents, OpenHands, etc.) using the `fledge` CLI alongside a human. Humans get `README.md` and `docs/`; agents get this one page.

## What fledge is, in one paragraph

`fledge` is a single-binary dev-lifecycle CLI (Rust). It scaffolds projects from templates, runs tasks (`fledge run`), composes them into lanes (`fledge lane run`), manages branches and PRs (`fledge work`), validates specs (`fledge spec`), checks deps (`fledge deps`), runs AI review (`fledge review`), and more. It replaces ad-hoc Makefile + scripts + per-repo READMEs with a uniform verb/noun interface.

If you are about to run `npm`, `cargo`, `make`, `git checkout -b`, or `gh pr create`, check first whether fledge has a wrapper — it usually does, and the wrapper often has `--json` and guardrails you want.

## Golden rules

1. **Prefer fledge subcommands over raw tools** when wrapped. E.g. use `fledge work start` instead of `git checkout -b`, `fledge run test` instead of guessing `cargo test` vs `npm test`.
2. **Always add `--json` when a command supports it** (see list below). Parse the JSON; do not screen-scrape pretty output.
3. **Set `FLEDGE_NON_INTERACTIVE=1` once** in your shell, or pass `--non-interactive` (alias `--ni`) per invocation. Every command then treats confirmation prompts as `--yes`; prompts with no default bail with a clear error instead of blocking forever.
4. **Check exit codes.** Non-zero means something failed, even if stdout looks fine.
5. **Don't mutate without running `fledge lane run pre-commit` first** — it's the project-defined quality gate.

## Discover what fledge can do

```bash
fledge introspect --json             # full command tree — every subcommand, every flag
fledge introspect                    # same, human-readable indented listing
fledge --help                        # top-level command list (human text)
fledge <cmd> --help                  # per-command flags
fledge spec list --json              # all specs as a JSON array
fledge spec show <name> --json       # one spec's structure as JSON
```

`fledge introspect --json` is the right starting point for an agent that has never seen fledge before — it reveals the entire CLI surface in one call.

Specs (`specs/<name>/*.spec.md` and companion files) are the source of truth for *why* a module exists. When you need context beyond what the code shows, read the spec — particularly `specs/<name>/context.md` (design decisions) and `specs/<name>/requirements.md` (user stories).

## Machine-readable surface (`--json`)

| Command | What you get | Use when |
|---------|-------------|----------|
| `fledge introspect --json` | Full command tree: every subcommand, every flag, every arg | First contact with fledge |
| `fledge spec list --json` | Array of spec summaries (module, version, status, sections, companions) | Orienting to a new codebase |
| `fledge spec show <name> --json` | Single spec detail (frontmatter + section list + companion status) | Need structured view of one module |
| `fledge spec check --json` | `{specs: [{name, version, status, errors, warnings, ...}], totals, strict}` | Spec-sync validation as data |
| `fledge ask "..." --json` | `{question, answer, provider, model}` from the active LLM provider over the codebase | Answering a question about the code |
| `fledge review --json` | `{base, file, diff_stats, spec_context, review, provider, model}` AI code review of current changes (auto-includes specs of touched modules) | Before opening a PR |
| `fledge checks --json` | CI/CD check status for a branch | Verifying a branch is green |
| `fledge doctor --json` | Environment health diagnostics | Debugging a broken toolchain |
| `fledge deps --json` | Dependency report (outdated, audit, licenses) | Auditing a project |
| `fledge metrics --json` | LOC, churn, test ratio per file | Triaging hotspots |
| `fledge changelog --json` | Structured changelog from tags | Generating release notes |
| `fledge validate-template --json` | Template validation report | Before publishing a template |
| `fledge search <q> --json` | Template/plugin search results | Finding reusable pieces |
| `fledge plugins list --json` | Installed plugins with trust tier, capabilities | Auditing plugin state |
| `fledge issues --json` | GitHub issues list | Picking up work |
| `fledge lane run <name> --json` | Lane execution results | Running the project's own CI pipeline |
| `fledge work start <name> --json` | `{branch, base, type, prefix, issue}` — branch name the agent just created | Branch scripting |
| `fledge work pr --json` | `{url, number, title, head, base, draft}` — PR URL to report back | After agent finishes a task |
| `fledge work status --json` | `{branch, default, ahead, behind, pr?}` — current state of the branch | Pre-action sanity check |

Commands **without** `--json` (pretty output only): `init`, `spec init`, `spec new`, `run`, `publish`, `create-template`, `watch`, `release`. If you need structured output from one of these, add it via a spec + PR — it's an accepted pattern.

## Non-interactive mode (the one-switch answer)

Set this once at the top of your shell session and forget about it:

```bash
export FLEDGE_NON_INTERACTIVE=1
```

Or pass the flag per invocation: `fledge --non-interactive <cmd>` (alias `--ni`). Both are equivalent.

When non-interactive mode is active, every command that would otherwise prompt behaves **as if `--yes` / `--force` were passed**:

| Command | Effect |
|---------|--------|
| `fledge init` | Skip template-variable prompts (uses detected defaults) |
| `fledge templates publish` | Skip "update existing repo?" confirmation |
| `fledge templates create` | Skip name/description/type prompts |
| `fledge plugins install` | Skip trust-tier and capability-grant prompts |
| `fledge plugins publish` | Skip confirmations |
| `fledge plugins create` | Skip scaffolding prompts |
| `fledge lane publish` | Skip description prompt |

Prompts that have **no sensible default** (e.g. `fledge init` being asked to pick a template when none was specified) fail fast with a clear error naming the flag to pass instead. No silent hangs.

You can still pass `--yes`/`--force` per command if you prefer — they and `FLEDGE_NON_INTERACTIVE` compose.

Commands safe to call even without this flag (they never prompt): `ask`, `review`, `checks`, `doctor`, `deps`, `metrics`, `changelog`, `spec *`, `work start`, `work pr`, `work status`, `run`, `lane run`, `release`, `validate-template`, `search`, `plugins list`, `plugins remove`, `plugins update`, `plugins audit`, `issues`, `prs`, `watch`.

## AI commands

`fledge ask` and `fledge review` go through a provider abstraction. Two providers ship in core:

- **Claude** (default) — shells out to the `claude` CLI, which must be installed and authenticated on the host.
- **Ollama** — HTTP to any Ollama-speaking endpoint: local daemon (`http://localhost:11434`), Ollama Cloud / Turbo, or self-hosted. Supports a Bearer API key.

### Picking a provider

```bash
# Config (persists)
fledge config set ai.provider ollama
fledge config set ai.ollama.host https://ollama.com
fledge config set ai.ollama.api_key sk-...
fledge config set ai.ollama.model "llama3.3:70b"

# Or env vars (agent-shell friendly)
export FLEDGE_AI_PROVIDER=ollama
export OLLAMA_HOST=https://ollama.com
export OLLAMA_API_KEY=sk-...
export FLEDGE_AI_MODEL=llama3.3:70b

# Or per invocation (highest precedence)
fledge ask --provider ollama --model llama3.3:70b "..."
fledge review --provider claude --model opus-4
```

Precedence: CLI flag > env var > config > default (`claude`).

### `fledge ask` is spec-aware by default

**Every `fledge ask` invocation automatically prepends a compact index of all specs** (one line per module: name, version, status, files, first-paragraph purpose). Claude can then cite specific specs in its answer even when the user didn't mention them.

```bash
# Default: compact index of every spec auto-included
fledge ask "how does the work module build branch names?" --json

# Include full spec + all companion files for one or more modules
fledge ask --with-specs work "why does it sanitize names this way?"
fledge ask --with-specs work,trust "how do these modules interact?"
fledge ask --with-specs all "which modules touch GitHub?"

# Skip the index entirely (saves tokens for off-topic questions)
fledge ask --no-spec-index "quick Rust syntax question"
```

When `--with-specs <name>` is passed, fledge loads `specs/<name>/<name>.spec.md` plus every existing companion (`requirements.md`, `context.md`, `tasks.md`, `testing.md`) into the prompt. Companions carry the design rationale — usually what you want for *why* questions.

### `fledge review` (spec-aware, auto-detect)

`fledge review` auto-detects which modules a diff touches (via each spec's frontmatter `files:` and any edits under `specs/<name>/`) and includes their full spec + companion files as context. Nothing to pass — it just works.

```bash
fledge review --json                       # auto-detects, cites specs in review
fledge review --base HEAD~3 --json         # reviews last 3 commits with spec context
fledge review --with-specs plugin --json   # auto-detected + force-include plugin spec
fledge review --no-auto-specs --json       # back to spec-free review
fledge review --model opus --format checklist --json
```

The JSON output now contains a `spec_context` array listing every module whose spec was included. The prompt is explicitly constrained: Claude reviews *only the diff*, treats the specs as context-only, and must not suggest changes to code outside the diff or critique the specs themselves.

## Typical agent workflows

### Start a task
```bash
fledge spec list --json                                    # orient
fledge spec show <module> --json                           # dig into one area
fledge work start my-change -t feat                        # branch
```

### Before reporting a task done
```bash
fledge lane run pre-commit                                 # fmt + lint + test + spec-check
# or the project-specific full lane:
fledge lane run ci
```

### Open a PR
```bash
fledge work pr --title "feat: ..." --body "## Summary\n..."
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
- Non-zero exit also fires on `fledge spec check` errors, `fledge lane run` failures, and `fledge review` errors

## Project-specific quality gate

This repo defines its own lanes in `fledge.toml`. The key ones for agents:

- `fledge lane run pre-commit` — fmt + lint + test + spec-check (required before opening a PR)
- `fledge lane run ci` — full CI pipeline locally
- `fledge lane run check` — quick parallel fmt+lint then test
- `fledge spec check` — always run if you touched `src/` or `specs/`

## When things go wrong

- **A command hung**: you probably skipped `--yes` or `--force`. Cancel, re-run with the bypass flag.
- **`fledge ask` / `review` errored with auth**: the host's `claude` CLI isn't set up. This is a host-config problem, not a fledge bug.
- **`fledge spec check` fails**: read the error — almost always a missing section, missing source file, or unknown status. Don't "fix" it by editing the validator.
- **`fledge work pr` fails on push**: you're probably not on a remote-tracking branch. Re-run after `git push -u origin HEAD` or debug with `fledge work status`.

## Extending fledge for better agent support

If a command you want doesn't expose `--json`, or a workflow isn't automatable, the right fix is:
1. Open an issue tagged `agent-surface`
2. Update the corresponding spec (`specs/<module>/<module>.spec.md`) — bump the version, add the new flag to Public API + Behavioral Examples
3. Implement + `fledge lane run pre-commit` + PR

The project explicitly welcomes agent-surface improvements.
