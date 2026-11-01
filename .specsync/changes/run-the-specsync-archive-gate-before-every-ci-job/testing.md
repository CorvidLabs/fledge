---
change: run-the-specsync-archive-gate-before-every-ci-job
artifact: testing
---

# Testing

- Each edited workflow parses as YAML
  (`ruby -e 'require "psych"; Psych.safe_load(File.read(ARGV[0]))'`).
- Printing every job's `needs` shows `archive-gate` with no dependencies and
  every other job reaching it directly or through `test` / `build`.
- `bash .github/scripts/archive-readiness.sh .` from CorvidLabs/spec-sync run
  against this checkout fails while any change under `.specsync/changes/` is
  open, which is what the `Archive` job will do on the pull request until this
  change and the pre-existing open change are archived.
- `specsync change audit --strict` (6.0.0, the version CI pins) reports the
  workflow paths covered by this change.
- On the pull request, the `Archive` check appears first in each workflow, and
  no other job starts until it passes.
