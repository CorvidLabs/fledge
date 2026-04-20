# Ship — Track and Release

Manage issues, review PRs, check CI status, and generate changelogs. Everything you need to get code out the door without leaving the terminal.

## GitHub issues with `fledge issues`

List and view issues for the current repo.

```bash
fledge issues
fledge issues --state closed
fledge issues --label bug
fledge issues view 42
fledge issues --json
```

## Pull requests with `fledge prs`

List and view PRs.

```bash
fledge prs
fledge prs --state all
fledge prs view 85
fledge prs --json
```

## CI/CD status with `fledge checks`

See what's passing and what's failing on any branch.

```bash
fledge checks
fledge checks --branch feat/add-auth
fledge checks --json
```

## Changelogs with `fledge changelog`

Generate a changelog from git tags and conventional commits.

```bash
fledge changelog
fledge changelog --unreleased     # changes since last tag
fledge changelog --tag v0.7.0     # specific release
fledge changelog --limit 5        # last 5 releases
fledge changelog --json
```
