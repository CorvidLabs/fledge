# Develop: Branch and Spec

Work on features with proper branch isolation and keep your specs in sync with the code.

## Work branches with `fledge work`

Instead of manually creating branches and PRs, `fledge work` handles the git ceremony for you.

```bash
# Start a work branch (defaults to feat/ type)
fledge work start add-auth

# Start a bug fix branch
fledge work start login-crash --type fix

# Link to a GitHub issue
fledge work start login-crash --type fix --issue 42

# Check where you are
fledge work status

# Open a PR when ready
fledge work pr --title "Add auth middleware"
```

This creates a branch using your configured format (default: `{author}/{type}/{name}`), and `fledge work pr` opens a pull request against your base branch with sensible defaults.

**Options for `work start`:**
- `-t, --type <TYPE>` - Branch type: `feat`, `fix`, `chore`, `docs`, `hotfix`, `refactor` [default: `feat`]
- `-i, --issue <NUMBER>` - Link to GitHub issue (prefixes branch name with issue number)
- `--prefix <PREFIX>` - Override branch prefix entirely (e.g. `user/leif`)
- `--base <branch>` - Base branch (defaults to `main`)

**Options for `work pr`:**
- `--title <title>` - PR title
- `--body <body>` - PR description
- `--draft` - Open as draft
- `--base <branch>` - Target branch

## Spec-sync with `fledge spec`

Specs are markdown files in `specs/` that define how a module should work. `fledge spec check` validates that the code matches the spec. Catches drift before it becomes a problem.

```bash
# Set up spec-sync
fledge spec init

# Create a new spec
fledge spec new auth

# Check specs against code
fledge spec check
fledge spec check --strict   # warnings become errors
```

Specs are the source of truth. Write the spec first, then write the code to match.
