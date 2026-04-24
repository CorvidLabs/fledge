# Using fledge with AI Agents

fledge is designed for humans *and* AI agents to use the same CLI. Human-facing docs (the rest of this book) cover *why* and *how-to*; this page covers what an agent specifically needs to know.

The canonical short-form entrypoint for agents lives at `AGENTS.md` in the repository root. This page is its long-form companion — link or point agents to either.

## Design principles

fledge follows three rules that make it agent-usable:

1. **Structured output is first-class.** Most commands accept `--json`. Never require an agent to screen-scrape.
2. **Non-interactive bypass exists.** Every command that prompts has a `--yes` or `--force` flag. Silence isn't success — it's a hung process.
3. **Specs are machine-readable.** `fledge spec list --json` and `fledge spec show <name> --json` let agents discover the codebase's intent without filesystem spelunking.

## The agent-facing surface

### Commands with `--json`

| Command | Payload shape |
|---------|---------------|
| `fledge spec list --json` | Array of `{name, version, status, path, files, section_count, required_sections, companions, missing_companions}` |
| `fledge spec show <name> --json` | `{name, version, status, path, files, sections, companions, missing_companions}` |
| `fledge ask "..." --json` | `{question, answer}` |
| `fledge review --json` | `{base, file, diff_stats, review}` |
| `fledge checks --json` | CI check status |
| `fledge doctor --json` | Environment diagnostics |
| `fledge deps --json` | Dependency report |
| `fledge metrics --json` | LOC / churn / test ratio |
| `fledge changelog --json` | Structured changelog |
| `fledge validate-template --json` | Template validation report |
| `fledge search <q> --json` | Search results |
| `fledge plugins list --json` | Installed plugins |
| `fledge issues --json` | GitHub issues |
| `fledge lane run <name> --json` | Lane execution results |

### Commands that block without `--yes`

`fledge init`, `fledge templates publish`, `fledge templates create`, `fledge plugins install`, `fledge plugins publish`, `fledge plugins create`. Pass `--yes` (and `--force` for plugin install) in agent contexts.

### AI-powered commands

`fledge ask` and `fledge review` wrap the `claude` CLI. The host environment must have `claude` installed and authenticated. These commands will exit non-zero if auth is missing — that's a host-config issue, not a fledge issue.

## Typical agent workflows

### Orient to a new repo

```bash
fledge doctor --json                 # is the toolchain present?
fledge spec list --json              # what modules exist, what's their status?
fledge spec show <interesting> --json  # drill in
cat specs/<name>/context.md          # design decisions (not yet in JSON)
```

### Do a task

```bash
fledge work start my-change -t feat  # branch created off main
# ... edit files ...
fledge lane run pre-commit           # fmt + lint + test + spec-check
fledge work pr --title "..." --body "..."  # push + open PR
```

### Validate before reporting done

```bash
fledge lane run pre-commit           # project's required gate
fledge spec check                    # no drift between code and specs
fledge checks --json                 # CI status after push
```

## Reading specs programmatically

Every module in `src/` has a matching spec under `specs/<module>/`. The `.spec.md` file holds the formal API, invariants, and change log. The companion files hold:

| File | What's inside |
|------|----------------|
| `requirements.md` | User stories, acceptance criteria, constraints, out-of-scope |
| `tasks.md` | What's done, what's gapped, review sign-offs |
| `context.md` | Design decisions, files-to-read-first, current status |
| `testing.md` | Unit/integration/manual test plan |

`fledge spec show <name> --json` returns the frontmatter and section list, but not the body text. For body text, read the file directly.

## Exit codes

- `0` — success
- Non-zero — some failure (validation, missing prereq, user-facing error). Always check.

## What's not yet agent-friendly

These are known gaps; PRs welcome. Each is tracked via the `agent-surface` label.

- `fledge run`, `fledge init`, `fledge spec check/init/new`, `fledge work`, `fledge release` have no `--json` today
- `fledge ask` does not automatically feed specs into its prompt — agents must paste spec excerpts themselves
- No global `--non-interactive` flag that forces `--yes` on every subcommand at once
- No `fledge introspect --json` to dump the full command tree as JSON

## Contributing agent-surface improvements

1. Open an issue tagged `agent-surface` describing what you need.
2. Update the relevant spec (`specs/<module>/<module>.spec.md`) — bump version, add the new flag to Public API and Behavioral Examples, add a Change Log entry.
3. Implement. Run `fledge lane run pre-commit`. Open a PR via `fledge work pr`.

The goal is that every fledge command, eventually, is equally usable by a human at a terminal and an agent scripting against it.
