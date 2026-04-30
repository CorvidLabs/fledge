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

Lanes chain tasks together. Think of `fledge lanes run ci` as your local CI:

```bash
fledge lanes init            # generate default lanes for your project type
fledge lanes list            # see what's available
fledge lanes run ci          # run the full pipeline
fledge lanes run ci --dry-run  # just show the plan
```

Lanes support parallel groups and inline commands. More on that in [Lanes & Pipelines](../lanes.md).

## Project Health

```bash
fledge doctor            # anything broken in your env? (core; includes Toolchains)
```

After `fledge plugins install --defaults`:

```bash
fledge metrics           # LOC by language          (fledge-plugin-metrics)
fledge metrics --churn   # most-changed files       (fledge-plugin-metrics)
fledge deps              # list deps                (fledge-plugin-deps)
fledge deps --outdated   # find stale ones          (fledge-plugin-deps)
fledge deps --audit      # security check           (fledge-plugin-deps)
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
fledge checks                    # CI status (fledge-plugin-github)
fledge review                    # AI code review (core)
fledge ask "how does X work?"    # ask about the codebase (core)
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

Shows everything available: built-in (`rust-cli`, `ts-bun`, `python-cli`, `go-cli`, `ts-node`, `static-site`), configured repos, and local paths. More templates (Angular, MCP server, Deno, Swift, etc.) at [CorvidLabs/fledge-templates](https://github.com/CorvidLabs/fledge-templates). See [Templates](../templates.md) for the full list.
