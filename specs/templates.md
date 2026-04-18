# Templates Specification

## Overview

Fledge ships with built-in templates for common CorvidLabs project types. Templates are directories containing project files with Tera template syntax for variable substitution.

## Built-in Templates

### `rust-cli`

Rust CLI application modeled after spec-sync's structure.

**Generates:**
- `Cargo.toml` — with clap, anyhow, serde
- `src/main.rs` — clap-based CLI skeleton
- `.github/workflows/ci.yml` — lint, test, build matrix (Linux/macOS/Windows)
- `.github/workflows/release.yml` — tag-triggered binary releases
- `CLAUDE.md` — AI assistant conventions
- `specs/` — spec-sync directory with initial spec
- `.gitignore` — Rust defaults
- `LICENSE` — MIT
- `README.md` — project overview

### `rust-lib`

Rust library crate for publishing to crates.io.

**Generates:**
- `Cargo.toml` — library crate with docs metadata
- `src/lib.rs` — module skeleton with doc comments
- `.github/workflows/ci.yml` — lint, test, docs build
- `.github/workflows/publish.yml` — crates.io publish on tag
- `CLAUDE.md`, `specs/`, `.gitignore`, `LICENSE`, `README.md`

### `swift-pkg`

Swift package following CorvidLabs conventions.

**Generates:**
- `Package.swift` — Swift package manifest
- `Sources/{{ project_name_pascal }}/` — source directory
- `Tests/{{ project_name_pascal }}Tests/` — test directory
- `.github/workflows/ci.yml` — Swift CI
- `CLAUDE.md`, `specs/`, `.gitignore`, `LICENSE`, `README.md`

### `ts-bun`

TypeScript project with Bun runtime.

**Generates:**
- `package.json` — Bun-compatible
- `tsconfig.json` — strict TypeScript config
- `src/index.ts` — entry point
- `biome.json` — linter/formatter config
- `.github/workflows/ci.yml` — Bun CI
- `CLAUDE.md`, `specs/`, `.gitignore`, `LICENSE`, `README.md`

### `angular-app`

Angular application with mobile-first setup.

**Generates:**
- Angular CLI project structure
- `biome.json` — linter config
- `.github/workflows/ci.yml` — Angular CI
- `CLAUDE.md`, `specs/`, `.gitignore`, `LICENSE`, `README.md`

## Template Authoring

### Directory Layout

```
templates/
  rust-cli/
    template.toml          # Template manifest
    Cargo.toml.tera        # Files ending in .tera are rendered, extension stripped
    src/
      main.rs.tera
    .github/
      workflows/
        ci.yml.tera
    .gitignore              # Non-.tera files copied as-is (unless in render globs)
    LICENSE.tera
    README.md.tera
```

### Tera Rendering

Files matching `render` globs in `template.toml` are processed through Tera. All template variables from the CLI spec are available.

**Example `Cargo.toml.tera`:**
```toml
[package]
name = "{{ project_name }}"
version = "0.1.0"
edition = "2024"
description = "{{ description }}"
license = "{{ license }}"
repository = "https://github.com/{{ github_org }}/{{ project_name }}"
```

### Custom Prompts

Templates can define additional prompts in `template.toml`:

```toml
[prompts]
description = { message = "Project description", default = "A new project" }
binary_name = { message = "Binary name", default = "{{ project_name }}" }
```

## Source Files

- `src/templates.rs` — Template discovery, loading, and rendering
- `templates/` — Built-in template directories
