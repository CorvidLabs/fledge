---
spec: plugin.spec.md
---

## Context

The plugin system lets the community extend fledge without forking. It follows the git-style convention where `fledge deploy` resolves to a `fledge-deploy` binary. This keeps the core CLI lean while enabling ecosystem growth through installable extensions.

## Related Modules

- `config` — plugin directory paths under the platform config directory (`dirs::config_dir()`)
- `github` — GitHub API for search and publish operations

## Design Decisions

- Git-style binary resolution (`fledge-<name>` on PATH) — familiar to git users, works without installation
- Plugin manifest (`plugin.toml`) — explicit declaration of commands and hooks
- Symlinks into `plugins/bin/` — centralized lookup without modifying PATH
- GitHub topic convention (`fledge-plugin`) — same discovery pattern as templates
- Local path installs live-link by default — plugin authors can dogfood scaffolded plugins without publishing or reinstalling after every edit
- Generic git URL installs clone as-is — GitHub shorthand remains a convenience, not a hard dependency
