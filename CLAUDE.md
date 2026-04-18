# fledge

Rust CLI for project scaffolding. Uses clap for argument parsing, Tera for template rendering.

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
- `specs/` — spec-sync specifications (source of truth)
- `templates/` — Built-in project templates

## Conventions

- Specs are the source of truth — read before modifying code
- Run `spec-sync check` before committing
- No direct commits to main — use feature branches
