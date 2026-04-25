# Use Cases & Plugin Ideas

fledge works for solo devs, teams, and AI agents. This page covers real workflows and plugin ideas across all three.

## For Solo Developers

### Zero-config task runner

Drop into any project and run tasks without writing config:

```bash
cd my-rust-project
fledge run test       # detects Cargo.toml, runs cargo test
fledge run lint       # runs cargo clippy
fledge run build      # runs cargo build
```

Works for Rust, Node, Go, Python, Ruby, Java, and Swift out of the box.

### Consistent workflow across projects

Stop remembering which project uses `npm test` vs `cargo test` vs `go test`:

```bash
fledge run test    # always works, regardless of language
fledge lane ci     # same pipeline shape everywhere
```

### Quick code review before pushing

Get a second opinion on your changes without waiting for a human reviewer:

```bash
fledge review                                           # review with active model
fledge review --file src/auth.rs                        # focus on one file
fledge review --with-model ollama:gpt-oss:120b-cloud --with-model ollama:qwen3-coder:480b-cloud
                                                        # multi-model panel — parallel critiques
```

### Branch workflow without remembering git incantations

```bash
fledge work start fix-login      # creates author/fix/fix-login, pushes
fledge work pr --ai              # AI-drafted body, preview + confirm
fledge checks                    # watch CI status (via fledge-plugin-github)
```

## For Teams

### Shared lane definitions

Define your CI pipeline once, share it across all repos:

```bash
# Create a lanes repo
fledge lanes create company-lanes
cd company-lanes
# Edit lanes.toml with your team's pipeline
fledge lanes publish

# In any project
fledge lanes import your-org/company-lanes
fledge lane ci
```

### Pre-PR quality gates via plugins

Install plugins that enforce standards before code ships:

```bash
fledge plugins install your-org/fledge-plugin-lint-config
# Now pre_pr hook runs your org's lint rules before every PR
```

### Dependency auditing across the stack

```bash
fledge plugins install --defaults    # one-line install of fledge-plugin-deps + 4 others
fledge deps --outdated --audit       # Works for Rust, Node, Python — auto-detected from lockfiles
```

## For AI Agents

AI agents benefit from fledge's structured output and consistent interface. Every command supports `--json` for machine-readable output.

### Structured project understanding

```bash
fledge introspect --json         # full command tree (incl. plugin commands)
fledge lanes list --json         # what pipelines exist
fledge plugins list --json       # what extensions are installed
fledge spec list --json          # spec index — semantic project map
fledge ai status --json          # what model is active and where each value came from

# After fledge plugins install --defaults:
fledge deps --json               # dependency report as data
fledge metrics --json            # codebase stats as data
fledge checks --json             # CI status as data
```

### Autonomous code-review-and-fix loops

An AI agent can:
1. `fledge work start fix-issue-42 --issue 42` — create a branch linked to an issue
2. Make changes
3. `fledge lanes run ci` — run the full pipeline
4. `fledge review --with-model ollama:gpt-oss:120b-cloud --with-model ollama:qwen3-coder:480b-cloud --json`
   — multi-model review for higher-confidence findings
5. Fix issues that multiple models agree on
6. `fledge work pr --ai --yes` — AI-drafted PR with no prompt needed

### Plugin protocol for agent tooling

The fledge-v1 plugin protocol is designed for programmatic interaction. Plugins communicate via JSON-over-stdin/stdout, which means AI agents can write and use plugins natively:

```json
{"type": "exec", "id": "1", "command": "cargo test", "cwd": "."}
{"type": "store", "id": "2", "key": "last_run", "value": "2026-04-23T06:00:00Z"}
{"type": "metadata", "id": "3"}
```

Agents get project context, run commands, and persist state — all through a structured protocol instead of shell scraping.

## Plugin Ideas

