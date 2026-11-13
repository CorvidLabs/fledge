---
change: run-cargo-audit-directly-in-the-ci-audit-job
artifact: testing
---

# Testing

- **Local:** `cargo generate-lockfile && cargo audit` (cargo-audit 0.22.2) exits 0 and
  reports 2 allowed warnings, RUSTSEC-2024-0384 and RUSTSEC-2025-0119.
- **CI:** the `audit` job on this PR. It should pass and print the same two warnings.
- `specsync check` passes.
