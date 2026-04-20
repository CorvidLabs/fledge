# Develop — Branch and Spec

Work on features with proper branch isolation and keep your specs in sync with the code.

## Feature branches with `fledge work`

Instead of manually creating branches and PRs, `fledge work` handles the git ceremony for you.

```bash
# Start a feature branch
fledge work start add-auth

# Check where you are
fledge work status

# Open a PR when ready
fledge work pr --title "Add auth middleware"
```

This creates a `feat/add-auth` branch, and `fledge work pr` opens a pull request against your base branch with sensible defaults.

**Options for `work start`:**
- `--base <branch>` — Base branch (defaults to `main`)

**Options for `work pr`:**
- `--title <title>` — PR title
- `--body <body>` — PR description
- `--draft` — Open as draft
- `--base <branch>` — Target branch

## Spec-sync with `fledge spec`

Specs are markdown files in `specs/` that define how a module should work. `fledge spec check` validates that the code matches the spec — catches drift before it becomes a problem.

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
