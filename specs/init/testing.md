---
spec: init.spec.md
---

## Automated Testing

| Test File | Type | What It Covers |
|-----------|------|----------------|
| `src/init.rs` (inline) | Unit | Template resolution by name, unknown template errors, error lists available templates, git init creates repo + initial commit, hook execution, hook failure handling, remote hooks with --yes |

## Manual Testing

- [x] `fledge init my-app` with interactive template selector
- [x] `fledge init my-app --template rust-cli` creates a Rust CLI project
- [x] `fledge init my-app --template owner/repo` fetches and scaffolds from GitHub
- [x] `fledge init my-app --dry-run` shows preview without creating files
- [x] `fledge init my-app --no-git` skips git initialization
- [x] `fledge init my-app --no-install` skips post-create hooks
- [x] `fledge init my-app --yes --template owner/repo` runs remote hooks without prompt
- [x] `fledge init my-app --refresh --template owner/repo` re-fetches cached remote
- [x] Existing directory produces clear error message
- [x] Template with missing variables produces clear rendering error

## Edge Cases & Boundary Conditions

| Scenario | Expected Behavior |
|----------|-------------------|
| Target directory already exists | Error: "Directory already exists. Choose a different name or remove it first." |
| No templates found (empty dirs, no builtins) | Error: "No templates found" |
| Template name not found | Error listing available templates |
| Remote ref with subpath (`owner/repo/templates/rust`) | Resolves to subdirectory within cloned repo |
| Remote repo with multiple templates | Interactive selector from discovered set |
| Hook command fails (non-zero exit) | Error with exit code and failed command |
| Empty hooks list | No-op, no output |
| Git not installed | `git init` errors with context |
| CI environment with no git identity | Auto-configures `fledge` / `fledge@localhost` |
| `.tera` files in template | Rendered through Tera and extension stripped |