These are plugins we think would be valuable. Community contributions welcome — see [Building a Plugin](./plugins.md#building-a-plugin) to get started.

### Developer Experience

| Plugin | What it does | Capabilities |
|--------|-------------|--------------|
| `fledge-plugin-env` | Sync `.env` files across environments, warn on missing vars, rotate secrets | `store`, `metadata` |
| `fledge-plugin-docker` | Build, push, compose up/down integrated with lanes | `exec`, `metadata` |
| `fledge-plugin-notify` | Send Slack/Discord/webhook notifications on lane completion or failure | `exec`, `store` |
| `fledge-plugin-bench` | Run benchmarks, track history, flag regressions | `exec`, `store`, `metadata` |
| `fledge-plugin-db` | Database migration workflow — up, down, status, seed | `exec`, `store` |
| `fledge-plugin-todo` | Extract TODOs/FIXMEs from source, track them, warn on stale ones | `metadata` |

### Quality & Security

| Plugin | What it does | Capabilities |
|--------|-------------|--------------|
| `fledge-plugin-coverage` | Test coverage tracking with minimum thresholds and trend graphs | `exec`, `store`, `metadata` |
| `fledge-plugin-secrets` | Scan for leaked secrets, API keys, and credentials before commit | `exec`, `metadata` |
| `fledge-plugin-license` | Check dependency licenses against an allow/deny list | `exec`, `metadata` |
| `fledge-plugin-sbom` | Generate Software Bill of Materials (SPDX/CycloneDX) | `exec`, `metadata` |
| `fledge-plugin-guardian` | Pre-PR gate that blocks merge unless all checks pass | `exec`, `store`, `metadata` |

### AI & Automation

| Plugin | What it does | Capabilities |
|--------|-------------|--------------|
| `fledge-plugin-context` | Generate a project context summary for AI agents (structure, key files, conventions) | `metadata` |
| `fledge-plugin-changelog-ai` | AI-generated changelogs from commit messages with semantic grouping | `exec`, `metadata` |
| `fledge-plugin-migrate` | AI-assisted code migration between frameworks or language versions | `exec`, `store`, `metadata` |
| `fledge-plugin-explain` | Generate explanations of modules, functions, or architectural decisions | `exec`, `metadata` |
| `fledge-plugin-test-gen` | AI-powered test generation for uncovered code paths | `exec`, `store`, `metadata` |

### Infrastructure & Deployment

| Plugin | What it does | Capabilities |
|--------|-------------|--------------|
| `fledge-plugin-deploy` | Deploy to cloud providers (AWS, GCP, Fly.io, Railway) with rollback | `exec`, `store`, `metadata` |
| `fledge-plugin-k8s` | Kubernetes manifest validation, diff, and apply | `exec`, `store`, `metadata` |
| `fledge-plugin-terraform` | Terraform plan/apply integrated with fledge lanes | `exec`, `store` |
| `fledge-plugin-cdn` | Invalidate CDN caches and verify propagation after deploy | `exec`, `store` |

## Example: Building a Notification Plugin

Here's a complete example of a plugin that sends a Discord webhook on lane completion. This demonstrates the plugin protocol in action.

**plugin.toml:**
```toml
[plugin]
name = "fledge-plugin-notify"
version = "0.1.0"
description = "Send notifications on lane events"
protocol = "fledge-v1"

[capabilities]
exec = true
store = true

[[commands]]
name = "notify"
description = "Send a notification"
binary = "bin/notify"

[hooks]
post_install = "bin/setup"
```

**bin/notify** (Python):
```python
#!/usr/bin/env python3
import json, sys

def read_msg():
    return json.loads(input())

def send_msg(msg):
    print(json.dumps(msg), flush=True)

# Read init message from fledge
init = read_msg()
args = init["args"]
project = init["project"]["name"]

# Load saved webhook URL
send_msg({"type": "load", "id": "1", "key": "webhook_url"})
resp = read_msg()
webhook = resp.get("value")

if not webhook:
    send_msg({"type": "output", "text": "No webhook configured. Run: fledge notify --setup"})
    sys.exit(0)

if "--setup" in args:
    send_msg({"type": "prompt", "id": "2", "message": "Discord webhook URL:"})
    resp = read_msg()
    send_msg({"type": "store", "id": "3", "key": "webhook_url", "value": resp["value"]})
    send_msg({"type": "output", "text": "Webhook saved."})
else:
    message = " ".join(args) or f"{project}: lane completed"
    send_msg({
        "type": "exec", "id": "4",
        "command": f"curl -s -X POST -H 'Content-Type: application/json' -d '{{\"content\":\"{message}\"}}' {webhook}"
    })
    result = read_msg()
    send_msg({"type": "output", "text": "Notification sent."})
```

**Using it in a lane:**
```toml
[lanes.deploy]
description = "Build, deploy, notify"
steps = [
  "build",
  { run = "fledge deploy --target production" },
  { run = "fledge notify 'Deploy complete'" },
]
fail_fast = true
```

## Example: AI Context Plugin

A plugin that helps AI agents understand a project quickly:

**plugin.toml:**
```toml
[plugin]
name = "fledge-plugin-context"
version = "0.1.0"
description = "Generate project context for AI agents"
protocol = "fledge-v1"

[capabilities]
metadata = true
exec = true

[[commands]]
name = "context"
description = "Generate project context summary"
binary = "bin/context"
```

The plugin would:
1. Request project metadata (language, name, git info)
2. Run `find` to map the directory structure
3. Read key files (README, config, entry points)
4. Output a structured context document that AI agents can consume

```bash
# Human use
fledge context

# AI agent use (structured output)
fledge context --json
```

This gives any AI agent — Claude Code, Cursor, Copilot — instant project understanding without scanning every file.

## Combining Plugins with Lanes

The real power comes from composing plugins into lanes:

```toml
[lanes.ship]
description = "Full release pipeline"
steps = [
  { parallel = ["lint", "test"] },
  "build",
  { run = "fledge coverage --min 80" },
  { run = "fledge secrets --scan" },
  { run = "fledge deploy --target staging" },
  { run = "fledge notify 'Staging deploy complete — ready for review'" },
]

[lanes.audit]
description = "Full project audit"
fail_fast = false
steps = [
  { run = "fledge deps --outdated --audit" },
  { run = "fledge coverage" },
  { run = "fledge secrets --scan" },
  { run = "fledge license --check" },
  { run = "fledge metrics --churn --tests" },
  { run = "fledge sbom --format spdx" },
]
```

Every step is a plugin command, a built-in task, or an inline command. Mix and match freely.
