# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Step timing for lanes — each step shows elapsed time, lane summary shows total time
- Plugin lifecycle hooks — `pre_init`, `post_work_start`, `pre_pr` fire at fledge lifecycle events
- Parallel lane steps accept inline commands alongside task references
- SECURITY.md — vulnerability reporting policy and security model documentation
- CONTRIBUTING.md — development setup, workflow, code guidelines, and contribution process
- Doctor guide page in documentation (`docs/src/doctor.md`)
- Troubleshooting page in documentation (`docs/src/troubleshooting.md`)

### Fixed

- CLI Reference: added missing `--author` and `--org` flags for `fledge init`
- CLI Reference: added missing `--description`, `--render-patterns`, `--hooks`, `--prompts`, `--yes` flags for `fledge create-template`
- CLI Reference: corrected `--type` to `--branch-type` for `fledge work start` (matching actual flag name)
- CLI Reference: removed non-existent `-y, --yes` flag from `fledge update`
- CLI Reference: updated `fledge lane` to document subcommand structure (`run`, `list`, `init`, `search`, `import`)
- CLI Reference: added short flags (`-t`, `-b`) for `fledge work pr`
- Removed misplaced TUI section from plugins documentation page
- Fixed `--type` → `--branch-type` in develop guide, GitHub integration guide, and quick start
- Updated SUMMARY.md with new documentation pages

## [1.0.0] - 2026-04-20

### Added

- `fledge lane` - composable workflow pipelines with sequential, parallel, and inline steps
- `fledge lane --init` - auto-generate lanes for your project type
- `fledge plugin` - plugin architecture (install, remove, list, search, run) via GitHub repos
- `fledge validate-template` - validate templates for correctness with `--strict` and `--json` output
- `fledge run` zero-config mode - auto-detects project type and runs tasks without `fledge.toml`
- Community lane registry - search and import lanes from GitHub
- `fledge.toml` in the repo root - fledge now dogfoods its own CLI for development workflows
- "Using Fledge with Existing Projects" documentation guide

### Fixed

- **Security**: path traversal in template rendering - malicious templates can no longer write outside the project directory
- CLI reference examples now use correct built-in template names

### Changed

- **Breaking**: `fledge lane` now uses subcommands — `fledge lane run <name>` replaces `fledge lane <name>`, `fledge lane list` replaces `fledge lane --list`, etc.
- Full end-to-end dev lifecycle coverage from scaffold to ship
- Homebrew formula updated to 1.0.0
- Promoted to 1.0.0 - stable API

## [0.8.0] - 2026-04-19

### Added

- `fledge deps` - dependency health check (outdated packages, audit, license scan) for Rust, Node, Python, Go, Ruby
- `fledge metrics` - project stats (lines of code by language, test file ratio, churn analysis)
- `fledge doctor` - environment diagnostics (toolchain versions, missing dependencies, config validation)
- JSON output for all three commands (`--json`)

## [0.7.0] - 2026-04-19

### Added

- `fledge run` — task runner with `fledge.toml` support, `--init` scaffolding, language-aware defaults (Rust, Node, Go, Python, Ruby, Java/Gradle/Maven)
- `fledge checks` — view CI/CD check status for any branch with `--json` output
- `fledge changelog` — generate changelogs from git tags and conventional commits with `--limit`, `--tag`, `--unreleased`, `--json` flags

### Fixed

- Made fledge fully language-agnostic — `.gitignore` template covers all ecosystems, upgrade message links to install docs instead of assuming `cargo install`
- Split Java detection into Gradle/Maven, reinstated `/target/` in `.gitignore`
- Removed invalid `--prompt` flag from Claude CLI calls in `fledge ask`/`fledge review`

## [0.6.0] - 2026-04-19

### Added

- Install script (`curl -fsSL .../install.sh | sh`) — detects OS/arch, downloads the right binary
- Homebrew formula (`brew install CorvidLabs/tap/fledge`)
- Nix flake (`nix run github:CorvidLabs/fledge`)
- `fledge completions --install` — auto-installs shell completions for bash, zsh, or fish
- SHA256 checksums in GitHub releases

## [0.5.0] - 2026-04-19

### Added

- `fledge issues` — list and view GitHub issues with `--state`, `--label`, `--json` filters
- `fledge prs` — list and view GitHub pull requests with `--state`, `--json` filters
- `fledge review` — AI-powered code review of current changes via Claude CLI
- `fledge ask` — ask questions about your codebase via Claude CLI

## [0.4.0] - 2026-04-19

### Added

- `fledge update` — re-apply source template to existing projects with `--dry-run` and `--refresh`
- `fledge spec check` — validate spec-sync specifications against source code
- `fledge spec init` — initialize spec-sync configuration
- `fledge spec new` — scaffold a new spec module
- `fledge work start` — begin a feature branch with naming conventions
- `fledge work pr` — create a PR from the current branch
- `fledge work status` — show current branch and PR status

## [0.3.0] - 2026-04-19

### Added

- `fledge search` — template discovery via GitHub topics
- `fledge publish` — publish templates to GitHub with `fledge-template` topic
- `fledge create-template` — scaffold a new fledge template
- Template versioning and compatibility checks (`min_fledge_version`)
- Version pinning for remote templates with `@ref` syntax
- Additional built-in templates: `python-cli`, `go-cli`, `ts-node`, `static-site`

### Changed

- `fledge config` — full subcommand interface (get/set/unset/add/remove/list/path)
- mdBook documentation site on GitHub Pages

## [0.1.0] - 2026-04-18

### Added

- Core scaffolding engine with Tera template rendering
- 6 built-in templates: `rust-cli`, `ts-bun`, `python-cli`, `go-cli`, `ts-node`, `static-site`
- Remote template support via `owner/repo` GitHub syntax
- Interactive prompts with dialoguer for project configuration
- Hook system with `pre_create` and `post_create` lifecycle hooks
- Hook security: confirmation prompts, `--dry-run`, and `--yes` flags
- Shell completions for bash, zsh, fish, elvish, and PowerShell (`fledge completions`)
- Colored error output with contextual help messages
- Global configuration via `~/.config/fledge/config.toml`
- Optional TUI mode (`--features tui`)
- CI pipeline: tests (3 OS), clippy, fmt, dependency audit, spec-sync validation
- Cross-platform release builds (Linux, macOS x86/ARM, Windows)
