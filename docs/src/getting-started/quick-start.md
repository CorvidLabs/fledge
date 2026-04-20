# Quick Start

## Create Your First Project

Pick a template and go:

```bash
fledge init my-tool --template rust-cli
```

That gives you a full Rust CLI project with clap, CI, and release automation in a `my-tool` directory.

## Browse Templates

Not sure what you want? Just run init without a template and fledge will walk you through it:

```bash
fledge init my-project
```

## Use a Remote Template

Any GitHub repo works as a template source:

```bash
fledge init my-app --template CorvidLabs/fledge-templates/react-app
```

## Dry Run First

Preview what you'd get before writing anything:

```bash
fledge init my-tool --template rust-cli --dry-run
```

## Set Up Tasks

Once your project exists, generate a `fledge.toml` with auto-detected tasks:

```bash
cd my-tool
fledge run --init    # generates fledge.toml based on what it finds
fledge run build
fledge run test
```

## Workflow Pipelines

Lanes chain tasks together. Think of `fledge lane ci` as your local CI:

```bash
fledge lane --init       # generate default lanes for your project type
fledge lane              # see what's available
fledge lane ci           # run the full pipeline
fledge lane ci --dry-run # just show the plan
```

Lanes support parallel groups and inline commands. More on that in [Lanes & Pipelines](../lanes.md).

## Project Health

```bash
fledge doctor            # anything broken in your env?
fledge metrics           # LOC by language
fledge metrics --churn   # most-changed files
fledge deps              # list deps
fledge deps --outdated   # find stale ones
fledge deps --audit      # security check
```

## Plugins

Community extensions, git-style:

```bash
fledge plugin search deploy
fledge plugin install someone/fledge-deploy
fledge plugin list
```

## Feature Branches + PRs

```bash
fledge work start add-logging    # creates feat/add-logging
# ... hack on your feature ...
fledge work pr                   # opens a PR
fledge work status               # where are we?
```

## CI + Code Review

```bash
fledge checks                    # CI status
fledge review                    # AI code review
fledge ask "how does X work?"    # ask about the codebase
```

## Changelog

```bash
fledge changelog                 # from git tags + conventional commits
fledge changelog --unreleased    # what's new since last tag
```

## All Templates

```bash
fledge list
```

Shows everything available — built-in, configured repos, and local paths.

### Built-in Templates

| Template | What you get |
|----------|--------------|
| `rust-cli` | Rust CLI with clap, CI, release automation |
| `ts-bun` | TypeScript project on Bun |

More templates (Angular, Go, Python, Swift, etc.) available at [CorvidLabs/fledge-templates](https://github.com/CorvidLabs/fledge-templates).
