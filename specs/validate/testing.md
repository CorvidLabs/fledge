---
spec: validate.spec.md
---

## Test Plan

### Unit Tests

- Tera syntax validation (valid and broken templates)
- Variable extraction and builtin filtering
- GitHub Actions expression exclusion
- Render glob matching

### Integration Tests

- Valid template passes with green output
- Missing template.toml produces error
- Empty name/description produces error
- Broken Tera syntax in rendered file produces error
- Undefined variable produces warning
- Strict mode promotes warnings to errors
- JSON output is valid
- Batch mode validates multiple templates
