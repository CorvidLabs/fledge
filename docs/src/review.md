# Review: Quality and Insight

Check your code before it ships. AI-powered review, codebase Q&A, code metrics, and dependency auditing.

## AI Code Review

Get feedback on your changes before opening a PR. Diffs your branch against the base and runs it through Claude.

```bash
fledge review                       # all changes on current branch
fledge review --base main           # diff against main
fledge review --base develop        # diff against develop
fledge review --file src/auth.rs    # review a single file
fledge review --json                # machine-readable output
```

### What it reviews

The review checks for:
- **Bugs and logic errors** — off-by-ones, null handling, race conditions
- **Security issues** — injection, auth bypasses, secret exposure
- **Performance** — unnecessary allocations, N+1 queries, blocking calls
- **Clarity** — confusing naming, missing context, overly complex code

### How it works

1. Computes a diff between your current branch and the base (defaults to `main` or `master`)
2. Shows you a summary of changed files
3. Sends the diff to Claude with a structured review prompt
4. Returns findings with file locations and severity

### Tips

- Review early and often — smaller diffs get better reviews
- Use `--file` to focus on the module you're least confident about
- Pipe `--json` output into other tools for automated quality gates
- Combine with lanes for pre-PR automation:

```toml
[lanes.pre-pr]
steps = [
  { parallel = ["lint", "test"] },
  { run = "fledge review" },
]
```

### Requirements

Requires the [Claude CLI](https://docs.anthropic.com/en/docs/claude-code) installed and authenticated. See [Troubleshooting](./troubleshooting.md) if `fledge review` reports that Claude is not found.

## Ask Your Codebase

Got a question about how something works? Ask it. Uses Claude to answer questions with your project as context.

```bash
fledge ask "how does the template rendering work?"
fledge ask "what tests cover the config module?"
fledge ask "what does the release command do?"
fledge ask "where are environment variables loaded?" --json
```

### Good questions to ask

- **Architecture**: "how is the request pipeline structured?"
- **Coverage**: "what tests cover the payment module?"
- **Onboarding**: "what does this project do and how is it organized?"
- **Dependencies**: "what external services does this code talk to?"
- **Debugging**: "where could a null pointer exception happen in the auth flow?"

### For AI agents

Both `review` and `ask` support `--json` output, making them composable in automated pipelines:

```bash
# AI agent workflow
REVIEW=$(fledge review --json)
# Parse findings, fix issues, re-run
```

## Code Metrics

Lines of code by language, file churn from git history, and test coverage ratio.

```bash
fledge metrics                   # full overview
fledge metrics --churn           # most-changed files
fledge metrics --tests           # test file ratio
fledge metrics --churn --tests --json
```

### What you get

- **LOC by language** — breakdown of source lines across detected languages
- **Churn** — files with the most commits (high churn + low coverage = risk)
- **Test ratio** — percentage of files that are test files vs source files

## Dependency Health

Find outdated packages, run security audits, and scan licenses. Works across Rust, Node, Go, Python, and Ruby.

```bash
fledge deps                      # overview
fledge deps --outdated           # outdated packages
fledge deps --audit              # security vulnerabilities
fledge deps --licenses           # license inventory
fledge deps --outdated --audit --licenses --json
```

### Cross-language support

| Language | Outdated | Audit | Licenses |
|----------|----------|-------|----------|
| Rust | `cargo outdated` | `cargo audit` | `cargo license` |
| Node | `npm outdated` | `npm audit` | `license-checker` |
| Go | `go list -m -u` | `govulncheck` | `go-licenses` |
| Python | `pip list --outdated` | `pip-audit` | `pip-licenses` |
| Ruby | `bundle outdated` | `bundle audit` | `license_finder` |

fledge auto-detects your project type and runs the appropriate tools. If a tool isn't installed, it tells you how to get it.
