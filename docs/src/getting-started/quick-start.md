# Quick Start

## Create Your First Project

The simplest way to get started is to create a new project:

```bash
fledge init my-tool --template rust-cli
```

This creates a new Rust CLI project in a `my-tool` directory with all the scaffolding you need.

## Browse Templates Interactively

If you're not sure which template to use, let fledge guide you:

```bash
fledge init my-project
```

You'll be prompted to select a template and fill in project variables.

## Use a Remote GitHub Template

Any GitHub repository can be a template source. Use the `owner/repo` syntax:

```bash
fledge init my-app --template CorvidLabs/fledge-templates/react-app
```

## Preview Changes with Dry Run

Before committing to creating a project, preview what would be generated:

```bash
fledge init my-tool --template rust-cli --dry-run
```

## Set Up Your Project

Once your project is created, initialize the task runner:

```bash
cd my-tool
fledge run --init    # generates fledge.toml with language-aware defaults
fledge run build     # run the build task
fledge run test      # run tests
```

## Start a Feature Branch

Use the workflow commands to manage branches and PRs:

```bash
fledge work start add-logging    # creates feat/add-logging branch
# ... make changes ...
fledge work pr                   # creates a PR from current branch
fledge work status               # check branch and PR status
```

## Check CI and Review Code

```bash
fledge checks                    # view CI/CD status
fledge review                    # AI-powered code review
fledge ask "how does X work?"    # ask about the codebase
```

## Generate a Changelog

```bash
fledge changelog                 # from git tags + conventional commits
fledge changelog --unreleased    # see what's new since last tag
```

## List Available Templates

See all available templates (built-in and configured):

```bash
fledge list
```

## Built-in Templates

fledge comes with several built-in templates ready to use:

| Template | Description |
|----------|-------------|
| `rust-cli` | Rust CLI application with clap, CI, and release automation |
| `rust-lib` | Rust library crate with docs and publishing workflow |
| `swift-pkg` | Swift package with Package.swift, CI, and coding conventions |
| `ts-bun` | TypeScript project with Bun runtime |
| `angular-app` | Angular application with mobile-first setup |
| `python-cli` | Python CLI with Click, tests, and packaging |
| `go-cli` | Go CLI with Cobra, Makefile, and CI |
| `node-cli` | Node.js CLI with TypeScript |
| `node-lib` | Node.js library with TypeScript and npm publishing |
| `monorepo` | Monorepo with workspace tooling |
| `static-site` | Static site with build pipeline |

Each template includes sensible defaults, CI/CD workflows, and best practices for its language ecosystem.
