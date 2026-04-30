# The six pillars

fledge organizes the dev workflow into six pillars. They cover the whole lifecycle from project creation to release, and they're explicitly the *only* things core ships. Anything else is a plugin.

```text
Scaffold --> Run --> Spec --> AI --> Ship
                                       \
                  Extend (the protocol that lets plugins add the rest)
```

## Scaffold

Get a project off the ground. Pick a template (built-in or remote), scaffold it, and you're writing code in under a minute.

**Commands:** `templates init`, `templates create`, `templates validate`, `templates list`, `templates search`, `templates publish`

## Run

Define your tasks, wire them into pipelines, watch files for re-runs. This is where `fledge.toml` lives.

**Commands:** `run`, `lanes`, `watch`

## Spec

spec-sync. Every module declares a contract (`specs/<name>/<name>.spec.md` plus optional companion files). The contract is the source of truth for *why* a module exists, and AI commands inject the relevant specs as context. Code and docs literally cannot drift.

**Commands:** `spec` (subcommands: `check`, `init`, `list`, `show`, `new`)

## AI

Provider-agnostic AI in the daily-driver path. Switch between Claude CLI and any Ollama-speaking endpoint in one line. Ask questions about your codebase. Review your diff with one model or a panel of them in parallel.

**Commands:** `ai` (`status`, `models`, `use`), `ask`, `review`

## Ship

Branch, draft an AI-written PR body, preview, confirm, push. Then bump version, generate changelog, tag, push the tag.

**Commands:** `work` (`start`, `pr`, `status`), `release`, `changelog`

For GitHub-specific browsing of the resulting PR, checks, and issues, install [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github). Ships in the default plugin set.

## Extend

Plugins, configuration, command-tree introspection, environment diagnostics, shell completions. The mechanism layer.

**Commands:** `plugins`, `config`, `introspect`, `completions`, `doctor`

`fledge plugins install --defaults` installs the curated set:

- [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github) adds `checks`, `issues`, `prs`
- [`fledge-plugin-deps`](https://github.com/CorvidLabs/fledge-plugin-deps) adds `deps`
- [`fledge-plugin-metrics`](https://github.com/CorvidLabs/fledge-plugin-metrics) adds `metrics` (Rust binary linking `tokei` as a library)

Why these are plugins and not core: each one bakes an ecosystem assumption (GitHub-only, polyglot lockfile parsers, niche metrics) that not every fledge user needs. Plugins keep the binary small and let each capability evolve independently.
