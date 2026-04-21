---
spec: deps.spec.md
---

## Automated Testing

| Test File | Type | What It Covers |
|-----------|------|----------------|
| `src/deps.rs` (inline) | Unit | Cargo.lock parsing, package-lock.json parsing, yarn.lock parsing, go.sum parsing, requirements.txt parsing, Pipfile.lock parsing, poetry.lock parsing, Gemfile.lock parsing, unquote helper, generic project detection |

## Manual Testing

- [x] `fledge deps` in a Rust project lists all Cargo.lock dependencies sorted alphabetically
- [x] `fledge deps --json` outputs valid JSON with ecosystem, source, and dependencies array
- [x] `fledge deps --outdated` runs `cargo outdated` (or equivalent per ecosystem)
- [x] `fledge deps --audit` runs `cargo audit` (or equivalent per ecosystem)
- [x] `fledge deps --licenses` runs `cargo license` (or equivalent per ecosystem)
- [x] Running in a directory with no recognized project type shows a clear error listing supported types
- [x] Running in a Rust project without Cargo.lock shows an error suggesting `cargo generate-lockfile`
- [x] Running in a Node project with pnpm-lock.yaml shows an error suggesting direct pnpm commands

## Edge Cases & Boundary Conditions

| Scenario | Expected Behavior |
|----------|-------------------|
| No lock file present | Error with install command for the detected ecosystem |
| Empty lock file | Returns empty dependency list (0 deps) |
| Lock file with only root package (npm) | Root package entry (empty key) is skipped |
| Duplicate entries in go.sum | Deduplicated via HashSet |
| Duplicate entries in yarn.lock | Deduplicated via HashSet |
| requirements.txt with comments/blank lines | Skipped correctly |
| requirements.txt with bare package name (no version) | Version shown as "*" |
| Gemfile.lock sub-dependencies (6-space indent) | Skipped, only top-level gems (4-space) parsed |
| Java project (Gradle/Maven) | Informational message, empty dep list, no error |
| Missing ecosystem tool (e.g., cargo-outdated not installed) | Clear error with install suggestion |
| Ecosystem tool exits non-zero | Warning printed but no error returned |
| Generic/unrecognized project | Hard error listing supported project types |
