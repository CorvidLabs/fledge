---
id: accept-windows-refused-connection-wording-in-the-isolation-test
state: implementing
type: bug_fix
base_commit: c65af7f55d832475c877fb9032bc89da4bb685f3
---

# Accept Windows' refused-connection wording in the isolation test

## Intent

Accept Windows' refused-connection wording in the isolation test

## Affected Canonical Specs

- None

## Acceptance Criteria

- default_temp_env_points_github_at_a_dead_port in tests/isolation.rs passes on Linux, macOS and Windows: it still asserts the default TempEnv GitHub base starts with http://127.0.0.1: and that the spawned templates search fails, and it recognizes the refusal as 'connection refused' (Linux, macOS) or Windows' WSAECONNREFUSED wording ('actively refused' / 'os error 10061'), so a genuine api.github.com failure still does not satisfy it; cargo test --locked is green on ubuntu-latest, macos-latest and windows-latest with both the ureq 3.3.0 lockfile on main and the ureq 3.4.2 lockfile the v1.8.0 release carries

## No-spec Rationale

Only tests/isolation.rs changes, and tests/ is outside source_dirs (src, templates). The behavior under test is unchanged: the default TempEnv GitHub base is loopback and the spawned search fails with a refused connection. The test now also recognizes the Windows wording of that refusal, which ureq 3.4 passes through from the OS. specs/github/testing.md already describes the test in wording-neutral terms.
