# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.11.1] - 2026-04-23

### Added

- `fledge run --json` flag for structured JSON output — improves AI agent usability (#228)

## [0.11.0] - 2026-04-23

### Added

- Plugin trust tiers and `fledge plugin audit` command — verify plugin provenance (#220)
- Trust tier badges for templates and lanes — warnings for unverified sources (#221)
- Non-TTY support for AI agents and CI environments — all interactive prompts gracefully degrade (#222)
- `uv.lock` support in `fledge deps` for Python projects (#223)
- Use cases page and enhanced review documentation (#219)

### Fixed

- Proper TOML parsing for `uv.lock` instead of fragile line-based parsing (#224)
- Plural command names (`lanes`, `plugins`) used consistently across all docs and specs (#216, #217, #218)

### Changed

- CONTRIBUTING.md fully dogfoods fledge — uses `fledge run`, `fledge lanes`, and `fledge work` instead of raw cargo commands (#223, #225, #226)

## [0.10.0] - 2026-04-22

### Added

- `fledge lanes create` / `fledge lanes validate` — scaffold and validate lane definitions (#203)
- `fledge plugin create` / `fledge plugin validate` — scaffold and validate plugin manifests (#203)
- Plugin protocol v1 — full JSON-lines IPC with capability manifest, structured logging, and lifecycle events (#178, #179, #196, #197)
- Plugin and lane publishing — `fledge plugin publish` and `fledge lanes publish` (#176, #177)
- GitHub CLI (`gh`) token fallback — fledge uses `gh auth token` when no `GITHUB_TOKEN` is set (#201)
- Release workflow hardening — duplicate tag pre-check prevents overwriting existing releases (#214)
- 10 new release tests covering gemspec, setup.cfg, pom.xml bumping, `--no-tag` flag, and edge cases (#214)
- Cross-platform plugin protocol tests (#180)

### Fixed

- Plugin state.json locking, env filtering, key validation, and exec timeout cap (#188)
- Plugin protocol security hardening — input validation, output size limits (#187)
- Plugin audit findings from security review (#195)
- TOML serialization crashes, UTF-8 truncation panics, and remote ref parsing failures (#200)
- 6 crash and security findings from codebase audit (#199)
- Security review findings — input sanitization and error handling (#198)
- Error message config keys now reference correct `fledge config` commands (#213)
- Spec frontmatter documented as YAML (not TOML) with correct field types (#212)
- Audit round 2 — doc/spec inaccuracies, missing CLI flags, wrong command names (#211)
- Documentation and spec gaps filled (#210)
- Infra and publishing audit bug fixes (#209)
- Dev loop audit bug fixes (#208)
- Templates audit bug fixes (#206)
- Doc inaccuracies in language defaults, lane docs, and CLI reference (#214)

### Changed

- Removed TUI module — will be reimplemented as a plugin (#204)
- CLI documentation updated to match current subcommand structure (#202)

## [0.9.1] - 2026-04-21

### Fixed

- Release workflow: use `cp` instead of `mv` in checksum step to fix artifact packaging with `download-artifact@v4` (#173)

## [0.9.0] - 2026-04-21

### Added

- `fledge lane` — composable workflow pipelines with sequential, parallel, and inline steps
- `fledge lane --init` — auto-generate lanes for your project type
- `fledge plugin` — plugin architecture (install, remove, list, search, run) via GitHub repos
- `fledge validate-template` — validate templates for correctness with `--strict` and `--json` output
- `fledge run` zero-config mode — auto-detects project type and runs tasks without `fledge.toml`
- Community lane registry — search and import lanes from GitHub
- `fledge.toml` in the repo root — fledge now dogfoods its own CLI for development workflows
- "Using Fledge with Existing Projects" documentation guide
- Step timing for lanes — each step shows elapsed time, lane summary shows total time
- Plugin lifecycle hooks — `pre_init`, `post_work_start`, `pre_pr` fire at fledge lifecycle events
- Parallel lane steps accept inline commands alongside task references
- SECURITY.md — vulnerability reporting policy and security model documentation
- CONTRIBUTING.md — development setup, workflow, code guidelines, and contribution process
- Doctor guide page in documentation (`docs/src/doctor.md`)
- Troubleshooting page in documentation (`docs/src/troubleshooting.md`)

### Fixed

- **Security**: path traversal in template rendering — malicious templates can no longer write outside the project directory
- **Security**: GitHub token no longer leaked via process table — auth passed via environment variables instead of CLI args
- **Security**: config files now enforce 0600 permissions on both new and pre-existing files
- **Security**: plugin binary path traversal hardened — both plugin dir and binary path are canonicalized before comparison
- **Security**: plugin command names validated to prevent symlink injection (rejects `/`, `\`, `.`, `-` prefix)
- **Security**: plugin install now shows security warning and requires confirmation (use `--force` to skip in CI)
- **Security**: post-create hooks always require confirmation regardless of template source (use `--yes` to skip in CI)
- **Security**: template requirement checker rejects tool names starting with `-` to prevent `which` false positives
- **Security**: replaced hand-rolled base64 with audited `base64` crate
- CLI reference examples now use correct built-in template names
- CLI Reference: added missing `--author` and `--org` flags for `fledge init`
- CLI Reference: added missing `--description`, `--render-patterns`, `--hooks`, `--prompts`, `--yes` flags for `fledge create-template`
- CLI Reference: corrected `--type` to `--branch-type` for `fledge work start` (matching actual flag name)
- CLI Reference: removed non-existent `-y, --yes` flag from `fledge update`
- CLI Reference: updated `fledge lane` to document subcommand structure (`run`, `list`, `init`, `search`, `import`)
- CLI Reference: added short flags (`-t`, `-b`) for `fledge work pr`
- Removed misplaced TUI section from plugins documentation page
- Fixed `--type` → `--branch-type` in develop guide, GitHub integration guide, and quick start
- Updated SUMMARY.md with new documentation pages

### Changed

- **Breaking**: `fledge lane` now uses subcommands — `fledge lane run <name>` replaces `fledge lane <name>`, `fledge lane list` replaces `fledge lane --list`, etc.
- **Breaking**: post-create hooks now always prompt for confirmation (pass `--yes` to auto-approve for CI/scripts)
- **Breaking**: `fledge plugin install` now requires confirmation before cloning (pass `--force` to skip for CI/scripts)
- **Breaking**: hook execution uses direct process invocation instead of shell — pipes, redirects, and shell expansions in hook commands are no longer supported; use a wrapper script instead
- Full end-to-end dev lifecycle coverage from scaffold to ship
- Homebrew formula updated to 0.9.0
- CLI commands reorganized: `fledge templates`, `fledge lanes`, `fledge plugins` with subcommands

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
