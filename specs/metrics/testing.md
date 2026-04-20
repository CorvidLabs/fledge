---
spec: metrics.spec.md
---

## Test Plan

### Unit Tests

- Language detection by file extension
- Line classification (code, blank, comment) for each supported language
- Test file pattern matching per language
- Directory exclusion list

### Integration Tests

- `fledge metrics` produces LOC summary in a real project
- `fledge metrics --churn` shows files sorted by commit count
- `fledge metrics --tests` detects test files and computes ratio
- `fledge metrics --json` outputs valid JSON
