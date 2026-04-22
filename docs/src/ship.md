# Ship: Track and Release

Manage issues, review PRs, check CI status, generate changelogs, and cut releases. Everything you need to get code out the door without leaving the terminal.

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

## Releases with `fledge release`

Cut a release — bump the version, generate changelog, create a git tag, and optionally push.

```bash
fledge release patch                          # bump patch version
fledge release minor --push                   # bump minor + push to remote
fledge release major --pre-lane ci            # run CI lane first, then bump major
fledge release 2.0.0 --dry-run               # preview a specific version bump
fledge release patch --no-tag --no-changelog  # just bump version, skip extras
fledge release minor --allow-dirty            # release even with uncommitted changes
```

**Options:**
- `--dry-run` - Preview without making changes
- `--no-tag` - Skip git tag
- `--no-changelog` - Skip changelog generation
- `--push` - Push commit and tag to remote
- `--pre-lane <name>` - Run a lane before releasing (e.g. `ci`)
- `--allow-dirty` - Allow uncommitted changes
