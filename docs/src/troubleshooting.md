# Troubleshooting

Common issues and how to fix them.

## "command not found: fledge"

fledge isn't on your `PATH`. If you installed via `cargo install`:

```bash
# Make sure ~/.cargo/bin is on your PATH
export PATH="$HOME/.cargo/bin:$PATH"
```

Add that line to your `.bashrc`, `.zshrc`, or shell profile. Then restart your terminal or run `source ~/.zshrc`.

## GitHub commands fail with "no token"

Commands that talk to GitHub (`issues`, `prs`, `checks`, `work pr`, `review`, `templates search`) need a GitHub personal access token.

```bash
# Easiest: use the GitHub CLI (zero config)
gh auth login

# Or set it as an environment variable
export GITHUB_TOKEN="ghp_..."

# Or store it in fledge config
fledge config set github.token "ghp_..."
```

The token needs `repo` scope for private repos, or just `public_repo` for public ones. If you have `gh` installed and authenticated, fledge uses it automatically as a fallback.

## "no template found" when running init

Check what's available:

```bash
fledge templates list
```

If you expect remote templates, make sure the repo is configured:

```bash
fledge config add templates.repos "CorvidLabs/fledge-templates"
```

Or use the full path:

```bash
fledge templates init my-app --template CorvidLabs/fledge-templates/python-api
```

## Tasks not detected (fledge run shows nothing)

fledge auto-detects your project type from marker files. If it doesn't find anything:

1. Make sure you're in the project root (where `Cargo.toml`, `package.json`, etc. live)
2. Run `fledge doctor` to check your environment
3. If auto-detection doesn't fit, create a `fledge.toml` manually:

```bash
fledge run --init
```

## Template hooks won't run

Remote template hooks require confirmation for security. If you trust the template:

```bash
fledge templates init my-app --template user/repo --yes
```

If hooks fail after confirmation, check that the required tools are installed (e.g., `npm`, `pip`, `cargo fmt`).

## "rate limit exceeded" from GitHub

GitHub's API has rate limits. Authenticated requests get 5,000/hour vs 60/hour for unauthenticated.

```bash
# Set your token to get higher limits
export GITHUB_TOKEN="ghp_..."
```

## Cached templates are stale

Templates from remote repos are cached locally. To force a fresh download:

```bash
fledge templates init my-app --template user/repo --refresh
```

## fledge review / fledge ask not working

These commands use the Claude CLI. Make sure it's installed and authenticated:

```bash
claude --version
```

If Claude isn't installed, see [Claude CLI docs](https://docs.anthropic.com/en/docs/claude-code/overview) for setup instructions.

## Clippy or fmt warnings in CI

Run these locally before pushing:

```bash
cargo clippy -- -D warnings
cargo fmt --check
```

If clippy finds issues, fix them. If formatting is off, run `cargo fmt` (without `--check`) to auto-fix.

## Still stuck?

1. Run `fledge doctor` for a full environment check
2. Check [GitHub Issues](https://github.com/CorvidLabs/fledge/issues) for known problems
3. Open a new issue with:
   - Your OS and fledge version (`fledge --version`)
   - The command you ran
   - The full error output
