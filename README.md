# fledge

Get your projects ready to fly.

A project scaffolding CLI that creates new repositories from templates with [CorvidLabs](https://github.com/CorvidLabs) conventions baked in.

## Usage

```bash
# Create a new Rust CLI project
fledge init my-project --template rust-cli

# List available templates
fledge list
```

## Templates

| Template | Description |
|----------|-------------|
| `rust-cli` | Rust CLI application with clap, CI, and release automation |
| `rust-lib` | Rust library crate with docs and publishing workflow |
| `swift-pkg` | Swift package with Package.swift, CI, and coding conventions |
| `ts-bun` | TypeScript project with Bun runtime |
| `angular-app` | Angular application with mobile-first setup |

## Install

```bash
cargo install fledge
```

## License

MIT
