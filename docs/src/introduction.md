# Introduction

One CLI for the entire dev lifecycle — scaffold, run tasks, sync specs, AI review, ship PRs, and release. Works with any language, outputs JSON for automation, and extends through plugins.

## Why I built this

I kept setting up the same boilerplate across projects. CI workflows, linters, task runners, the works. Every new repo meant copy-pasting from the last one and fixing whatever broke. fledge started as a scaffolding tool and grew into a full dev lifecycle CLI because once you have a tool that understands your project structure, it makes sense to keep going.

The core stays tight. Anything ecosystem-specific (lockfile parsers, GitHub clients, language toolchain probes) lives in [plugins](./plugins.md). You opt in to what you need.

## The six pillars

| Pillar | Commands | What it does |
|--------|----------|-------------|
| Scaffold | `templates` (`init`, `create`, `list`, `search`, `validate`, `publish`) | Start a project from a template, local or remote |
| Run | `run`, `lanes`, `watch` | Task runner, composable pipelines, file-watch reruns |
| Spec | `spec` | spec-sync. Modules declare their contract, AI uses it as context |
| AI | `ai`, `ask`, `review` | Provider/model selection, spec-aware Q&A, single and multi-model review |
| Ship | `work`, `release`, `changelog` | Branch and PR flow with AI-drafted bodies, version bump, tag, push |
| Extend | `plugins`, `config`, `introspect`, `completions`, `doctor` | Plugin protocol, global config, command-tree introspection, env health |

Start a project, run your tasks, evolve specs and code together, get AI to ask + review, ship to git/GitHub. Extend through plugins. That's the whole loop.

## The plugin layer

Three plugins extend fledge with commands that don't belong in every install. One command installs them all:

```bash
fledge plugins install --defaults
```

That gets you `checks`/`issues`/`prs` (GitHub), `deps`, and `metrics`. See [Extend: Plugins](./plugins.md) for the full list and how to build your own.

## Zero-config

It auto-detects your project type (Rust, Node, Go, Python, Ruby, Java, Swift) and generates sensible defaults. You don't need `fledge templates init` to get started, just `cd` into any existing project and run `fledge run test`. It works with zero config. When you want more control, `fledge run --init` generates a `fledge.toml` tailored to your stack.
