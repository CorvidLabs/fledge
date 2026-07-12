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

### REQ-init-001

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge init <name>` creates a directory with rendered template files
### REQ-init-002

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge init <name> --template <name>` uses the specified template
### REQ-init-003

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge init <name> --template owner/repo` fetches and uses a remote template
### REQ-init-004

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Without `--template`, an interactive selector is shown
### REQ-init-005

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `--dry-run` prints file list, hooks, and git status without writing
### REQ-init-006

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `--no-git` skips git init and initial commit
### REQ-init-007

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `--no-install` skips post-create hooks
### REQ-init-008

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `--yes` auto-confirms remote hook prompts
### REQ-init-009

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `--refresh` clears cached remote repos before fetching
### REQ-init-010

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- If the target directory already exists, the command errors immediately
### REQ-init-011

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Git init includes an initial commit with all scaffolded files

## Constraints

- Must work in CI environments (headless, no git identity configured)
- Hooks execute via `sh -c` — must work on macOS and Linux
- Remote template hooks are untrusted by default

## Out of Scope

- Merging into an existing directory
- Template versioning or pinning (handled by `versioning` module)
