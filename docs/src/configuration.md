# Configuration

Configure fledge to customize defaults and add custom template repositories.

## Quick Setup with Presets

Initialize config with a preset for fast onboarding:

```bash
# CorvidLabs preset — sets author, org, license, and template repo
fledge config init --preset corvidlabs

# Default config with MIT license
fledge config init
```

## Config File Location

fledge reads configuration from:

```
~/.config/fledge/config.toml
```

## Configuration Sections

### defaults

Set default values for project creation:

```toml
[defaults]
author = "Your Name"
github_org = "YourOrg"
license = "MIT"
```

| Key | Description | Default |
|-----|-------------|---------|
| `author` | Default author name | falls back to `git config user.name` |
| `github_org` | Default GitHub organization | `CorvidLabs` |
| `license` | Default license for new projects | `MIT` |

If `author` is not set, fledge falls back to your git config:

```bash
git config user.name
```

### templates

Configure template locations and repositories:

```toml
[templates]
paths = ["~/my-templates", "~/work/templates"]
repos = ["CorvidLabs/fledge-templates", "myorg/templates"]
```

| Key | Description |
|-----|-------------|
| `paths` | Local directories containing template files (relative to home) |
| `repos` | GitHub repositories to search for templates (`owner/repo` format) |

### github

Configure GitHub access for private template repositories:

```toml
[github]
token = "ghp_..."
```

| Key | Description |
|-----|-------------|
| `token` | GitHub personal access token for private repos |

**Note:** fledge checks GitHub token in order:
1. `FLEDGE_GITHUB_TOKEN` environment variable
2. `GITHUB_TOKEN` environment variable
3. `token` in config file

## Complete Example Config

```toml
[defaults]
author = "Leif"
github_org = "CorvidLabs"
license = "MIT"

[templates]
paths = ["~/.fledge/templates", "~/projects/templates"]
repos = ["CorvidLabs/fledge-templates", "my-org/my-templates"]

[github]
token = "ghp_1234567890abcdefghijklmnopqrstuvwxyz"
```

## Environment Variables

fledge respects the following environment variables:

| Variable | Purpose |
|----------|---------|
| `FLEDGE_GITHUB_TOKEN` | GitHub token (overrides config) |
| `GITHUB_TOKEN` | GitHub token (fallback) |

Example:

```bash
export FLEDGE_GITHUB_TOKEN="ghp_..."
fledge init my-project --template private-org/private-template
```

## Defaults Behavior

When creating a project, fledge uses this priority for defaults:

1. Command-line arguments (highest priority)
2. Config file settings
3. Git config (for author)
4. Built-in defaults (lowest priority)

For example, if you set `author = "Leif"` in your config, fledge will use that unless you provide a different author when prompted.
