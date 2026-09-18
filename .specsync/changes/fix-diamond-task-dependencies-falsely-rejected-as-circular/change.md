---
id: fix-diamond-task-dependencies-falsely-rejected-as-circular
state: accepted
type: bug_fix
base_commit: 83d7561b8571d17270016e3f933e1733475d1d83
---

# Fix diamond task dependencies falsely rejected as circular

## Intent

Fix diamond task dependencies falsely rejected as circular

## Affected Canonical Specs

- `run`
- `lanes`
- `main`

## Acceptance Criteria

- fledge run a, fledge lanes run build, and fledge lanes validate succeed on diamond DAG a->[b,c] b->d c->d; genuine cycle a->b->a still fails; cargo test and cargo clippy --all-targets pass

## No-spec Rationale

Not applicable
