---
spec: create_template.spec.md
---

## User Stories

- As a template author, I want to scaffold a new template project so I don't have to manually create `template.toml` from scratch
- As a template author, I want example files showing Tera variable usage so I can learn by example
- As a template author, I want to choose which features (hooks, prompts) my template uses so the manifest stays clean

## Durable Requirements

### REQ-create-template-001

The implementation SHALL satisfy the following criterion: `fledge create-template my-template` creates a new directory with a valid template scaffold

Acceptance Criteria

- `fledge create-template my-template` creates a new directory with a valid template scaffold

### REQ-create-template-002

The implementation SHALL satisfy the following criterion: Generated `template.toml` is valid TOML parseable as `TemplateManifest`

Acceptance Criteria

- Generated `template.toml` is valid TOML parseable as `TemplateManifest`

### REQ-create-template-003

The implementation SHALL satisfy the following criterion: Interactive prompts ask for name, description, render globs, hooks, and custom prompts

Acceptance Criteria

- Interactive prompts ask for name, description, render globs, hooks, and custom prompts

### REQ-create-template-004

The implementation SHALL satisfy the following criterion: All prompts have sensible defaults that can be accepted with Enter

Acceptance Criteria

- All prompts have sensible defaults that can be accepted with Enter

### REQ-create-template-005

The implementation SHALL satisfy the following criterion: Includes example `.tera` file demonstrating variable substitution

Acceptance Criteria

- Includes example `.tera` file demonstrating variable substitution

### REQ-create-template-006

The implementation SHALL satisfy the following criterion: Includes author-facing README with instructions for testing locally

Acceptance Criteria

- Includes author-facing README with instructions for testing locally

### REQ-create-template-007

The implementation SHALL satisfy the following criterion: Fails with a clear error if the target directory already exists

Acceptance Criteria

- Fails with a clear error if the target directory already exists

## Acceptance Criteria

- `fledge create-template my-template` creates a new directory with a valid template scaffold
- Generated `template.toml` is valid TOML parseable as `TemplateManifest`
- Interactive prompts ask for name, description, render globs, hooks, and custom prompts
- All prompts have sensible defaults that can be accepted with Enter
- Includes example `.tera` file demonstrating variable substitution
- Includes author-facing README with instructions for testing locally
- Fails with a clear error if the target directory already exists

## Constraints

- Must use `dialoguer` for prompts (consistent with rest of CLI)
- Generated manifest must always pass `toml::from_str::<TemplateManifest>()`
- `template.toml` must always include itself in the ignore list

## Out of Scope

- Template validation (`fledge validate`) — separate feature
- Publishing templates to GitHub — tracked in issue #6
- Git initialization of the template directory
