# Quick Start

## Use Fledge in an Existing Project

Already have a project? Just `cd` in and go:

```bash
cd my-project
fledge run test     # auto-detects your stack, runs the right command
fledge run build
fledge run lint
```

No config file needed. When you want to customize, generate one:

```bash
fledge run --init   # creates fledge.toml with detected defaults
```

See [Existing Projects](./existing-projects.md) for the full guide.

## Create a New Project

Pick a template and go:

```bash
fledge templates init my-tool --template rust-cli
```

That gives you a full Rust CLI project with clap, CI, and release automation in a `my-tool` directory.

## Browse Templates

Not sure what you want? Just run init without a template and fledge will walk you through it:

```bash
fledge templates init my-project
```

## Use a Remote Template

Any GitHub repo works as a template source:

```bash
fledge templates init my-app --template CorvidLabs/fledge-templates/python-api
```

## Dry Run First

Preview what you'd get before writing anything:

```bash
fledge templates init my-tool --template rust-cli --dry-run
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
fledge lanes init            # generate default lanes for your project type
fledge lanes list            # see what's available
fledge lane ci               # run the full pipeline
fledge lane ci --dry-run     # just show the plan
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
fledge plugins search deploy
fledge plugins install someone/fledge-deploy           # latest
fledge plugins install someone/fledge-deploy@v1.0.0    # pinned version
fledge plugins list
```

## Work Branches + PRs

```bash
fledge work start add-logging                # creates leif/feat/add-logging (default: {author}/{type}/{name})
fledge work start fix-typo --branch-type fix        # creates leif/fix/fix-typo
fledge work start bump-deps --branch-type chore     # creates leif/chore/bump-deps
# ... hack on your branch ...
fledge work pr                               # opens a PR
fledge work status                           # where are we?
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
fledge templates list
```

Shows everything available: built-in, configured repos, and local paths.

### Built-in Templates

| Template | What you get |
|----------|--------------|
| `rust-cli` | Rust CLI with clap, CI, release automation |
| `ts-bun` | TypeScript on Bun with Biome |
| `python-cli` | Python CLI with Click and Ruff |
| `go-cli` | Go CLI with Cobra |
| `ts-node` | TypeScript on Node with tsx and Biome |
| `static-site` | Vanilla HTML/CSS/JS, no dependencies |

More templates (Angular, MCP server, Deno, Swift, etc.) available at [CorvidLabs/fledge-templates](https://github.com/CorvidLabs/fledge-templates).
