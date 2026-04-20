# Introduction

One CLI, six stages, your whole dev lifecycle.

**fledge** is a Rust CLI that replaces the pile of tools you're currently juggling. Instead of `cookiecutter` + `make` + `gh` + custom scripts, you get one binary that handles everything from project creation to changelog generation.

## Why I built this

I kept setting up the same boilerplate across projects. CI workflows, linters, task runners, the works. Every new repo meant copy-pasting from the last one and fixing whatever broke. fledge started as a scaffolding tool and grew into a full dev lifecycle CLI because once you have a tool that understands your project structure, it makes sense to keep going.

## What it does

| Pillar | Tagline | Commands |
|--------|---------|----------|
| **Start** | Scaffold and discover | `init`, `list`, `search`, `create-template`, `publish`, `validate-template`, `update` |
| **Build** | Configure and run | `run`, `flow`, `config`, `doctor` |
| **Develop** | Branch and spec | `work`, `spec` |
| **Review** | Quality and insight | `review`, `ask`, `metrics`, `deps` |
| **Ship** | Track and release | `issues`, `prs`, `checks`, `changelog` |
| **Extend** | Grow the tool | `plugin`, `completions`, `tui` |

Start a project, build your tasks and config, develop features on branches, review quality before merging, ship releases. Extend runs alongside everything with plugins and completions.

It auto-detects your project type (Rust, Node, Go, Python, Ruby, Java, Swift) and generates sensible defaults. Start with `fledge init`, define tasks in `fledge.toml`, compose them into flows, and you've got a consistent workflow across all your projects.
