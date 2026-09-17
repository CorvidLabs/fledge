---
change: CHG-0009-harden-the-setup-fledge-composite-action-after-pr-511-review-validate-the-vers
artifact: tasks
---

# Tasks

## Research before implementing

- [x] Survey sidecar coverage across all 39 releases (decides remove-vs-narrow)
- [x] Reproduce the curl path-traversal against real github.com
- [x] Verify `-f` + `-w '%{http_code}'` and `--retry` vs `--retry-all-errors` semantics

## Blocker 1 — input validation

- [x] Allowlist `version` to `latest` or a release tag, before any use
- [x] Re-validate the tag resolved from the API
- [x] Reject `..` segments and line breaks in `install-dir`

## Blocker 2 — checksum verification

- [x] Capture `%{http_code}` on the sidecar fetch; fail on any non-200
- [x] Report a 404 as "no sidecar published" rather than as a generic failure
- [x] Reject a sidecar body that is not a 64-hex-char digest
- [x] Drop `--retry-all-errors` from the sidecar fetch so a 404 fails fast
- [x] Correct README.md's checksum claim to match the implementation

## Correctness — Windows

- [x] Rewrite the unsupported-platform error: the action doesn't support it
      *yet*; fledge does publish `fledge-windows-x86_64.exe`
- [x] Update README.md's platform sentence to match
- [x] Rename the regression job so its name doesn't imply the binary is missing

## Nits

- [x] Parse `tag_name` with `jq`; drop the `|| true`
- [x] Surface the HTTP status when the `latest` lookup fails
- [x] Move the Bearer token from argv to a stdin curl config file
- [x] `trap 'rm -rf "$tmp"' EXIT`
- [x] Narrow `on: push` to `main` so a branch push runs the workflow once

## Verification

- [x] `refuses-unsafe-install` job covering traversal + sidecar-less v0.5.0
- [x] Pass step outcomes/outputs into assertion steps via `env:`
- [x] Re-run every branch of the extracted script locally (testing.md)
- [x] `shellcheck -s bash` clean; both YAML files parse
- [x] `fledge lanes run pre-commit` green
- [x] Refresh CHG-0008's stale evidence via audited `reopen → verify → accept`
