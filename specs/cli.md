# CLI Specification

## Overview

Fledge is a project scaffolding CLI that creates new repositories from templates with CorvidLabs conventions baked in. It replaces manual copy-paste project setup with a single command.

## Commands

### `fledge init <name>`

Creates a new project directory from a template.

**Arguments:**
- `name` (required) — Name of the project to create. Used as directory name, package name, and repo name.

**Flags:**
- `--template, -t <template>` — Template to use. If omitted, prompts interactively.
- `--no-git` — Skip git init and initial commit.
- `--no-install` — Skip dependency installation.
- `--output, -o <path>` — Parent directory for the project (default: current directory).

**Behavior:**
1. Resolve template (from flag or interactive prompt)
2. Create project directory at `<output>/<name>`
3. Copy and render template files using Tera templating
4. Replace template variables (project name, author, date, etc.)
5. Initialize git repo (unless `--no-git`)
6. Install dependencies (unless `--no-install`)
7. Print summary of created files

### `fledge list`

Lists all available templates with descriptions.

**Output format:**
```
Available templates:
  rust-cli      Rust CLI application with clap, CI, and release automation
  rust-lib      Rust library crate with docs and publishing workflow
  swift-pkg     Swift package with Package.swift, CI, and coding conventions
  ts-bun        TypeScript project with Bun runtime
  angular-app   Angular application with mobile-first setup
```

### `fledge add <component>`

Adds a component to an existing project (future).

**Planned components:**
- `ci` — GitHub Actions workflows
- `claude` — CLAUDE.md with project conventions
- `spec-sync` — spec-sync configuration and initial specs
- `license` — LICENSE file

## Template Variables

Templates use Tera syntax (`{{ variable }}`). Available variables:

| Variable | Source | Example |
|----------|--------|---------|
| `project_name` | CLI argument | `my-project` |
| `project_name_snake` | Derived | `my_project` |
| `project_name_pascal` | Derived | `MyProject` |
| `author` | Git config or prompt | `Leif` |
| `year` | Current date | `2026` |
| `date` | Current date | `2026-04-18` |
| `description` | Interactive prompt | `A cool project` |
| `github_org` | Config or prompt | `CorvidLabs` |

## Template Structure

Templates live in `templates/<name>/` with a `template.toml` manifest:

```toml
[template]
name = "rust-cli"
description = "Rust CLI application with clap, CI, and release automation"
min_fledge_version = "0.1.0"

[prompts]
description = { message = "Project description", default = "A new Rust CLI" }

[files]
# Files to render with Tera (glob patterns)
render = ["**/*.rs", "**/*.toml", "**/*.md", "**/*.yml"]
# Files to copy as-is
copy = ["**/*.png", "**/*.ico"]
# Files to skip
ignore = ["template.toml"]
```

## Configuration

Global config at `~/.config/fledge/config.toml`:

```toml
[defaults]
author = "Leif"
github_org = "CorvidLabs"
license = "MIT"

[templates]
# Additional template directories beyond built-in
paths = ["~/my-templates"]
# Remote template registries (future)
# registries = ["https://fledge.corvidlabs.com/templates"]
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Template not found |
| 3 | Directory already exists |
| 4 | Template rendering error |

## Source Files

- `src/main.rs` — CLI entry point and argument parsing
- `src/init.rs` — Project initialization logic
- `src/templates.rs` — Template loading, rendering, and variable resolution
- `src/config.rs` — Global configuration management
- `src/prompts.rs` — Interactive prompts
