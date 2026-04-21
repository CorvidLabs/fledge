---
module: tui
version: 2
status: active
files:
  - src/tui.rs

db_tables: []
depends_on:
  - config
  - templates
  - init
---

# TUI

## Purpose

Interactive dashboard for the entire fledge dev lifecycle. Provides a two-panel menu interface to browse and run all fledge commands without memorizing flags or subcommands. Feature-gated behind `--features tui`.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — launch the dashboard TUI |

### Structs & Enums

| Type | Description |
|------|-------------|
| `DashboardApp` | Main dashboard state: categories, focus, screen, input fields, output |
| `DashFocus` | Enum: `Categories`, `Actions` — which panel has focus |
| `DashScreen` | Enum: `Browse`, `Input`, `Output` — current screen mode |
| `ActionKind` | Enum: `Direct` (run immediately), `WithInput` (show form), `TemplateBrowser` (launch nested TUI) |
| `ActionId` | Enum: identifies each input-requiring action for command building |
| `CategoryDef` | Definition of a menu category: name, icon, description, actions |
| `ActionDef` | Definition of a menu action: name, description, kind |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(PathBuf, bool) -> Result<()>` | Main entry — launch dashboard |
| `run_template_browser` | `(PathBuf, bool) -> Result<()>` | Launch the nested template browser |
| `build_categories` | `() -> Vec<CategoryDef>` | Build all 11 menu categories with actions |
| `build_command` | `(ActionId, &[String]) -> Vec<String>` | Convert form input to CLI args |

## Invariants

1. Dashboard has 11 categories: Work, GitHub, Run, Specs, Metrics, Config, Templates, AI, Doctor, Changelog, Plugins
2. Every fledge CLI command is represented except `completions` (not useful in TUI) and `tui` (recursive)
3. Commands run as subprocesses via `std::env::current_exe()` — no internal module coupling
4. `NO_COLOR=1` is set on subprocesses; ANSI escape codes are stripped from output
5. Input forms validate required fields before submission; empty optional fields use defaults
6. Template Browser launches as a nested full-screen TUI, restoring the dashboard on exit
7. Output panel is scrollable with `↑↓`, `PgUp`/`PgDn`, `g`/`G`
8. `Ctrl+C` always exits cleanly, restoring terminal state
9. Terminal raw mode and alternate screen are properly entered/exited, including on error paths
10. The `tui` feature gate controls all TUI code via `#[cfg(feature = "tui")]`

## Behavioral Examples

```
$ fledge tui

  ┌─ Categories ──────┬─ Work ──────────────────────────┐
  │ 👉 🔀 Work          │ 👉 Start Branch     Create a new │
  │   🐙 GitHub        │   Create PR        Open a pull  │
  │   ▶️ Run           │   Status           Show current │
  │   📋 Specs         │                                  │
  │   📊 Metrics       │                                  │
  │   ⚙️ Config        │                                  │
  │   📦 Templates     │                                  │
  │   🤖 AI            │                                  │
  │   🩺 Doctor        │                                  │
  │   📝 Changelog     │                                  │
  │   🔌 Plugins       │                                  │
  └───────────────────┴──────────────────────────────────┘
  ↑↓ navigate  ⏎/→ open category  q quit
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Terminal setup fails | Cannot enter raw mode | anyhow error |
| Command not found | `current_exe()` fails | Error shown in output panel |
| Subprocess fails | Command returns non-zero | Exit code shown in output panel |
| Required field empty | User submits form without filling required field | Error popup shown |

## Dependencies

- `ratatui` for TUI rendering (optional, feature-gated)
- `crossterm` for terminal control (optional, feature-gated)
- `crate::config::Config` for detecting project config
- `crate::templates` for template browser integration
- `crate::init` for git init in template scaffolding

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 2 | 2026-04-20 | Update behavioral examples to use emojis instead of ASCII/Unicode symbols |
| 1 | 2026-04-20 | Initial spec — full dashboard with 11 categories, 40+ actions |
