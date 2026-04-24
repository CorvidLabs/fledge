# AGENTS.md — fledge for AI agents

This doc is for AI agents (Claude Code, GPT-based coding agents, OpenHands, etc.) using the `fledge` CLI alongside a human. Humans get `README.md` and `docs/`; agents get this one page.

## What fledge is, in one paragraph

`fledge` is a single-binary dev-lifecycle CLI (Rust). It scaffolds projects from templates, runs tasks (`fledge run`), composes them into lanes (`fledge lane run`), manages branches and PRs (`fledge work`), validates specs (`fledge spec`), checks deps (`fledge deps`), runs AI review (`fledge review`), and more. It replaces ad-hoc Makefile + scripts + per-repo READMEs with a uniform verb/noun interface.

If you are about to run `npm`, `cargo`, `make`, `git checkout -b`, or `gh pr create`, check first whether fledge has a wrapper — it usually does, and the wrapper often has `--json` and guardrails you want.

## Golden rules

1. **Prefer fledge subcommands over raw tools** when wrapped. E.g. use `fledge work start` instead of `git checkout -b`, `fledge run test` instead of guessing `cargo test` vs `npm test`.
2. **Always add `--json` when a command supports it** (see list below). Parse the JSON; do not screen-scrape pretty output.
3. **Always pass `--yes` / `--force` to commands that otherwise prompt** (see TTY table). A stalled prompt looks like success to you but blocks forever.
4. **Check exit codes.** Non-zero means something failed, even if stdout looks fine.
5. **Don't mutate without running `fledge lane run pre-commit` first** — it's the project-defined quality gate.

## Discover what fledge can do

```bash
fledge --help                        # top-level command list (human text)
fledge <cmd> --help                  # per-command flags
fledge spec list --json              # all 32 specs as a JSON array
fledge spec show <name> --json       # one spec's structure as JSON
```

Specs (`specs/<name>/*.spec.md` and companion files) are the source of truth for *why* a module exists. When you need context beyond what the code shows, read the spec — particularly `specs/<name>/context.md` (design decisions) and `specs/<name>/requirements.md` (user stories).

## Machine-readable surface (`--json`)

| Command | What you get | Use when |
|---------|-------------|----------|
| `fledge spec list --json` | Array of spec summaries (module, version, status, sections, companions) | Orienting to a new codebase |
| `fledge spec show <name> --json` | Single spec detail (frontmatter + section list + companion status) | Need structured view of one module |
| `fledge ask "..." --json` | `{question, answer}` from an LLM over the codebase | Answering a question about the code |
| `fledge review --json` | `{base, file, diff_stats, review}` AI code review of current changes | Before opening a PR |
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

Commands **without** `--json` (pretty output only): `init`, `spec check`, `spec init`, `spec new`, `run`, `publish`, `create-template`, `watch`, `work`, `release`. If you need structured output from one of these, add it via a spec + PR — it's an accepted pattern.

## Commands that block on TTY prompts

These commands use `dialoguer` prompts. Agents must pass the bypass flag or the process will hang forever.

| Command | Bypass flag | Effect |
|---------|------------|--------|
| `fledge init` | `--yes` | Skip interactive template-variable prompts (uses detected defaults) |
| `fledge templates publish` | `--yes` | Skip "update existing repo?" confirmation |
| `fledge templates create` | `--yes` | Skip name/description/type prompts |
| `fledge plugins install` | `--yes` + `--force` | Skip trust-tier and capability-grant prompts |
| `fledge plugins publish` | `--yes` | Skip confirmations |
| `fledge plugins create` | `--yes` | Skip scaffolding prompts |

Commands safe to call non-interactively today: `ask`, `review`, `checks`, `doctor`, `deps`, `metrics`, `changelog`, `spec *`, `work start`, `work pr`, `run`, `lane run`, `release`, `validate-template`, `search`, `plugins list`, `plugins remove`, `plugins update`, `plugins audit`, `issues`, `prs`, `watch`.

## AI commands

`fledge ask` and `fledge review` delegate to the `claude` CLI, which must be installed and authenticated on the host. The agent running fledge inherits whatever auth the `claude` CLI has.

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

### `fledge review`

```bash
fledge review --json                   # reviews current diff vs main
fledge review --base HEAD~3 --json     # reviews last 3 commits
fledge review --model opus --format checklist --json
```

Review does not yet feed specs into its prompt — if you want spec-aware review, run `fledge ask --with-specs <name> "review the recent change to X module"` as a workaround.

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
