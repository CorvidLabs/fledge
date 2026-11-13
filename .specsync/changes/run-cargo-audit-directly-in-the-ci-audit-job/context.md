---
change: run-cargo-audit-directly-in-the-ci-audit-job
artifact: context
---

# Context

The required `audit` job in `.github/workflows/ci.yml` runs `rustsec/audit-check@v2.0.0`. When
cargo-audit returns any warning, the action reports its result through the Checks API: it
creates a "Security audit" check run, then updates it. It also fails the job if either call
errors, even when no vulnerability was found.

On CorvidLabs/fledge#547, three CI runs reported the same two unmaintained warnings,
RUSTSEC-2024-0384 (`instant`) and RUSTSEC-2025-0119 (`number_prefix`):

| Run (UTC, 2026-11-13) | Result | Why |
|---|---|---|
| 03:42 | pass | The check-run update succeeded. |
| 04:47 | pass | The check-run update succeeded. |
| 14:16 | fail | The check-run update returned `Not Found` (`https://docs.github.com/rest/checks/runs#update-a-check-run`) after "No vulnerabilities were found". |

The workflow already grants `checks: write`. So the gate's verdict currently depends on that
Checks API call succeeding, not only on the audit result.
