# fledge

Dev-lifecycle CLI — scaffolding, task running, code review, and the full dev loop from init to changelog. Built in Rust with clap for CLI parsing, Tera for template rendering.

## Build & Test

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

## Architecture

- `src/main.rs` — CLI entry point (clap derive)
- `src/init.rs` — Project initialization logic
- `src/templates.rs` — Template loading and Tera rendering
- `src/config.rs` — Global config (~/.config/fledge/config.toml)
- `src/prompts.rs` — Interactive prompts (dialoguer)
- `src/run.rs` — Task runner (fledge.toml, language detection)
- `src/changelog.rs` — Changelog generation from git tags
- `src/checks.rs` — CI/CD status viewer
- `src/spec.rs` — Spec-sync management
- `src/work.rs` — Work branch and PR workflow (flexible branch types, configurable format)
- `src/issues.rs` — GitHub issues
- `src/prs.rs` — GitHub pull requests
- `src/review.rs` — AI-powered code review
- `src/ask.rs` — AI-powered codebase Q&A
- `src/search.rs` — Template discovery via GitHub
- `src/publish.rs` — Template publishing to GitHub
- `src/release.rs` — Release workflow (bump, changelog, tag, push)
- `src/update.rs` — Template re-application
- `src/create_template.rs` — Template scaffolding
- `src/versioning.rs` — Version management
- `src/github.rs` — Shared GitHub API helpers
- `src/remote.rs` — Remote template fetching and caching
- `src/lanes.rs` — Composable workflow pipelines
- `src/deps.rs` — Cross-language dependency health checker
- `src/metrics.rs` — Code metrics (LOC, churn, test ratio)
- `src/plugin.rs` — Plugin system for community extensions
- `src/validate.rs` — Template validation
- `src/tui.rs` — Interactive template browser (feature-gated)
- `src/doctor.rs` — Environment diagnostics
- `specs/` — spec-sync specifications (source of truth)
- `templates/` — Built-in project templates
- `docs/` — mdBook documentation site

## Conventions

- Specs are the source of truth — read before modifying code
- Run `cargo run -- spec check` before committing
- No direct commits to main — use feature branches
