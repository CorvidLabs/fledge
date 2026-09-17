---
change: CHG-0008-mocking-harness-home-isolation-and-real-publish-coverage-for-network-touching-p
artifact: plan
---

# Plan

1. Build `MockHttpServer`, `MockResponse`, `RecordedRequest` and `dead_port_url` in
   `src/test_support.rs`.
2. Add `GithubBaseGuard` and `GitIdentityGuard` with thread-local RAII semantics.
3. Introduce the `github::api_base()` and `publish::remote_base()` seams, gated so
   release builds are unchanged.
4. Add `TempEnv` to `tests/common/mod.rs` and adopt it in `tests/doctor.rs` and
   `tests/main.rs`.
5. Replace the `src/publish.rs` stub module with real coverage of the authenticated
   user, repo existence, repo creation, topic set, push and orchestration paths.
6. Cover `github_api_get` and `OllamaProvider::invoke` error and success paths.
7. Assert remote URLs via `git remote get-url origin` with normalized separators.
8. Update the `publish`, `github`, `llm` and `doctor` testing plans and bump the
   `publish` and `github` specs for the base-URL indirection.
9. Run the full gate on all three platforms via CI.
