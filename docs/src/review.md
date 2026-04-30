# AI: Ask and Review

The AI pillar is the daily-driver path for asking questions about your code and reviewing diffs before they land. Provider-agnostic (Claude or any Ollama-speaking endpoint), spec-aware, and capable of running multiple models in parallel against the same diff.

## Pick your provider and model

```bash
fledge ai use                                  # interactive picker
fledge ai use ollama llama3.2:latest           # scriptable
fledge ai use claude sonnet
fledge ai status                               # shows provider, model, host, and where each value came from
fledge ai models --provider ollama             # live list of installed/cloud models
fledge ai models --provider ollama --search cloud
```

Per-invocation overrides via `--provider` / `--model` flags work on every AI command and take precedence over config and env vars.

## AI Code Review

Get feedback on your changes before opening a PR. Diffs your branch against the base and runs it through the configured model.

```bash
fledge review                       # all changes on current branch
fledge review --base main           # diff against main
fledge review --base develop        # diff against develop
fledge review --file src/auth.rs    # review a single file
fledge review --json                # machine-readable output
```

### Multi-model review

Pass `--with-model <provider[:model]>` to add another slot to the panel. All slots run in parallel against the same diff and spec context. Per-slot failures don't abort the panel, you still get reviews from the models that succeeded.

```bash
# Active config + another model
fledge review --with-model ollama

# Comma-separated, exclude active config
fledge review --no-active --with-model claude:sonnet,ollama:llama3.2:latest

# JSON output gains a reviews[] array; legacy fields preserved when panel size is 1
fledge review --with-model ollama --json
```

The text output prints cyan banner headers between model slots and includes per-slot elapsed seconds. JSON output's `reviews[]` array has one entry per slot with `provider`, `model`, `elapsed_seconds`, and either `review` or `error`.

### What it reviews

- **Bugs and logic errors**: off-by-ones, null handling, race conditions
- **Security issues**: injection, auth bypasses, secret exposure
- **Performance**: unnecessary allocations, N+1 queries, blocking calls
- **Clarity**: confusing naming, missing context, overly complex code

### How it works

1. Computes a diff between your current branch and the base (defaults to `main` or `master`)
2. Auto-detects which spec modules the diff touches (from each spec's `files:` frontmatter and any `specs/<name>/` edits) and includes them as context
3. Sends the diff + spec context to the model(s) with a structured review prompt
4. Returns findings with file locations and severity

The prompt is explicitly constrained: the model reviews *only the diff*, treats specs as context-only, and must not suggest changes to code outside the diff or critique the specs themselves.

### Tips

- Review early and often, smaller diffs get better reviews
- Use `--file` to focus on the module you're least confident about
- Multi-model is most useful for high-stakes diffs. The models often disagree, and the disagreement is the signal
- Pipe `--json` output into other tools for automated quality gates
- Combine with lanes for pre-PR automation:

```toml
[lanes.pre-pr]
steps = [
  { parallel = ["lint", "test"] },
  { run = "fledge review --json > review.json" },
]
```

## Ask Your Codebase

Got a question about how something works? Ask it. The model gets your project's spec index automatically as context.

```bash
fledge ask "how does the template rendering work?"
fledge ask "what tests cover the config module?"
fledge ask --with-specs work,trust "how do these modules interact?"
fledge ask --with-specs all "which modules touch GitHub?"
fledge ask --no-spec-index "quick Rust syntax question"
```

### Spec-awareness

By default `fledge ask` injects a compact one-line-per-module index of every spec into the prompt. The model can cite specific specs in its answer even when you didn't mention them. Pass `--with-specs <names>` to include the *full* spec + companion files (requirements, context, tasks, testing) for one or more modules, useful for *why* questions where the design rationale matters.

### Good questions to ask

- **Architecture**: "how is the request pipeline structured?"
- **Coverage**: "what tests cover the payment module?"
- **Onboarding**: "what does this project do and how is it organized?"
- **Dependencies**: "what external services does this code talk to?"
- **Debugging**: "where could a null pointer exception happen in the auth flow?"

### For AI agents

Both `review` and `ask` support `--json` output, making them composable in automated pipelines:

```bash
REVIEW=$(fledge review --json)
# Parse findings, fix issues, re-run
```

## Related plugins

The default plugin set adds two commands that complement AI review:

- **`fledge metrics`** ([`fledge-plugin-metrics`](https://github.com/CorvidLabs/fledge-plugin-metrics)) — LOC summary, most-changed files, test/source ratio
- **`fledge deps`** ([`fledge-plugin-deps`](https://github.com/CorvidLabs/fledge-plugin-deps)) — dependency outdated/audit checks, auto-detected from lockfiles

Install with `fledge plugins install --defaults`. See the [CLI Reference](./cli-reference.md#fledge-deps-plugin) for full options and supported ecosystems.
