# Ship: Branch, Commit, Push, Release

The Ship pillar takes a clean working tree to a tagged release. Branch, commit (with optional AI-generated messages), push, then bump version and tag. PR creation lives in [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github) (not yet released; use `gh pr create` in the meantime).

## Git workflow with `fledge work`

```bash
fledge work start add-auth                       # creates leif/feat/add-auth
fledge work start fix-crash --branch-type fix    # leif/fix/fix-crash
fledge work start login --issue 42 --branch-type fix  # leif/fix/42-login
fledge work start v0.16 --branch-type chore      # leif/chore/v0.16
fledge work status                               # current branch + ahead/behind + dirty count
```

### Commit changes

`fledge work commit` stages and commits with conventional-commit formatting. The commit type is inferred from the branch prefix (e.g. `feat/` → `feat`). With `--ai` it sends the staged diff to the configured LLM to generate the message.

```bash
fledge work commit -m "add search index"         # explicit message
fledge work commit --all -m "wire up search"     # git add -A first
fledge work commit --ai                          # AI-generated message
fledge work commit --ai --provider ollama --model llama3.2:latest
fledge work commit -t fix -m "handle nil case"   # override commit type
fledge work commit --json                        # {schema_version, action, hash, message, branch}
```

### Push to remote

`fledge work push` pushes the current branch to origin with `-u` tracking. It refuses to push the default branch or when there is nothing to push. The `pre_push` plugin lifecycle hook runs before the push.

```bash
fledge work push                                 # push current branch
fledge work push --force                         # --force-with-lease for safety
fledge work push --json                          # {schema_version, action, branch, remote, force}
```

### Open a PR

PR creation is not in core fledge. Use the `gh` CLI:

```bash
gh pr create                                     # interactive
gh pr create --title "..." --body "..." --draft  # scripted
gh pr create --base develop
```

## GitHub integration (plugin)

Issues, PRs, and CI checks live in [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github). Install with `fledge plugins install --defaults`.

See [GitHub Integration](./github-integration.md) for the full command reference and setup instructions.

## Changelogs with `fledge changelog`

Generate a changelog from git tags and conventional commits. See [Changelog](./changelog.md) for the commit format reference and full options.

## Releases with `fledge release`

Cut a release. Bump the version, generate changelog, create an annotated git tag, and optionally push. Pure git, no GitHub-specific calls (the GitHub Releases UI object is created separately, e.g. via `gh release create`).

```bash
fledge release patch                          # bump patch version
fledge release minor --push                   # bump minor + push to remote
fledge release major --pre-lane ci            # run CI lane first, then bump major
fledge release 2.0.0 --dry-run                # preview a specific version bump
fledge release 2.0.0 --dry-run --json         # preview as JSON envelope
fledge release patch --no-tag --no-changelog  # just bump version, skip extras
fledge release minor --allow-dirty            # release even with uncommitted changes
```

**Options:**
- `--dry-run`: Preview without making changes
- `--no-tag`: Skip git tag
- `--no-changelog`: Skip changelog generation
- `--no-bump`: Skip bumping any version files (tag-only)
- `--push`: Push commit and tag to remote
- `--pre-lane <name>`: Run a lane before releasing (e.g. `ci`)
- `--allow-dirty`: Allow uncommitted changes
- `--json`: Emit a JSON envelope. Suppresses prose output

## Typical flow

```bash
fledge work start add-feature        # 1. branch
# ... code ...
fledge work commit --ai --all        # 2. AI-drafted conventional commit
fledge lanes run pre-commit          # 3. fmt + lint + test + spec-check
fledge work push                     # 4. push to remote
gh pr create --title "..." --draft   # 5. open PR (or via GitHub web UI)
fledge github checks                 # 6. wait for CI (fledge-plugin-github)
gh pr merge <num> --squash           # 7. merge
git checkout main && git pull
fledge release minor --push          # 8. version bump + changelog + tag
gh release create v<X.Y.Z> --notes-file ...   # 9. GitHub Release object
```
