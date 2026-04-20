# GitHub Integration

Fledge integrates with GitHub for issue tracking, pull requests, CI status, and a branch-based development workflow — all from the terminal.

## Setup

Fledge needs a GitHub token for API access. Set it via environment variable or config:

```bash
# Environment variable (recommended)
export GITHUB_TOKEN="ghp_..."

# Or via fledge config
fledge config set github.token "ghp_..."
```

Fledge checks tokens in order: `FLEDGE_GITHUB_TOKEN` > `GITHUB_TOKEN` > config file.

The repository is auto-detected from your git remote (`origin`).

## Feature Branch Workflow

The `fledge work` command provides a streamlined git workflow:

### Start a Feature Branch

```bash
fledge work start add-dark-mode
# Creates and switches to feat/add-dark-mode

fledge work start fix-login --base develop
# Branch from develop instead of main
```

Branch names are automatically sanitized (spaces become hyphens, special characters removed) and prefixed with `feat/`.

### Create a Pull Request

```bash
# Auto-generate title from branch name
fledge work pr

# Custom title and body
fledge work pr --title "Add dark mode support" --body "Implements dark/light theme toggle"

# Create as draft
fledge work pr --draft

# Target a specific base branch
fledge work pr --base develop
```

### Check Status

```bash
fledge work status
# Shows current branch and associated PR status
```

## Issues

```bash
# List open issues
fledge issues

# Filter by state
fledge issues --state closed
fledge issues --state all

# Filter by label
fledge issues --label bug

# View a specific issue
fledge issues view 42

# Limit results
fledge issues --limit 50

# JSON output
fledge issues --json
```

## Pull Requests

```bash
# List open PRs
fledge prs

# Filter by state
fledge prs --state closed

# View a specific PR
fledge prs view 75

# JSON output
fledge prs --json
```

## CI/CD Status

Check the status of CI checks on any branch:

```bash
# Current branch
fledge checks

# Specific branch
fledge checks --branch main

# JSON output
fledge checks --json
```

## AI-Powered Review

Get an AI code review of your current changes using Claude:

```bash
# Review all changes on current branch
fledge review

# Review against a specific base
fledge review --base develop

# Review a single file
fledge review --file src/main.rs
```

## AI Q&A

Ask questions about your codebase:

```bash
fledge ask "how does the template rendering work?"
fledge ask "what tests cover the config module?"
```

## Typical Workflow

```bash
# 1. Start a feature
fledge work start user-authentication

# 2. Write code, run tasks
fledge run test
fledge lane ci

# 3. Review your changes
fledge review

# 4. Create a PR
fledge work pr --title "Add user authentication"

# 5. Check CI status
fledge checks

# 6. Monitor issues
fledge issues --label "v1.0"
```
