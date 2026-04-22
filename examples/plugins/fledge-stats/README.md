# fledge-stats

Example plugin demonstrating the **fledge-v1 protocol**. Shows project statistics including file counts, lines of code, git history, and change tracking between runs.

## Protocol features used

- `log` — status messages during analysis
- `progress` — 5-step progress bar
- `exec` — runs shell commands (find, git, wc)
- `metadata` — fetches git_status and git_tags
- `store` / `load` — persists stats between runs for delta tracking
- `output` — renders a formatted stats card

## Build & install

```bash
# From the fledge repo root:
fledge plugin install ./examples/plugins/fledge-stats

# Or build manually:
cd examples/plugins/fledge-stats
cargo build --release
```

## Usage

```bash
fledge stats
```

## Example output

```
  ╭─────────────────────────────────────────╮
  │  fledge           stats                 │
  ├─────────────────────────────────────────┤
  │  Language    rust                        │
  │  Files          142 (+3)                │
  │  LOC          12847 (+215)              │
  │  Commits        287                     │
  │  Test files      12 (8%)               │
  ├─────────────────────────────────────────┤
  │  Rust               38                  │
  │  Markdown           22                  │
  │  TOML                8                  │
  │  Other              74                  │
  ├─────────────────────────────────────────┤
  │  Top contributors                       │
  │    42  Corvid Agent                     │
  │    18  Leif                             │
  ├─────────────────────────────────────────┤
  │  Recent commits                         │
  │    abc1234 fix: plugin protocol test    │
  │    def5678 feat: add lanes pipeline     │
  ╰─────────────────────────────────────────╯
```

## Zero dependencies

This plugin uses only the Rust standard library — no external crates. JSON is handled via simple string helpers, making it a minimal reference implementation of the fledge-v1 protocol.
