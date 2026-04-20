# Review: Quality and Insight

Check your code before it ships. AI-powered review, codebase Q&A, code metrics, and dependency auditing.

## AI code review with `fledge review`

Get feedback on your changes before opening a PR. Diffs your branch against the base and runs it through Claude.

```bash
fledge review
fledge review --base main
fledge review --file src/auth.rs    # review a single file
```

## Ask your codebase with `fledge ask`

Got a question about how something works? Ask it.

```bash
fledge ask "how does the template rendering work?"
fledge ask "what tests cover the config module?"
```

## Code metrics with `fledge metrics`

Lines of code by language, file churn from git history, and test coverage ratio.

```bash
fledge metrics
fledge metrics --churn           # most-changed files
fledge metrics --tests           # test file ratio
fledge metrics --churn --tests --json
```

## Dependency health with `fledge deps`

Find outdated packages, run security audits, and scan licenses. Works across Rust, Node, Go, Python, and Ruby.

```bash
fledge deps
fledge deps --outdated
fledge deps --audit
fledge deps --licenses
fledge deps --outdated --audit --licenses --json
```
