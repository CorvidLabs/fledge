# Fledge Roadmap

Fledge started as a scaffolding tool and grew into a full dev-lifecycle CLI. Scaffold, spec, build, ship, monitor, all from one Rust binary.

## Current State (v1.0.0)

Shipped: `init`, `list`, `config`, `create-template`, `search`, `publish`, `update`, `spec`, `work`, `completions`, `issues`, `prs`, `review`, `ask`, `checks`, `run`, `changelog`, `lane`, `doctor`, `deps`, `metrics`, `plugin`, `validate-template`, TUI (feature-gated). 6 built-in templates (rust-cli, ts-bun, python-cli, go-cli, ts-node, static-site), community templates via `CorvidLabs/fledge-templates`. Hook security, dry-run support, template versioning, version pinning with `@ref` syntax, project lifecycle commands, GitHub ops, AI-powered code review and Q&A, CI/CD status, task runner with language-aware defaults, changelog generation from git history, composable workflow pipelines (lanes), environment diagnostics, dependency health (outdated/audit/licenses), project metrics (LOC/churn/test ratio), plugin architecture (install/remove/search/run). Distribution via Homebrew, install script, Nix flake, and shell completions auto-install.

---

## 0.3: Template Ecosystem

Complete the template story: discovery, publishing, versioning, and more built-in templates.

- [x] `fledge search` improvements (#4) - GitHub template discovery with topic-based search
- [x] `fledge publish` - publish templates to GitHub with `fledge-template` topic (#6)
- [x] Template versioning and compatibility checks (#13)
- [x] Additional built-in templates: Python, Go, monorepo (#9)
- [ ] CorvidLabs template collection and org defaults (#8)
- [x] Publish to crates.io (#2)

## 0.4: Project Lifecycle

Move beyond scaffolding. Fledge stays with you after `init`.

- [x] `fledge update` - re-apply template to existing projects (#11)
- [x] `fledge spec` - integrate spec-sync (`check`, `init`, `new`) (#32)
- [x] `fledge work start` - begin a feature branch with conventions (#33)
- [x] `fledge work pr` - create PR from current branch (#33)

## 0.5: GitHub & AI Integration

Bring GitHub ops and AI assistance into the CLI.

- [x] `fledge issues` / `fledge prs` - list and manage GitHub issues and PRs (#34)
- [x] `fledge review` - AI-powered code review (#35)
- [x] `fledge ask` - ask questions about your codebase (#35)

## 0.6: Distribution & Polish

Make fledge easy to install everywhere.

- [x] Homebrew formula (#12)
- [x] Install script (`curl | sh`)
- [x] Nix package (#12)
- [x] Shell completions auto-install (`fledge completions --install`)

## 0.7: Task Runner & Observability

Run tasks, check CI, and generate changelogs. fledge becomes your daily driver.

- [x] `fledge run` - task runner with `fledge.toml`, language-aware defaults (#49)
- [x] `fledge checks` - view CI/CD status for any branch (#49)
- [x] `fledge changelog` - generate changelogs from git tags and conventional commits (#53)
- [x] Language-agnostic support - auto-detects Rust, Node, Go, Python, Ruby, Java (#51)

## 0.8: Project Health

Dependency management, project metrics, and environment diagnostics.

- [x] `fledge deps` - dependency health check (outdated packages, audit, license scan)
- [x] `fledge metrics` - project stats (LOC, test coverage, complexity, churn)
- [x] `fledge doctor` - environment diagnostics (toolchain versions, missing deps, config issues)

## 1.0: Lanes & Plugins

Extensible workflow automation. Workflow-as-code, but in Rust.

- [x] Lane system - composable, typed workflow pipelines (#36)
- [x] Plugin architecture - install, remove, list, search, run plugins from GitHub
- [x] Community lane registry - search and import lanes from GitHub
- [x] Full end-to-end dev lifecycle coverage - scaffold, spec, build, ship, monitor

---

## Design Principles

- **Single binary** - no runtime deps, instant startup
- **Opinionated defaults, escape hatches** - works out of the box, customizable when needed
- **Rust-native** - fast, safe, cross-platform without Ruby/Python/Node baggage
- **Spec-driven** - every module has a spec; specs are the source of truth
