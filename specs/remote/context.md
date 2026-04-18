# Remote — Context

## Overview

The remote module enables fledge to fetch templates from GitHub repositories, extending the scaffolding tool beyond bundled and local templates. Users can reference `owner/repo` or `owner/repo/subpath` to use templates hosted on GitHub.

## Integration

- `init` checks if the `--template` argument looks like a remote ref and delegates to the remote fetch flow
- `templates` can fetch repos listed in `config.templates.repos` during discovery
- Authentication uses the GitHub token from `FLEDGE_GITHUB_TOKEN`, `GITHUB_TOKEN`, or `config.github.token`
