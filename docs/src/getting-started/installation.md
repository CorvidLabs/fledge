# Installation

## From crates.io

The easiest way to install fledge is from crates.io:

```bash
cargo install fledge
```

## With TUI Support

fledge includes an optional interactive terminal UI for browsing and selecting templates. To enable it, install with the `tui` feature:

```bash
cargo install fledge --features tui
```

## Homebrew

```bash
brew install CorvidLabs/tap/fledge
```

## Install Script

Download and install the latest release automatically:

```bash
curl -fsSL https://raw.githubusercontent.com/CorvidLabs/fledge/main/install.sh | sh
```

The script detects your OS and architecture and downloads the correct binary.

## Nix

```bash
nix run github:CorvidLabs/fledge
```

Or add to your flake inputs for a permanent installation.

## From Source

If you prefer to build from source, clone the repository and install:

```bash
git clone https://github.com/CorvidLabs/fledge.git
cd fledge && cargo install --path .
```

## Shell Completions

After installation, set up shell completions for tab-completion support:

```bash
# Auto-install for your current shell
fledge completions --install

# Or generate manually
fledge completions bash >> ~/.bashrc
fledge completions zsh > ~/.zfunc/_fledge
fledge completions fish > ~/.config/fish/completions/fledge.fish
```

## Verify Installation

After installation, verify fledge works:

```bash
fledge --version
fledge list
```

You should see fledge's version and a list of available built-in templates.
