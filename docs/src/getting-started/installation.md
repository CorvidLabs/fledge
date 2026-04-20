# Installation

## From crates.io

Fastest way to get going:

```bash
cargo install fledge
```

## With TUI Support

Want an interactive template browser? Install with the `tui` feature:

```bash
cargo install fledge --features tui
```

## Homebrew

```bash
brew install CorvidLabs/tap/fledge
```

## Install Script

Detects your OS and arch, grabs the right binary:

```bash
curl -fsSL https://raw.githubusercontent.com/CorvidLabs/fledge/main/install.sh | sh
```

## Nix

```bash
nix run github:CorvidLabs/fledge
```

Or add it to your flake inputs.

## From Source

```bash
git clone https://github.com/CorvidLabs/fledge.git
cd fledge && cargo install --path .
```

## Shell Completions

Tab completion makes everything better:

```bash
# Auto-install for your shell
fledge completions --install

# Or do it manually
fledge completions bash >> ~/.bashrc
fledge completions zsh > ~/.zfunc/_fledge
fledge completions fish > ~/.config/fish/completions/fledge.fish
```

## Verify It Works

```bash
fledge --version
fledge list
```

You should see the version and a list of built-in templates. If you do, you're good.
