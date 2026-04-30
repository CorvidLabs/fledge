# Configuration

## Quick Setup

Use a preset to get started fast:

```bash
# CorvidLabs preset - sets author, org, license, template repo
fledge config init --preset corvidlabs

# Default config
fledge config init
```

## Config File

Lives at:

```text
~/.config/fledge/config.toml
```

## Sections

### [defaults]

Default values for new projects:

```toml
[defaults]
author = "Your Name"
github_org = "YourOrg"
license = "MIT"
```

| Key | What it does | Fallback |
|-----|-------------|----------|
| `author` | Default author name | `git config user.name` |
| `github_org` | Default GitHub org | Prompted |
| `license` | Default license | `MIT` |

### [templates]

Where to find templates:

```toml
[templates]
paths = ["~/my-templates", "~/work/templates"]
repos = ["CorvidLabs/fledge-templates", "myorg/templates"]
```

| Key | What it does |
|-----|-------------|
| `paths` | Local directories with templates |
| `repos` | GitHub repos to pull templates from (`owner/repo`) |

### [ai]

AI provider and model settings. Written by `fledge ai use` or `fledge config set`/`edit`:

```toml
[ai]
provider = "ollama"             # "claude" or "ollama"

[ai.claude]
model = "sonnet"               # model name passed to claude CLI

[ai.ollama]
host = "http://localhost:11434" # Ollama API endpoint (always normalized to include scheme)
model = "llama3.2:latest"       # use `fledge ai models --provider ollama` to list available models
api_key = "sk-..."             # for Ollama Cloud / authenticated endpoints
timeout_seconds = 600          # request timeout (default: 600)
```

> **Tip:** Run `fledge ai models --provider ollama` or `fledge ai models --provider claude` to see available models. Use `fledge ai use` for an interactive picker.

| Key | What it does | Default |
|-----|-------------|---------|
| `ai.provider` | Active LLM backend | `claude` |
| `ai.claude.model` | Model name for Claude CLI | Claude CLI default |
| `ai.ollama.host` | Ollama API endpoint URL | `http://localhost:11434` |
| `ai.ollama.model` | Ollama model name | first available |
| `ai.ollama.api_key` | Bearer token for authenticated endpoints | (none) |
| `ai.ollama.timeout_seconds` | Request timeout in seconds | `600` |

### [github]

```toml
[github]
token = "ghp_..."
```

Token priority:
1. `FLEDGE_GITHUB_TOKEN` env var
2. `GITHUB_TOKEN` env var
3. Config file
4. `gh auth token` (GitHub CLI fallback)

**Required token scopes:**

| Feature | Scopes needed |
|---------|--------------|
| Issues, PRs, CI checks | `repo` (or `public_repo` for public repos only) |
| Create PRs, push branches | `repo` |
| Search templates/plugins | `public_repo` |
| Publish templates | `repo`, `delete_repo` (if republishing) |

A classic token with `repo` covers everything. For fine-grained tokens, grant Read/Write on Contents, Pull Requests, and Issues for each repo you work with.

## Full Example

```toml
[defaults]
author = "Leif"
github_org = "CorvidLabs"
license = "MIT"

[templates]
paths = ["~/.fledge/templates", "~/projects/templates"]
repos = ["CorvidLabs/fledge-templates", "my-org/my-templates"]

[github]
token = "ghp_..."

[ai]
provider = "claude"

[ai.claude]
model = "sonnet"

[ai.ollama]
host = "http://localhost:11434"
model = "llama3.2:latest"
timeout_seconds = 600
```

## Environment Variables

| Variable | What it does |
|----------|-------------|
| `FLEDGE_GITHUB_TOKEN` | GitHub token (highest priority) |
| `GITHUB_TOKEN` | GitHub token (fallback after FLEDGE_GITHUB_TOKEN) |
| `FLEDGE_AI_PROVIDER` | AI provider override (`claude` or `ollama`) |
| `FLEDGE_AI_MODEL` | AI model override |
| `FLEDGE_AI_TIMEOUT` | Ollama request timeout in seconds |
| `OLLAMA_HOST` | Ollama API endpoint URL |
| `OLLAMA_API_KEY` | Ollama Bearer token |

If neither env var nor config is set, fledge falls back to `gh auth token` (GitHub CLI) automatically for GitHub operations.

## Project Configuration (fledge.toml)

Per-project settings live in `fledge.toml` in your project root. This file defines tasks, lanes, and project metadata. It's created by `fledge run --init` or `fledge templates init`.

For task and lane configuration, see:
- [Lanes & Pipelines](./lanes.md), defining lanes, step types, parallel groups, importing community lanes
- [Extend: Plugins](./plugins.md), extending fledge with community plugins

## Priority Order

When creating a project, values come from (highest to lowest):

1. Command-line arguments
2. Config file
3. Git config (author only)
4. Built-in defaults
