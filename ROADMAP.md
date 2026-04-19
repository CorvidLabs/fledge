# Fledge Roadmap

Fledge is evolving from a project scaffolding tool into a full dev-lifecycle CLI — scaffold, spec, build, ship, monitor — all from one opinionated Rust binary.

## Current State (v0.4.0)

Shipped: `init`, `list`, `config`, `create-template`, `search`, `publish`, `update`, `spec`, `work`, `completions`, TUI (feature-gated). 8 built-in templates (Rust CLI/lib, Node CLI/lib, Python CLI, Go CLI, monorepo, static site), hook security, dry-run support, template versioning, version pinning with `@ref` syntax, project lifecycle commands.

---

## 0.3 — Template Ecosystem

Complete the template story: discovery, publishing, versioning, and more built-in templates.

- [x] `fledge search` improvements (#4) — GitHub template discovery with topic-based search
- [x] `fledge publish` — publish templates to GitHub with `fledge-template` topic (#6)
- [x] Template versioning and compatibility checks (#13)
- [x] Additional built-in templates: Python, Go, monorepo (#9)
- [ ] CorvidLabs template collection and org defaults (#8)
- [ ] Publish to crates.io (#2)

## 0.4 — Project Lifecycle

Move beyond scaffolding. Fledge stays with you after `init`.

- [x] `fledge update` — re-apply template to existing projects (#11)
- [x] `fledge spec` — integrate spec-sync (`check`, `init`, `new`) (#32)
- [x] `fledge work start` — begin a feature branch with conventions (#33)
- [x] `fledge work pr` — create PR from current branch (#33)

## 0.5 — GitHub & AI Integration

Bring GitHub ops and AI assistance into the CLI.

- [ ] `fledge issues` / `fledge prs` — list and manage GitHub issues and PRs
- [ ] `fledge review` — AI-powered code review
- [ ] `fledge ask` — ask questions about your codebase

## 0.6 — Distribution & Polish

Make fledge easy to install everywhere.

- [ ] Homebrew formula (#12)
- [ ] Install script (`curl | sh`)
- [ ] Nix package (#12)
- [ ] Shell completions auto-install

## 1.0 — Lanes & Plugins

Extensible workflow automation — the Fastlane model, but in Rust.

- [ ] Lane system — composable, typed workflow pipelines
- [ ] Plugin architecture (Rust or WASM)
- [ ] Community lane registry
- [ ] Full end-to-end dev lifecycle coverage

---

## Design Principles

- **Single binary** — no runtime dependencies, instant startup
- **Opinionated defaults, escape hatches** — works out of the box, customizable when needed
- **Rust-native** — performance, safety, and cross-platform distribution without Ruby/Python/Node baggage
- **Spec-driven** — every module has a spec; specs are the source of truth
