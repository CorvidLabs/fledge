# Introduction

Get your projects ready to fly.

**fledge** is a fast, opinionated project scaffolding CLI built in Rust. Create new projects from templates — local or remote — with smart defaults, Tera-powered rendering, and zero boilerplate.

## Why fledge?

- **Fast** — native Rust binary, no runtime dependencies
- **Smart defaults** — pulls author/org from git config, renders dates, computes name variants automatically
- **Remote templates** — use any GitHub repo as a template source with `owner/repo` syntax
- **Extensible** — create your own templates with a simple `template.toml` manifest
- **Safe** — remote template hooks require explicit confirmation before running
- **Optional TUI** — interactive template browser with `--features tui`

fledge is designed to eliminate boilerplate setup when starting new projects. Whether you're scaffolding a Rust CLI, a Swift package, or a TypeScript project, fledge provides a consistent, powerful templating system with sensible defaults that just work.
