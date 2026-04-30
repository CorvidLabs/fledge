# Quick Start

## Create a New Project

Pick a template and go:

```bash
fledge templates init my-tool --template rust-cli
```

That gives you a full Rust CLI project with clap, CI, and release automation in a `my-tool` directory.

Not sure what you want? Just run init without a template and fledge will walk you through it:

```bash
fledge templates init my-project
```

### Remote templates

Any GitHub repo works as a template source:

```bash
fledge templates init my-app --template CorvidLabs/fledge-templates/python-api
```

### Dry run first

Preview what you'd get before writing anything:

```bash
fledge templates init my-tool --template rust-cli --dry-run
```

See [Templates](../templates.md) for the full list of built-in and remote templates.

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

## What's Next

Once your project exists, here's the rest of the dev loop:

```bash
# Workflow pipelines
fledge lanes init                    # generate default lanes for your project type
fledge lanes run ci                  # run the full pipeline

# AI review and Q&A
fledge review                        # AI code review of your current branch
fledge ask "how does X work?"        # ask about the codebase

# Branch and PR workflow
fledge work start add-logging        # create a work branch
fledge work pr --ai                  # AI-drafted PR with preview + confirm

# Environment health
fledge doctor                        # anything broken in your env?

# Plugins (optional extras)
fledge plugins install --defaults    # adds checks, issues, prs, deps, metrics
```

Each of these has its own chapter — see the sidebar for details.
