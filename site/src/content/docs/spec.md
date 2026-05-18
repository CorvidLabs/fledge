---
title: "Spec: Spec-sync"
section: "The six pillars"
order: 3
---


Keep your specs in sync with the code. Specs are the source of truth for module design. Write the spec first, then write the code to match.

## Why spec-sync?

Design docs rot. They get written once, then drift as the code evolves. Spec-sync solves this by making specs machine-checkable: `fledge spec check` validates that the code matches the spec, and CI catches drift before it lands. AI commands (`fledge ask`, `fledge review`) automatically inject relevant specs as context, so the model knows *why* the code is shaped the way it is, not just *what* it does.

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

## Spec format

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

## Validation rules

`fledge spec check` verifies:

1. Every spec in `specs/` has a corresponding source file (no orphaned specs)
2. Every tracked module has a spec (no undocumented modules)
3. Spec frontmatter is valid YAML with required fields (module, version, status)
4. Version field is present (integer)

With `--strict`, warnings (missing optional fields, minor drift) become errors.

## Workflow

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
