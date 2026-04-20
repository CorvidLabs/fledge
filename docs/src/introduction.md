# Introduction

One CLI for your whole dev lifecycle. Scaffold, build, test, ship.

**fledge** is a Rust CLI that replaces the pile of tools you're currently juggling. Instead of `cookiecutter` + `make` + `gh` + custom scripts, you get one binary that handles everything from project creation to changelog generation.

## Why I built this

I kept setting up the same boilerplate across projects — CI workflows, linters, task runners, the works. Every new repo meant copy-pasting from the last one and fixing whatever broke. fledge started as a scaffolding tool and grew into a full dev lifecycle CLI because honestly, once you have a tool that understands your project structure, it makes sense to keep going.

## What it does

| Category | Commands | The gist |
|----------|----------|----------|
| **Scaffolding** | `init`, `list`, `create-template`, `search`, `publish`, `update`, `validate-template` | Create projects from templates, find and share templates |
| **Project Lifecycle** | `run`, `lane`, `spec`, `work`, `changelog` | Task runner, workflow pipelines, spec management, git workflow |
| **Project Health** | `doctor`, `metrics`, `deps` | Environment checks, code stats, dependency auditing |
| **GitHub** | `issues`, `prs`, `checks` | Issues, PRs, and CI status without leaving the terminal |
| **AI-Powered** | `review`, `ask` | Code review and codebase Q&A powered by Claude |
| **Extensibility** | `plugin`, `config`, `completions`, `tui` | Community plugins, config, shell completions, interactive UI |

It auto-detects your project type (Rust, Node, Go, Python, Ruby, Java, Swift), generates sensible defaults, and stays out of your way. Start with `fledge init`, define tasks in `fledge.toml`, compose them into lanes, and you've got a consistent workflow across all your projects.
