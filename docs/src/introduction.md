# Introduction

Dev-lifecycle CLI — get your projects ready to fly.

**fledge** is a fast, opinionated CLI built in Rust. Scaffold projects, run tasks, compose workflow pipelines, manage plugins, check dependencies, review code, and ship — all from one binary.

## Why fledge?

- **Fast** — native Rust binary, no runtime dependencies
- **Smart defaults** — pulls author/org from git config, renders dates, computes name variants automatically
- **Remote templates** — use any GitHub repo as a template source with `owner/repo` syntax
- **Full lifecycle** — scaffolding, tasks, lanes, specs, CI checks, changelogs, GitHub ops, AI review
- **Composable lanes** — chain tasks into named pipelines with parallel execution
- **Plugin system** — community extensions via external executables (git-style)
- **Language-agnostic** — auto-detects Rust, Node, Go, Python, Ruby, Java and adapts defaults
- **Extensible** — create templates, plugins, and custom lane steps
- **Safe** — remote template hooks require explicit confirmation before running
- **Optional TUI** — interactive template browser with `--features tui`

fledge is designed to be the one CLI you reach for throughout the dev lifecycle. Start a project with `init`, define tasks in `fledge.toml` and compose them into lanes with `lane`, manage specs with `spec`, check dependencies with `deps`, review code with `review`, and extend with community plugins via `plugin`. Whether you're scaffolding a Rust CLI, a Go service, or a TypeScript project, fledge provides a consistent, powerful toolset with sensible defaults that just work.

## What can fledge do?

| Category | Commands | Description |
|----------|----------|-------------|
| **Scaffolding** | `init`, `list`, `create-template`, `search`, `publish`, `update`, `validate-template` | Create projects from templates, discover, validate, and share templates |
| **Project Lifecycle** | `run`, `lane`, `spec`, `work`, `changelog` | Task runner, workflow pipelines, spec management, git workflow |
| **Project Health** | `doctor`, `metrics`, `deps` | Environment diagnostics, code metrics, dependency auditing |
| **GitHub** | `issues`, `prs`, `checks` | View issues, PRs, and CI status from the terminal |
| **AI-Powered** | `review`, `ask` | Code review and codebase Q&A via Claude |
| **Extensibility** | `plugin`, `config`, `completions`, `tui` | Plugins, global settings, shell completions, interactive UI |
