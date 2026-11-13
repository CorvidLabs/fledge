---
change: run-cargo-audit-directly-in-the-ci-audit-job
artifact: design
---

# Design

Keep the `audit` job's id, name, runner, timeout, checkout, toolchain and
`cargo generate-lockfile` steps. Replace the `rustsec/audit-check` step with:

```yaml
- run: cargo install cargo-audit --locked
- run: cargo audit
```

- **Unchanged:** the required-check name (`audit`), the `corvid-pet` and `attest` `needs:`
  entries, and the pass/fail rule.
- **Removed:** the extra "Security audit" check run with the formatted report, and the job's
  use of `secrets.GITHUB_TOKEN`. The warnings are still printed in the job log.
- **Alternative considered:** keeping the action and adding `continue-on-error` on the step.
  Rejected, because that would also hide real vulnerability failures.
