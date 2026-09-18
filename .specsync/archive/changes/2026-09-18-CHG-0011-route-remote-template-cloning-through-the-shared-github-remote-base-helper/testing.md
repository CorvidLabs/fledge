---
change: CHG-0011-route-remote-template-cloning-through-the-shared-github-remote-base-helper
artifact: testing
---

# Testing

## REQ-remote-010: cloning derives its URL from the shared helper

- Integration (`tests/isolation.rs`): `templates init <owner>/<repo>` clones from a
  **local bare repository** and renders, with the remote base redirected and no network
  access. Unix-only, because `dirs::cache_dir()` is not env-redirectable on Windows;
  that limitation is documented in the test.
- Negative control: reverting the one-line `src/remote.rs` change makes this test fail by
  attempting a real clone. The test therefore exercises the seam rather than passing
  vacuously.
- Unit (`src/github.rs`): the remote base accepts loopback v4/v6 and existing absolute
  directories, and rejects relative or nonexistent paths.
- Release-gate: outside `cfg(debug_assertions)` the override resolves to `None`, so a
  shipped binary always uses the production constant.

## Rejection signals

- `clone_repo` formatting a `https://github.com/...` URL inline again — that silently
  re-detaches cloning from the shared resolver and from every test that depends on it.
- The clone test passing with the `remote.rs` change reverted, which would mean it is not
  actually exercising the redirection.
- Any override being honoured in a release build.
