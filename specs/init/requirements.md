---
spec: init.spec.md
---

## User Stories

- As a user, I want to run `fledge init my-project` and get a fully scaffolded project from a template
- As a user, I want to pick a template interactively if I don't specify one
- As a user, I want to use remote templates from GitHub with `fledge init my-app --template owner/repo`
- As a user, I want to preview what would happen with `--dry-run` before committing
- As a user, I want post-create hooks (like `npm install`) to run automatically for local templates
- As a user, I want to be warned before remote templates execute hooks on my machine

## Acceptance Criteria

- `fledge init <name>` creates a directory with rendered template files
- `fledge init <name> --template <name>` uses the specified template
- `fledge init <name> --template owner/repo` fetches and uses a remote template
- Without `--template`, an interactive selector is shown
- `--dry-run` prints file list, hooks, and git status without writing
- `--no-git` skips git init and initial commit
- `--no-install` skips post-create hooks
- `--yes` auto-confirms remote hook prompts
- `--refresh` clears cached remote repos before fetching
- If the target directory already exists, the command errors immediately
- Git init includes an initial commit with all scaffolded files

## Constraints

- Must work in CI environments (headless, no git identity configured)
- Hooks execute via `sh -c` — must work on macOS and Linux
- Remote template hooks are untrusted by default

## Out of Scope

- Merging into an existing directory
- Updating a previously scaffolded project
- Template versioning or pinning
