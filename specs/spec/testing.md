---
spec: spec.spec.md
---

## Test Plan

### Unit Tests

- Parse valid YAML frontmatter with all fields
- Parse frontmatter with missing optional fields
- Reject frontmatter missing required fields (module, version, status, files)
- Detect missing required sections in spec body
- Detect all required sections present
- Validate source files exist (using temp directory)
- Validate source files missing reports error
- Scaffold spec init creates correct file structure
- Scaffold spec new creates correct file structure
- Check report formatting with mixed results

### Integration Tests

- `fledge spec check` on fledge's own specs (self-validation)
- `fledge spec init` in a temp directory
- `fledge spec new` in an initialized project
- `fledge spec list` enumerates all specs, sorted by name
- `fledge spec list --json` produces parseable JSON array (empty array for empty projects)
- `fledge spec ls` is a working alias for `list`
- `fledge spec show <name>` prints frontmatter, sections, companions for a real module
- `fledge spec show <name> --json` produces parseable JSON object
- `fledge spec show <missing>` exits non-zero with a suggestion to run `fledge spec list`
