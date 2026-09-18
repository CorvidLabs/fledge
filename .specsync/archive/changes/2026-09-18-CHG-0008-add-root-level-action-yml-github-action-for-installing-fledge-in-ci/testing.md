---
change: CHG-0008-add-root-level-action-yml-github-action-for-installing-fledge-in-ci
artifact: testing
---

# Testing

## Automated (continuous, runs on every push)

`.github/workflows/test-action.yml`:

- `install` matrix (`ubuntu-latest`/`macos-latest` × `v1.7.2`/`latest`, `uses:
  ./` so it always tests the branch's own code): asserts both outputs are
  non-empty, a pinned `version` round-trips unchanged, the installed binary
  runs, and `fledge` resolves on `PATH`.
- `windows-unsupported`: asserts the step fails on `windows-latest`
  (`continue-on-error: true` + checks `outcome == 'failure'`), pinning down
  REQ-setup-action-4 as a regression guard.

## Manual verification performed during this change (local, macOS arm64,
authenticated `gh` session against the real `CorvidLabs/fledge` repo)

The exact `run:` block was extracted byte-for-byte from `action.yml` and
executed directly (not simulated) against the real v1.7.2 release:

1. **Pinned happy path** (`version: v1.7.2`, `RUNNER_OS=macOS`,
   `RUNNER_ARCH=ARM64`): downloaded `fledge-macos-aarch64`, verified its real
   `.sha256` sidecar, installed, `fledge --version` printed `fledge 1.7.2`,
   `outputs.version=v1.7.2`. Confirms REQ-setup-action-1/2/5/7.
2. **`latest`, authenticated** (real `gh auth token`): resolved to `v1.7.2`
   (matches `git describe --tags --abbrev=0`), same success path. The API
   call hit transient `curl (56)`/`(16)` network errors in the sandbox during
   this run and `--retry-all-errors` transparently recovered — an
   unplanned but direct live demonstration of REQ-setup-action-6, not just a
   read of the script. Confirms REQ-setup-action-3.
3. **Unsupported OS** (`RUNNER_OS=Windows`): failed with exit 1 and the
   expected `::error::` message, before any `curl` to a release asset ran.
   Confirms REQ-setup-action-4.
4. **Unsupported arch** (`RUNNER_ARCH=ARM`): same, exit 1 with a clear
   message. Confirms REQ-setup-action-4.
5. **Checksum match/mismatch/missing-sidecar**, isolated: reproduced the
   exact `cut`/`sha256sum`/`shasum` comparison logic against a fake binary +
   sidecar in the format `release.yml` actually produces
   (`sha256sum "$file" | sed "s|.*/||" > "${file}.sha256"`). A matching pair
   verifies; a corrupted binary against the original sidecar is correctly
   detected and rejected; an absent sidecar file falls through to the
   `curl -fsSL` failure branch, which the script already handles as a warn-
   and-skip. Confirms REQ-setup-action-5. (Not exercised against a real
   corrupted GitHub release asset — can't corrupt production release
   infrastructure to test this end-to-end; the isolated reproduction uses
   the identical shell commands the action runs.)
6. `python3 -c "import yaml; yaml.safe_load(...)"` on both `action.yml` and
   `test-action.yml` — both parse as valid YAML.

## Rejection signal

If a pinned `version` ever triggers a request to `api.github.com`, if
`latest` is ever requested without an `Authorization` header when a token is
available, if a checksum mismatch installs anyway, or if the
`windows-unsupported` job in CI ever goes green, the change is wrong.
