---
id: CHG-0011-route-remote-template-cloning-through-the-shared-github-remote-base-helper
state: accepted
type: refactor
base_commit: f8fd97a9c97489f705cf3ffc036af63fe1646f89
---

# Route remote template cloning through the shared GitHub remote-base helper

## Intent

Route remote template cloning through the shared GitHub remote-base helper

## Affected Canonical Specs

- `remote`

## Acceptance Criteria

- clone_repo composes its git URL via github::remote_url rather than an inline https://github.com literal; templates init <owner>/<repo> clones and renders from a local bare repository with no network access; reverting the one-line remote.rs change makes that test fail by attempting a real clone; outside cfg(debug_assertions) the override is absent and the production constant is used; cargo test, cargo clippy --all-targets -- -D warnings and cargo fmt --check are green.

## No-spec Rationale

Not applicable
