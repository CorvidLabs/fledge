# Contributing to fledge

Thanks for your interest in contributing to fledge! Whether it's a bug fix, new feature, documentation improvement, or template — contributions are welcome.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- Git
- A GitHub account (for PRs and issue tracking)

### Clone and Build

```bash
git clone https://github.com/CorvidLabs/fledge.git
cd fledge
cargo build
cargo test
```

Once built, install locally and use fledge itself for development (we dogfood our own CLI):

```bash
cargo install --path .
fledge run build
fledge run test
```

See `fledge.toml` at the repo root for all available tasks and lanes.

## Development Workflow

### 1. Find or Create an Issue

Check [existing issues](https://github.com/CorvidLabs/fledge/issues) first. If your change is non-trivial, open an issue to discuss the approach before writing code.

### 2. Create a Branch

```bash
# Use fledge for branch management
fledge work start my-feature
```

Branch naming convention: `{type}/{description}` where type is `feat`, `fix`, `chore`, `docs`, `refactor`, or `hotfix`.

### 3. Make Your Changes

- Read the relevant spec in `specs/` before modifying a module
- Follow the existing code style (Rust idioms, `anyhow` for errors)
- Add tests for new functionality
- Update documentation if you change CLI behavior

### 4. Verify

```bash
# Run the full pre-commit lane (fmt, lint, test, spec check)
fledge lanes run pre-commit

# Or check individual steps
fledge run fmt               # formatting is correct
fledge run lint              # no lint warnings
fledge run test              # all tests pass
fledge spec check            # specs are in sync
```

All checks in the `pre-commit` lane must pass before submitting a PR.

### 5. Submit a Pull Request

```bash
# Push your branch first
fledge work push

# Open a PR (requires fledge-plugin-github)
fledge github prs create --title "Add my feature"
# or infer title and body from your commits:
fledge github prs create --fill
```

In your PR description:
- Explain **what** changed and **why**
- Reference any related issues (`Fixes #123`)
- Note any breaking changes

## What to Contribute

### Good First Issues

Look for issues labeled [`good first issue`](https://github.com/CorvidLabs/fledge/labels/good%20first%20issue). These are scoped, well-defined tasks suitable for newcomers.

### Templates

Create new templates and publish them with `fledge templates publish`! See the [Template Authoring Guide](https://corvidlabs.github.io/fledge/template-authoring.html) for the full format.

### Plugins

Build plugins that extend fledge with new commands. See the [Plugins Guide](https://corvidlabs.github.io/fledge/plugins.html) for the plugin format.

### Lanes

Share workflow pipelines via the community lane registry. Push a repo with a `fledge.toml` and add the `fledge-lane` topic.

### Documentation

Documentation lives in `docs/src/` and is built with [mdBook](https://rust-lang.github.io/mdBook/). To preview locally:

```bash
# Install mdBook if you don't have it
cargo install mdbook

# Serve locally
fledge run docs-serve
```

## Code Guidelines

### Architecture

- Each CLI command lives in its own module (`src/<command>.rs`)
- `src/main.rs` handles argument parsing and dispatch
- Shared GitHub helpers are in `src/github.rs`
- Specs in `specs/` define how each module should work — read them before modifying code

### Error Handling

- Use `anyhow::Result` for all public functions
- Use `anyhow::bail!` for early returns with error messages
- Error messages should be user-friendly and actionable

### Testing

- Unit tests go in `#[cfg(test)] mod tests` at the bottom of each module
- Integration tests go in `tests/`
- Test both the happy path and error cases
- Use `tempfile` for tests that write to disk

### Style

- Run `fledge run fmt-fix` before committing
- Run `fledge run lint` and fix all warnings
- No `unsafe` code without discussion
- Prefer standard library types over external crates when practical

## Specs

Every module has a spec in `specs/<module>/`. The spec is the source of truth for what the module does. Before modifying a module:

1. Read its spec
2. If your change alters behavior, update the spec first
3. Run `fledge spec check` to verify alignment

### Dependencies

Dependency commands ship in `fledge-plugin-deps` (part of the default plugin
set). Install once, then check dependency health:

```bash
fledge plugins install --defaults   # one-time, gets github/deps/metrics
fledge deps                         # report dependency status
fledge deps --outdated              # show outdated entries
```

## Release Process

Releases are handled by maintainers. The process:

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md` with new entries
3. Tag with `git tag v<version>`
4. Push tag — CI builds and publishes to crates.io, Homebrew, and GitHub Releases

## Code of Conduct

Be respectful and constructive. We're building tools, not arguments. Harassment, discrimination, and unconstructive behavior aren't tolerated.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
