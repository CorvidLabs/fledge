# GitHub Integration

GitHub-specific browsing (`checks`, `issues`, `prs`) lives in [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github), one of the default plugins. **Branch and PR creation stays in core** via `fledge work`.

```bash
fledge plugins install --defaults
# or just the github plugin:
fledge plugins install CorvidLabs/fledge-plugin-github
```

## Setup

You need a GitHub token. The easiest option is to install `gh` and run `gh auth login` — fledge uses it as a fallback automatically. Otherwise, set `GITHUB_TOKEN` or configure it via `fledge config set github.token`. See [Configuration: GitHub](./configuration.md#github) for the full token resolution order and required scopes.

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

## Related

- [Ship: Branch, PR, Release](./ship.md) — branch creation, AI-drafted PRs, release workflow
- [AI: Ask and Review](./review.md) — multi-model review panels, spec-awareness, output formats
