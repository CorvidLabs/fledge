# GitHub Integration

fledge talks to GitHub for issues, PRs, CI status, and a branch-based dev workflow. All from the terminal.

## Setup

You need a GitHub token:

```bash
# Environment variable (recommended)
export GITHUB_TOKEN="ghp_..."

# Or store it in config
fledge config set github.token "ghp_..."
```

Token priority: `FLEDGE_GITHUB_TOKEN` > `GITHUB_TOKEN` > config file > `gh auth token` (GitHub CLI).

If you have the GitHub CLI (`gh`) installed and authenticated, fledge uses it automatically as a fallback — no extra config needed.

The repo is auto-detected from your git remote.

## Feature Branch Workflow

### Start a Branch

```bash
fledge work start add-dark-mode
# → creates leif/feat/add-dark-mode (default: {author}/{type}/{name})

fledge work start login-crash --branch-type fix
# → creates leif/fix/login-crash

fledge work start fix-login --base develop
# → branches from develop instead of main

fledge work start login-crash --issue 42
# → creates leif/feat/42-login-crash (linked to issue #42)

fledge work start my-feature --prefix user/leif
# → creates user/leif/my-feature (custom prefix, overrides format)
```

Branch names get sanitized automatically (spaces → hyphens, special chars removed). The default type is `feat`, but you can use `feature`, `fix`, `bug`, `chore`, `task`, `docs`, `hotfix`, or `refactor` via `--branch-type` (or `-t` for short). The branch format is configurable in `fledge.toml`.

### Open a PR

```bash
fledge work pr                                    # auto-title from branch
fledge work pr --title "Add dark mode" --body "Adds dark mode toggle to settings" # custom
fledge work pr --draft                            # draft PR
fledge work pr --base develop                     # target branch
```

### Check Status

```bash
fledge work status    # current branch + PR status
```

## Issues

```bash
fledge issues                    # open issues
fledge issues --state closed     # closed ones
fledge issues --label bug        # filter by label
fledge issues view 42            # specific issue
fledge issues --limit 50
fledge issues --json
```

## Pull Requests

```bash
fledge prs                       # open PRs
fledge prs --state closed
fledge prs view 75
fledge prs --json
```

## CI Status

```bash
fledge checks                    # current branch
fledge checks --branch main
fledge checks --json
```

## AI Code Review

Uses Claude to review your changes:

```bash
fledge review                    # all changes on current branch
fledge review --base develop     # diff against develop
fledge review --file src/main.rs # just one file
```

## AI Q&A

Ask questions about your codebase:

```bash
fledge ask "how does the template rendering work?"
fledge ask "what tests cover the config module?"
```

## Typical Workflow

```bash
fledge work start user-auth      # 1. start a work branch
fledge run test                  # 2. code + test
fledge lanes run ci              # 3. run the full pipeline
fledge review                    # 4. AI review
fledge work pr --title "Add user auth"  # 5. open PR
fledge checks                    # 6. watch CI
```
