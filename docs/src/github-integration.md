# GitHub Integration

GitHub-specific browsing (`checks`, `issues`, `prs`) lives in [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github), one of the default plugins. **Branch and PR creation stays in core** via `fledge work`.

```bash
fledge plugins install --defaults
# or just the github plugin:
fledge plugins install CorvidLabs/fledge-plugin-github
```

## Setup

You need a GitHub token. The easiest option is to install `gh` and run `gh auth login` — fledge uses it as a fallback automatically. Otherwise, set `GITHUB_TOKEN` or configure it via `fledge config set github.token`. See [Configuration: GitHub](./configuration.md#github) for the full token resolution order and required scopes.

## Feature Branch Workflow (core)

Branch and PR creation live in core via `fledge work`. See [Ship: Branch, PR, Release](./ship.md) for the full workflow, including `--ai`-drafted PR bodies, branch type options, and issue linking.

```bash
fledge work start add-dark-mode                   # creates leif/feat/add-dark-mode
fledge work pr --ai                               # AI-drafted body, preview + confirm
fledge work status                                # current branch + PR status
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

## AI Review & Q&A (core)

AI code review and codebase Q&A are core commands. See [AI: Ask and Review](./review.md) for details on multi-model review panels, spec-awareness, and output formats.

```bash
fledge review                    # single-model review (active config)
fledge review --with-model ollama   # multi-model panel
fledge ask "how does the template rendering work?"
```

## Typical Workflow

```bash
fledge work start user-auth              # 1. start a branch
fledge run test                          # 2. code + test
fledge lanes run ci                      # 3. run the full pipeline
fledge review --with-model ollama              # 4. AI review
fledge work pr --ai                      # 5. AI-drafted PR with preview + confirm
fledge checks                            # 6. watch CI (plugin)
```
