# fledge

Dev lifecycle CLI. One tool for the dev loop, any language. Scaffolding, task running, code review, and the full loop from init to changelog. Built in Rust with clap for CLI parsing, Tera for template rendering.

Six pillars: **scaffold** (templates), **run** (run/lanes/watch), **spec** (spec), **AI** (ai/ask/review), **ship** (work/release/changelog), **extend** (plugins/config/introspect/completions/doctor). Anything outside that surface lives in plugins (`CorvidLabs/fledge-plugin-*`), installed via `fledge plugins install --defaults`. See `AGENTS.md` for the agent-facing tour.

## Build & Test

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

## Architecture

### Entry point & CLI
- `src/main.rs` — CLI entry point, top-level dispatch
- `src/cli.rs` — Clap derive types for all subcommands
- `src/config_cmds.rs` — Config subcommand handlers
- `src/template_cmds.rs` — Template subcommand handlers

### Core commands (single-file modules)
- `src/init.rs` — Project initialization
- `src/run.rs` — Task runner (fledge.toml, language detection)
- `src/watch.rs` — File watcher / re-run on change
- `src/work.rs` — Work branch and PR workflow
- `src/changelog.rs` — Changelog generation from git tags
- `src/review.rs` — AI-powered code review
- `src/ask.rs` — AI-powered codebase Q&A
- `src/ai.rs` — General-purpose AI assistant subcommand
- `src/doctor.rs` — Environment diagnostics
- `src/introspect.rs` — JSON command-tree dump (for agents/automation)

### Multi-file modules (folder modules with `mod.rs`)
- `src/plugin/` — Plugin install/list/run/create/publish/update/remove/validate; lifecycle hooks
- `src/lanes/` — Composable workflow pipelines (execute, community, create, publish, validate, defaults)
- `src/protocol/` — fledge-v1 plugin protocol (detect, exec, metadata, store, UI)
- `src/spec/` — Spec-sync management (commands, parse, validation)
- `src/release/` — Release workflow (bump, changelog, git, version, toml_utils)

### Templates
- `src/templates.rs` — Template loading and Tera rendering
- `src/create_template.rs` — Template scaffolding
- `src/validate.rs` — Template validation
- `src/publish.rs` — Template publishing to GitHub
- `src/search.rs` — Template discovery via GitHub
- `src/remote.rs` — Remote template fetching and caching

### Shared infra
- `src/trust.rs` — Plugin trust-tier classification
- `src/config.rs` — Global config (~/.config/fledge/config.toml)
- `src/prompts.rs` — Interactive prompts (dialoguer)
- `src/spinner.rs` — Terminal spinner UI
- `src/llm.rs` — LLM backend selection
- `src/github.rs` — Shared GitHub API helpers
- `src/versioning.rs` — Version parsing/comparison
- `src/meta.rs` — Project metadata used by introspect
- `src/utils.rs` — Shared utilities (e.g. non-interactive flag)

### Other directories
- `specs/` — spec-sync specifications (source of truth)
- `templates/` — Built-in project templates (embedded via `include_dir!`)
- `docs/` — mdBook documentation site
- `Formula/` — Homebrew formula
- `flake.nix` — Nix flake
- `install.sh` — Curl-pipe installer

## Conventions

- Specs are the source of truth — read before modifying code
- Run `cargo run -- spec check` before committing
- No direct commits to main — use feature branches
- Releases bump `Cargo.toml`, `flake.nix`, and `Formula/fledge.rb` together (see `[release].files` in `fledge.toml`)
