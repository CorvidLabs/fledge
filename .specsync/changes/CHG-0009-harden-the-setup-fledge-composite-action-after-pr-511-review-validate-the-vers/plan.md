---
change: CHG-0009-harden-the-setup-fledge-composite-action-after-pr-511-review-validate-the-vers
artifact: plan
---

# Plan

1. Survey sidecar coverage across every release before touching the checksum
   logic — the decision between "remove the skip" and "narrow the skip" turns
   entirely on how many real releases the skip protects (research.md).
2. Confirm the traversal against real github.com rather than reasoning about
   it, and confirm the `-f` + `-w '%{http_code}'` and `--retry`-vs-`--retry-all-errors`
   semantics the new error handling depends on.
3. `action.yml`: add the validation prologue (`version` allowlist,
   `install-dir` guards), the `trap` cleanup, the `jq` + status-capturing
   `latest` lookup with the token moved to a stdin curl config, and the
   mandatory checksum stage. Correct the unsupported-platform message.
4. `.github/workflows/test-action.yml`: add the `refuses-unsafe-install` job,
   narrow `on: push` to `main`, pass step outcomes/outputs through `env:`.
5. `README.md`: replace the checksum claim with what the script actually does,
   note the `version` input's accepted forms, and describe Windows as not
   supported *yet* with a pointer to the binary that ships.
6. Verify locally by extracting the `run:` block byte-for-byte from
   `action.yml` and executing it against the real releases — every happy path
   and every refusal branch, plus `shellcheck` and a temp-dir leak check
   (testing.md).
7. `fledge lanes run pre-commit`, then the change lifecycle: definition
   approval → implement → verify → closing approval → accept.
8. Refresh CHG-0008's evidence with an audited `reopen → verify → accept`.
   Its definition is frozen and stays byte-identical; this workspace exists
   precisely because the definition needed to move.
9. Not in scope, deliberately: Windows support (see design.md), and the moving
   `v1` tag, which remains the manual post-merge step CHG-0008 documented in
   CONTRIBUTING.md.
