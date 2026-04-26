# GitHub Integration

GitHub-specific browsing, `checks`, `issues`, `prs`, moved out of core in v0.15. They live in [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github), one of the default plugins. **Branch and PR creation stays in core** via `fledge work`.

```bash
fledge plugins install --defaults
# or just the github plugin:
fledge plugins install CorvidLabs/fledge-plugin-github
```

## Setup

You need a GitHub token:

```bash
# Environment variable (recommended)
export GITHUB_TOKEN="ghp_..."

# Or store it in config
fledge config set github.token "ghp_..."
```

Token priority: `FLEDGE_GITHUB_TOKEN` > `GITHUB_TOKEN` > config file > `gh auth token` (GitHub CLI).

If you have the GitHub CLI (`gh`) installed and authenticated, fledge uses it automatically as a fallback, no extra config needed.

The repo is auto-detected from your git remote.

## Feature Branch Workflow (core)

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

`fledge work pr` auto-generates the body from your commits, shows a preview, and asks you to confirm before pushing. With `--ai` it hands the diff to your configured LLM and gets a richer Markdown body with `## Summary` and `## Test plan` sections.

```bash
fledge work pr                                    # heuristic body, preview + confirm
fledge work pr --ai                               # AI-drafted body, preview + confirm
fledge work pr --title "Add dark mode" --body "..." # explicit overrides
fledge work pr --yes --ai                         # skip the prompt (agent-friendly)
fledge work pr --draft                            # draft PR
fledge work pr --base develop                     # target branch
```

### Check Status (core)

```bash
fledge work status    # current branch + PR status (uses gh under the hood)
```

## Browsing GitHub (plugin)

Once you have `fledge-plugin-github` installed:

### Issues

```bash
fledge issues                    # open issues
fledge issues --state closed     # closed ones
fledge issues --label bug        # filter by label
fledge issues view 42            # specific issue
fledge issues --limit 50
fledge issues --json
```

### Pull Requests

```bash
fledge prs                       # open PRs
fledge prs --state closed
fledge prs view 256
fledge prs --json
```

### CI Status

```bash
fledge checks                    # current branch
fledge checks --branch main
fledge checks --json
```

## AI Code Review (core)

```bash
fledge review                    # single-model review (active config)
fledge review --base develop     # diff against develop
fledge review --file src/main.rs # just one file
fledge review --with-model ollama:gpt-oss:120b-cloud --with-model ollama:qwen3-coder:480b-cloud
                                 # multi-model panel, same diff, parallel critiques
```

## AI Q&A (core)

Ask questions about your codebase. Specs are auto-injected as context.

```bash
fledge ask "how does the template rendering work?"
fledge ask "what tests cover the config module?"
fledge ask --with-specs work,trust "how do these modules interact?"
```

## Typical Workflow

```bash
fledge work start user-auth              # 1. start a branch
fledge run test                          # 2. code + test
fledge lanes run ci                      # 3. run the full pipeline
fledge review --with-model ollama:gpt-oss:120b-cloud  # 4. AI review
fledge work pr --ai                      # 5. AI-drafted PR with preview + confirm
fledge checks                            # 6. watch CI (plugin)
```
