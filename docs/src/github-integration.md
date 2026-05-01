# GitHub Integration

GitHub-specific commands (CI checks, issues, PRs) live in [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github), one of the default plugins. Branch creation is in core via `fledge work start`.

```bash
fledge plugins install --defaults
# or just the github plugin:
fledge plugins install CorvidLabs/fledge-plugin-github
```

## Setup

You need a GitHub token. The easiest option is to install `gh` and run `gh auth login` — fledge uses it as a fallback automatically. Otherwise, set `GITHUB_TOKEN` or configure it via `fledge config set github.token`. See [Configuration: GitHub](./configuration.md#github) for the full token resolution order and required scopes.

## GitHub commands (plugin)

Once you have `fledge-plugin-github` installed, all commands are under `fledge github`:

### Issues

```bash
fledge github issues                    # open issues
fledge github issues --state closed     # closed ones
fledge github issues --label bug        # filter by label
fledge github issues view 42            # specific issue
fledge github issues --limit 50
fledge github issues --json
fledge github issues create --title "bug: something broke"
fledge github issues create --title "..." --body "..." --label bug --json
```

### Pull Requests

```bash
fledge github prs                       # open PRs
fledge github prs --state closed
fledge github prs view 256
fledge github prs --json
fledge github prs create --fill         # infer title/body from commits
fledge github prs create --ai          # AI-generated title + body (via fledge ask) with preview
fledge github prs create --ai --draft  # AI-generated, open as draft
fledge github prs create --title "feat: new thing" --draft --json
```

### CI Status

```bash
fledge github checks                    # current branch
fledge github checks --branch main
fledge github checks --json
```

## Related

- [Ship: Branch, Commit, Push, Release](./ship.md) — branch creation, commit, push, release workflow
- [AI: Ask and Review](./review.md) — multi-model review panels, spec-awareness, output formats
