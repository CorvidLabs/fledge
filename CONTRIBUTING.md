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

- `src/cli.rs` defines the clap derive types for every command and flag
- `src/main.rs` dispatches parsed args to the appropriate handler
- Single-file modules cover the simple commands (`src/init.rs`, `src/run.rs`, `src/watch.rs`, `src/work.rs`, `src/changelog.rs`, `src/review.rs`, `src/ask.rs`, `src/ai.rs`, `src/doctor.rs`, `src/introspect.rs`)
- Folder modules with `mod.rs` cover the bigger surfaces — `src/plugin/`, `src/lanes/`, `src/protocol/`, `src/spec/`, `src/release/`
- Shared infra: `src/trust.rs`, `src/config.rs`, `src/prompts.rs`, `src/spinner.rs`, `src/llm.rs`, `src/github.rs`, `src/versioning.rs`, `src/meta.rs`, `src/utils.rs`
- Specs in `specs/<module>/` define how each module should work — read them before modifying code. The `files:` frontmatter list ties each spec to its source files

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

Releases are handled by maintainers using fledge itself:

```bash
# Preview what would happen (no writes, no tag, no push)
fledge release --dry-run patch     # or minor / major / 1.2.3

# Cut the release
fledge release patch               # or minor / major / 1.2.3
```

`fledge release` does the version bump (`Cargo.toml`, `flake.nix`, and `Formula/fledge.rb` together — see `[release].files` in `fledge.toml`), regenerates `CHANGELOG.md` from git history, creates the bump commit, tags `v<version>`, and pushes the tag. The `release.yml` workflow then builds the multi-platform binaries and publishes to crates.io and GitHub Releases. The Homebrew formula's URL/sha256 lines are bumped post-release by `post-release-formula.yml` once the release artifacts (and their `.sha256` sidecars) exist.

For the JSON contract (e.g. for scripting), `fledge release --dry-run --json` and `fledge release --json` emit `{schema_version: 1, action: "release", ...}`.

## Code of Conduct

Be respectful and constructive. We're building tools, not arguments. Harassment, discrimination, and unconstructive behavior aren't tolerated.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
