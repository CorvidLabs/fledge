# Introduction

One CLI, six stages, your whole dev lifecycle.

**fledge** is a Rust CLI that replaces the pile of tools you're currently juggling. Instead of `cookiecutter` + `make` + `gh` + custom scripts, you get one binary that handles everything from project creation to changelog generation.

## Why I built this

I kept setting up the same boilerplate across projects — CI workflows, linters, task runners, the works. Every new repo meant copy-pasting from the last one and fixing whatever broke. fledge started as a scaffolding tool and grew into a full dev lifecycle CLI because honestly, once you have a tool that understands your project structure, it makes sense to keep going.

## What it does

| Pillar | Tagline | Commands |
|--------|---------|----------|
| **Start** | Scaffold and discover | `init`, `list`, `search`, `create-template`, `publish`, `validate-template`, `update` |
| **Build** | Configure and run | `run`, `flow`, `config`, `doctor` |
| **Develop** | Branch and spec | `work`, `spec` |
| **Review** | Quality and insight | `review`, `ask`, `metrics`, `deps` |
| **Ship** | Track and release | `issues`, `prs`, `checks`, `changelog` |
| **Extend** | Grow the tool | `plugin`, `completions`, `tui` |

The lifecycle flows naturally: Start a project, Build your tasks and config, Develop features on branches, Review quality before merging, and Ship releases. Extend runs alongside everything — plugins and completions enhance any stage.

It auto-detects your project type (Rust, Node, Go, Python, Ruby, Java, Swift), generates sensible defaults, and stays out of your way. Start with `fledge init`, define tasks in `fledge.toml`, compose them into flows, and you've got a consistent workflow across all your projects.
