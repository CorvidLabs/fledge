---
spec: deps.spec.md
---

## Tasks

- [x] Write deps spec
- [x] Implement DepsOptions struct and run() entry point
- [x] Implement parse_dependencies() dispatcher for all ecosystems
- [x] Implement Cargo.lock parser (name/version pairs from TOML-like format)
- [x] Implement package-lock.json parser (v3 packages map)
- [x] Implement yarn.lock parser (version lines after package headers)
- [x] Implement go.sum parser (deduplicated module/version pairs)
- [x] Implement requirements.txt parser (==, >=, and bare package names)
- [x] Implement Pipfile.lock parser (JSON default section)
- [x] Implement poetry.lock parser (TOML-like package blocks)
- [x] Implement Gemfile.lock parser (specs section, 4-space indented entries)
- [x] Implement --outdated, --audit, --licenses ecosystem command dispatch
- [x] Implement --json output with DepsReport struct
- [x] Handle Java (gradle/maven) with unsupported message and --outdated/--audit guidance
- [x] Handle pnpm-lock.yaml with error and direct pnpm command suggestion
- [x] Wire DepsAction subcommand into main.rs
- [x] Add unit tests for all lock file parsers (10 tests)
- [x] Add unquote helper test
- [x] Add generic project detection test
- [x] Register spec and verify with cargo test, clippy, fmt, spec-check

## Gaps

- No pnpm-lock.yaml parsing (YAML dependency would be needed)
- No transitive dependency tree display
- Java lock file formats not parsed
