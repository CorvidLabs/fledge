# Introduction

Get your projects ready to fly.

**fledge** is a fast, opinionated dev-lifecycle CLI built in Rust. Scaffold projects from templates, manage specs, run tasks, check CI, review code, and ship — all from one binary.

## Why fledge?

- **Fast** — native Rust binary, no runtime dependencies
- **Smart defaults** — pulls author/org from git config, renders dates, computes name variants automatically
- **Remote templates** — use any GitHub repo as a template source with `owner/repo` syntax
- **Full lifecycle** — scaffolding, specs, tasks, CI checks, changelogs, GitHub ops, AI review
- **Language-agnostic** — auto-detects Rust, Node, Go, Python, Ruby, Java and adapts defaults
- **Extensible** — create your own templates with a simple `template.toml` manifest
- **Safe** — remote template hooks require explicit confirmation before running
- **Optional TUI** — interactive template browser with `--features tui`

fledge is designed to be the one CLI you reach for throughout the dev lifecycle. Start a project with `init`, manage specs with `spec`, run tasks with `run`, check CI with `checks`, review code with `review`, and generate changelogs with `changelog`. Whether you're scaffolding a Rust CLI, a Go service, or a TypeScript project, fledge provides a consistent, powerful toolset with sensible defaults that just work.

## What can fledge do?

| Category | Commands | Description |
|----------|----------|-------------|
| **Scaffolding** | `init`, `list`, `create-template`, `search`, `publish`, `update` | Create projects from templates, discover and share templates |
| **Project Lifecycle** | `run`, `spec`, `work`, `changelog` | Task runner, spec management, git workflow, changelog generation |
| **GitHub** | `issues`, `prs`, `checks` | View issues, PRs, and CI status from the terminal |
| **AI-Powered** | `review`, `ask` | Code review and codebase Q&A via Claude |
| **Configuration** | `config`, `completions`, `tui` | Global settings, shell completions, interactive UI |
