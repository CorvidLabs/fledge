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

## From Source

If you prefer to build from source, clone the repository and install:

```bash
git clone https://github.com/CorvidLabs/fledge.git
cd fledge && cargo install --path .
```

## Verify Installation

After installation, verify fledge works:

```bash
fledge --version
fledge list
```

You should see fledge's version and a list of available built-in templates.
