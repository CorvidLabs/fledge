---
change: CHG-0008-mocking-harness-home-isolation-and-real-publish-coverage-for-network-touching-p
artifact: tasks
---

# Tasks

- [x] `MockHttpServer` + `MockResponse` + `RecordedRequest` + `dead_port_url`
- [x] `GithubBaseGuard` (api / api_and_remote) and `GitIdentityGuard`
- [x] `github::api_base()` and `publish::remote_base()` test-only seams
- [x] `TempEnv` integration fixture; adopted in doctor and main tests
- [x] Real `src/publish.rs` coverage replacing the `#[ignore]` stub
- [x] `github_api_get` and `OllamaProvider::invoke` coverage
- [x] Remote URL asserted via `git remote get-url origin`, separators normalized
- [x] Audit the diff for other OS-path-substring assertions
- [x] Specs and testing plans updated (publish v6, github v4)
- [x] fmt, clippy and tests green
