# Changelog

fledge generates changelogs from your git tags and conventional commits.

## Usage

```bash
fledge changelog                 # recent releases
fledge changelog --unreleased    # changes since last tag
fledge changelog --tag v0.7.0    # specific release
fledge changelog --limit 5       # last 5 releases
fledge changelog --json          # machine-readable
```

## Commit Format

fledge follows [Conventional Commits](https://www.conventionalcommits.org/). The format is:

```
<type>[optional scope]: <description>
```

**Types and how they appear in the changelog:**

| Type | Section |
|------|---------|
| `feat` | Features |
| `fix` | Bug Fixes |
| `docs` | Documentation |
| `chore` | Maintenance |
| `refactor` | Refactoring |
| `perf` | Performance |
| `test` | Tests |
| `ci` | CI/CD |

Commits that don't match any type are grouped under "Other".

**Examples:**

```
feat: add dark mode toggle
fix(auth): handle expired tokens
chore: bump dependencies
refactor: simplify config loader
```

## Versioning

fledge reads git tags that follow semver (`v1.2.3`). Each tag becomes a changelog section. Commits without a tag end up in the `--unreleased` section.

## Config

No config required. fledge reads your git history directly.

## Full Changelog

See [CHANGELOG.md](https://github.com/CorvidLabs/fledge/blob/main/CHANGELOG.md) for the complete release history.
