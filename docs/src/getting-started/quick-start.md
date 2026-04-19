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

## Skip All Prompts with Defaults

If you want to skip confirmations and use defaults, use `--yes`:

```bash
fledge init my-tool --template rust-cli --yes
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

Each template includes sensible defaults, CI/CD workflows, and best practices for its language ecosystem.
