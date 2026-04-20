# Fledge Roadmap

Fledge is evolving from a project scaffolding tool into a full dev-lifecycle CLI — scaffold, spec, build, ship, monitor — all from one opinionated Rust binary.

## Current State (v0.7.0)

Shipped: `init`, `list`, `config`, `create-template`, `search`, `publish`, `update`, `spec`, `work`, `completions`, `issues`, `prs`, `review`, `ask`, `checks`, `run`, `changelog`, TUI (feature-gated). 8 built-in templates (Rust CLI/lib, Node CLI/lib, Python CLI, Go CLI, monorepo, static site), hook security, dry-run support, template versioning, version pinning with `@ref` syntax, project lifecycle commands, GitHub ops, AI-powered code review and Q&A, CI/CD status, task runner with language-aware defaults, changelog generation from git history. Distribution via Homebrew, install script, Nix flake, and shell completions auto-install.

---

## 0.3 — Template Ecosystem

Complete the template story: discovery, publishing, versioning, and more built-in templates.

- [x] `fledge search` improvements (#4) — GitHub template discovery with topic-based search
- [x] `fledge publish` — publish templates to GitHub with `fledge-template` topic (#6)
- [x] Template versioning and compatibility checks (#13)
- [x] Additional built-in templates: Python, Go, monorepo (#9)
- [ ] CorvidLabs template collection and org defaults (#8)
- [x] Publish to crates.io (#2)

## 0.4 — Project Lifecycle

Move beyond scaffolding. Fledge stays with you after `init`.

- [x] `fledge update` — re-apply template to existing projects (#11)
- [x] `fledge spec` — integrate spec-sync (`check`, `init`, `new`) (#32)
- [x] `fledge work start` — begin a feature branch with conventions (#33)
- [x] `fledge work pr` — create PR from current branch (#33)

## 0.5 — GitHub & AI Integration

Bring GitHub ops and AI assistance into the CLI.

- [x] `fledge issues` / `fledge prs` — list and manage GitHub issues and PRs (#34)
- [x] `fledge review` — AI-powered code review (#35)
- [x] `fledge ask` — ask questions about your codebase (#35)

## 0.6 — Distribution & Polish

Make fledge easy to install everywhere.

- [x] Homebrew formula (#12)
- [x] Install script (`curl | sh`)
- [x] Nix package (#12)
- [x] Shell completions auto-install (`fledge completions --install`)

## 0.7 — Task Runner & Observability

Run tasks, check CI, and generate changelogs — fledge becomes your daily driver.

- [x] `fledge run` — task runner with `fledge.toml`, language-aware defaults (#49)
- [x] `fledge checks` — view CI/CD status for any branch (#49)
- [x] `fledge changelog` — generate changelogs from git tags and conventional commits (#53)
- [x] Language-agnostic support — auto-detects Rust, Node, Go, Python, Ruby, Java (#51)

## 0.8 — Project Health (planned)

Dependency management, project metrics, and environment diagnostics.

- [ ] `fledge deps` — dependency health check (outdated packages, audit, license scan)
- [ ] `fledge metrics` — project stats (LOC, test coverage, complexity, churn)
- [ ] `fledge doctor` — environment diagnostics (toolchain versions, missing deps, config issues)

## 1.0 — Flows & Plugins

Extensible workflow automation — the workflow-as-code model, but in Rust.

- [ ] Flow system — composable, typed workflow pipelines (#36)
- [ ] Plugin architecture (Rust or WASM)
- [ ] Community flow registry
- [ ] Full end-to-end dev lifecycle coverage

---

## Design Principles

- **Single binary** — no runtime dependencies, instant startup
- **Opinionated defaults, escape hatches** — works out of the box, customizable when needed
- **Rust-native** — performance, safety, and cross-platform distribution without Ruby/Python/Node baggage
- **Spec-driven** — every module has a spec; specs are the source of truth
