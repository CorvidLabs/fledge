---
change: CHG-0008-add-root-level-action-yml-github-action-for-installing-fledge-in-ci
artifact: tasks
---

# Tasks

- [x] Write `action.yml` at repo root (composite action)
- [x] Write `.github/workflows/test-action.yml` (matrix + windows-unsupported job)
- [x] Add `## GitHub Actions` section to `README.md`
- [x] Add "Moving the `v1` tag" note to `CONTRIBUTING.md`
- [x] Locally verify checksum match/mismatch/missing-sidecar branches
- [x] Locally verify the real pinned + `latest` happy paths end-to-end against the actual v1.7.2 release
- [x] Locally verify the Windows/unsupported-arch guards fail before any network call
- [x] `fledge lanes run check` and `fledge lanes run pre-commit` green
- [x] `specsync check --json` passes
