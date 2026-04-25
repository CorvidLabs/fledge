# Introduction

**fledge: one Rust binary, six pillars, spec-driven by default. Templates scaffold, lanes run, plugins extend, spec-sync keeps the docs honest about the code — and any LLM drives the same CLI you do.**

## Why I built this

I kept setting up the same boilerplate across projects. CI workflows, linters, task runners, the works. Every new repo meant copy-pasting from the last one and fixing whatever broke. fledge started as a scaffolding tool and grew into a full dev lifecycle CLI because once you have a tool that understands your project structure, it makes sense to keep going.

In v0.15 the tool got smaller. Anything ecosystem-specific (lockfile parsers, GitHub clients, language toolchain probes) moved out to plugins. The core stays tight; you opt in to what you need.

## The Six Pillars

| Pillar | Commands | What it does |
|--------|----------|-------------|
| **Scaffold** | `templates` (`init`, `create`, `validate`, `list`) | Local templates pillar — start any project |
| **Run** | `run`, `lanes`, `watch` | Task runner, composable pipelines, file-watch reruns |
| **Spec** | `spec` | spec-sync — modules declare their contract; AI uses it as context |
| **AI** | `ai`, `ask`, `review` | Provider+model selection, spec-aware Q&A, single- and multi-model review |
| **Ship** | `work`, `release`, `changelog` | Branch + PR flow with AI-drafted bodies, version bump, tag, push |
| **Extend** | `plugins`, `config`, `introspect`, `completions`, `doctor` | Plugin protocol, global config, command-tree introspection, env health |

Start a project, run your tasks, evolve specs and code together, get AI to ask + review, ship to git/GitHub. Extend through plugins. That's the whole loop.

## The plugin layer

Five plugins took over commands removed from core in v0.15. One command installs them all:

```bash
fledge plugins install --defaults
```

That gets you `checks`/`issues`/`prs` (GitHub), `deps`, `metrics`, `templates-search`/`templates-publish`, and `doctor-tools` back. See the [Plugins page](./plugins.md) for the full set.

## Zero-config

It still auto-detects your project type (Rust, Node, Go, Python, Ruby, Java, Swift) and generates sensible defaults. You don't need `fledge templates init` to get started — just `cd` into any existing project and run `fledge run test`. It works with zero config. When you want more control, `fledge run --init` generates a `fledge.toml` tailored to your stack.
