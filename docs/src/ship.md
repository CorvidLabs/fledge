# Ship: Branch, PR, Release

The Ship pillar takes a clean working tree to a tagged release. Branch, draft a PR (with the LLM if you want), preview, push, then bump version and tag.

## Branch and PR with `fledge work`

```bash
fledge work start add-auth                       # creates leif/feat/add-auth
fledge work start fix-crash --branch-type fix    # leif/fix/fix-crash
fledge work start login --issue 42 --branch-type fix  # leif/fix/42-login
fledge work start v0.16 --branch-type chore      # leif/chore/v0.16
fledge work status                               # current branch + PR + ahead/behind
```

### Open a PR

`fledge work pr` auto-generates the body from your commits, shows a styled preview (title, head→base, draft tag, full body), and prompts y/n before pushing. With `--ai` it hands the diff to the configured LLM and gets back a Markdown body with `## Summary` + `## Test plan` sections.

```bash
fledge work pr                                   # heuristic body, preview + confirm
fledge work pr --ai                              # AI-drafted body, preview + confirm
fledge work pr --yes --ai                        # skip the prompt (agent-friendly)
fledge work pr --title "..." --body "..."        # explicit overrides
fledge work pr --draft
fledge work pr --base develop
fledge work pr --json                            # {url, number, title, head, base, draft}

# Per-call AI provider/model overrides
fledge work pr --ai --provider ollama --model gpt-oss:120b-cloud --yes
```

The preview reads:

```
────────────────────────────────────────────────────────────
Title: feat: work pr — auto body + preview + confirm
Branch:  0xleif/feat/pr-preview-and-body → main

  ## Summary
  - Add styled preview block before any push or gh call
  - Add yes/no confirmation prompt with default Yes
  - Add --ai flag for LLM-drafted body
  ...

────────────────────────────────────────────────────────────
? Create this pull request? (Y/n)
```

Choosing **n** prints `✋ Aborted.` and exits 0 with no side effects — nothing is pushed.

## GitHub browsing (plugin)

Read-only views of issues, PRs, and CI status moved to [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github) in v0.15. PR *creation* stays in core via `fledge work pr` above. Install:

```bash
fledge plugins install --defaults
```

Then:

```bash
fledge issues                          # GitHub issues (open by default)
fledge issues view 42 --json
fledge prs                             # GitHub PRs
fledge prs view 256 --json
fledge checks                          # CI status for current branch
fledge checks --branch main --json
```

## Changelogs with `fledge changelog`

Generate a changelog from git tags and conventional commits.

```bash
fledge changelog
fledge changelog --unreleased     # changes since last tag
fledge changelog --tag v0.15.0    # specific release
fledge changelog --limit 5        # last 5 releases
fledge changelog --json
```

## Releases with `fledge release`

Cut a release — bump the version, generate changelog, create an annotated git tag, and optionally push. Pure git, no GitHub-specific calls (the GitHub Releases UI object is created separately, e.g. via `gh release create`).

```bash
fledge release patch                          # bump patch version
fledge release minor --push                   # bump minor + push to remote
fledge release major --pre-lane ci            # run CI lane first, then bump major
fledge release 2.0.0 --dry-run                # preview a specific version bump
fledge release patch --no-tag --no-changelog  # just bump version, skip extras
fledge release minor --allow-dirty            # release even with uncommitted changes
```

**Options:**
- `--dry-run` — Preview without making changes
- `--no-tag` — Skip git tag
- `--no-changelog` — Skip changelog generation
- `--push` — Push commit and tag to remote
- `--pre-lane <name>` — Run a lane before releasing (e.g. `ci`)
- `--allow-dirty` — Allow uncommitted changes

## Typical flow

```bash
fledge work start add-feature        # 1. branch
# ... code, commit ...
fledge lanes run pre-commit          # 2. fmt + lint + test + spec-check
fledge work pr --ai                  # 3. AI-drafted PR with preview/confirm
fledge checks                        # 4. wait for CI (plugin)
gh pr merge <num> --squash           # 5. merge (or via fledge work pr's URL on GitHub)
git checkout main && git pull
fledge release minor --push          # 6. version bump + changelog + tag
gh release create v<X.Y.Z> --notes-file ...   # 7. GitHub Release object (manual)
```
