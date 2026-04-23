# Develop: Branch and Spec

Work on features with proper branch isolation and keep your specs in sync with the code.

## Work branches with `fledge work`

Instead of manually creating branches and PRs, `fledge work` handles the git ceremony for you.

```bash
# Start a work branch (defaults to feat/ type)
fledge work start add-auth

# Start a bug fix branch
fledge work start login-crash --branch-type fix

# Link to a GitHub issue
fledge work start login-crash --branch-type fix --issue 42

# Check where you are
fledge work status

# Open a PR when ready
fledge work pr --title "Add auth middleware"
```

This creates a branch using your configured format (default: `{author}/{type}/{name}`), and `fledge work pr` opens a pull request against your base branch with sensible defaults.

**Options for `work start`:**
- `-t, --branch-type <TYPE>` - Branch type: `feat`, `feature`, `fix`, `bug`, `chore`, `task`, `docs`, `hotfix`, `refactor` [default: `feat`]
- `-i, --issue <NUMBER>` - Link to GitHub issue (prefixes branch name with issue number)
- `--prefix <PREFIX>` - Override branch prefix entirely (e.g. `user/leif`)
- `--base <branch>` - Base branch (defaults to `main`)

**Options for `work pr`:**
- `-t, --title <title>` - PR title
- `-b, --body <body>` - PR description
- `--draft` - Open as draft
- `--base <branch>` - Target branch

## Spec-sync with `fledge spec`

Specs are markdown files in `specs/` that define how a module should work. `fledge spec check` validates that the code matches the spec.

```bash
# Set up spec-sync
fledge spec init

# Create a new spec
fledge spec new auth

# Check specs against code
fledge spec check
fledge spec check --strict   # warnings become errors
```

### Spec format

Each spec is a markdown file with a YAML frontmatter block:

```markdown
---
module: auth
version: 1
status: active
---

# Auth Module

Description of what this module does.

## Public API

List the public functions/types and what they do.

## Invariants

Any guarantees the code must uphold.
```

fledge reads the frontmatter to track the module name, version, and status. The body is free-form markdown.

### Validation rules

`fledge spec check` verifies:

1. Every spec in `specs/` has a corresponding source file (no orphaned specs)
2. Every tracked module has a spec (no undocumented modules)
3. Spec frontmatter is valid YAML with required fields (module, version, status)
4. Version field is present (integer)

With `--strict`, warnings (missing optional fields, minor drift) become errors.

### Workflow

Write the spec first, then write the code to match. Before committing, run:

```bash
fledge spec check
```

Add it to your CI lane:

```toml
[lanes.ci]
steps = ["fmt", "lint", "test", "spec-check"]

[tasks.spec-check]
cmd = "fledge spec check"
```

The `.specsync/hashes.json` file tracks content hashes. Commit it alongside spec changes so CI can detect drift.
