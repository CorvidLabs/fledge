---
id: CHG-0008-fix-diamond-task-dependencies-falsely-rejected-as-circular
state: implementing
type: bug_fix
base_commit: 6eb7a3ea595c949f4bc0078c0738508d800356b3
---

# Fix diamond task dependencies falsely rejected as circular

## Intent

Fix diamond task dependencies falsely rejected as circular

## Affected Canonical Specs

- `run`
- `lanes`

## Acceptance Criteria

- fledge run a, fledge lanes run build, and fledge lanes validate succeed on diamond DAG a->[b,c] b->d c->d; genuine cycle a->b->a still fails; cargo test and cargo clippy --all-targets pass

## No-spec Rationale

Not applicable
