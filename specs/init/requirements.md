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

## Durable Requirements

### REQ-init-001

The implementation SHALL satisfy the following criterion: `fledge init <name>` creates a directory with rendered template files

Acceptance Criteria

- `fledge init <name>` creates a directory with rendered template files

### REQ-init-002

The implementation SHALL satisfy the following criterion: `fledge init <name> --template <name>` uses the specified template

Acceptance Criteria

- `fledge init <name> --template <name>` uses the specified template

### REQ-init-003

The implementation SHALL satisfy the following criterion: `fledge init <name> --template owner/repo` fetches and uses a remote template

Acceptance Criteria

- `fledge init <name> --template owner/repo` fetches and uses a remote template

### REQ-init-004

The implementation SHALL satisfy the following criterion: Without `--template`, an interactive selector is shown

Acceptance Criteria

- Without `--template`, an interactive selector is shown

### REQ-init-005

The implementation SHALL satisfy the following criterion: `--dry-run` prints file list, hooks, and git status without writing

Acceptance Criteria

- `--dry-run` prints file list, hooks, and git status without writing

### REQ-init-006

The implementation SHALL satisfy the following criterion: `--no-git` skips git init and initial commit

Acceptance Criteria

- `--no-git` skips git init and initial commit

### REQ-init-007

The implementation SHALL satisfy the following criterion: `--no-install` skips post-create hooks

Acceptance Criteria

- `--no-install` skips post-create hooks

### REQ-init-008

The implementation SHALL satisfy the following criterion: `--yes` auto-confirms remote hook prompts

Acceptance Criteria

- `--yes` auto-confirms remote hook prompts

### REQ-init-009

The implementation SHALL satisfy the following criterion: `--refresh` clears cached remote repos before fetching

Acceptance Criteria

- `--refresh` clears cached remote repos before fetching

### REQ-init-010

The implementation SHALL satisfy the following criterion: If the target directory already exists, the command errors immediately

Acceptance Criteria

- If the target directory already exists, the command errors immediately

### REQ-init-011

The implementation SHALL satisfy the following criterion: Git init includes an initial commit with all scaffolded files

Acceptance Criteria

- Git init includes an initial commit with all scaffolded files

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
- Template versioning or pinning (handled by `versioning` module)
