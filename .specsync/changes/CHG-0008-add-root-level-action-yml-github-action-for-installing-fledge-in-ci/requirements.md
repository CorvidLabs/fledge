---
change: CHG-0008-add-root-level-action-yml-github-action-for-installing-fledge-in-ci
artifact: requirements
---

# Requirements

- **REQ-setup-action-1**: `action.yml` SHALL exist at the repository root so
  that `uses: CorvidLabs/fledge@<ref>` resolves in any consumer workflow.
  - Acceptance: `find . -maxdepth 1 -iname action.yml` finds it; `uses: ./`
    resolves from a checkout of this repo.

- **REQ-setup-action-2**: When `version` is a concrete tag (not `latest`),
  the action SHALL download the release asset directly and SHALL NOT make
  any call to `api.github.com`.
  - Acceptance: with `version: v1.7.2`, the only network calls are to
    `github.com/CorvidLabs/fledge/releases/download/v1.7.2/...`.

- **REQ-setup-action-3**: When `version` is `latest`, the action SHALL
  resolve the tag via the GitHub releases API and SHALL send the `token`
  input (defaulting to `github.token`) as a Bearer credential on that
  request.
  - Acceptance: an `Authorization: Bearer` header is present on the
    `releases/latest` request whenever a non-empty token is available.

- **REQ-setup-action-4**: The action SHALL support only Linux and macOS on
  `x86_64`/`aarch64`. Any other `RUNNER_OS` or `RUNNER_ARCH` SHALL fail the
  step with a `::error::`-annotated, human-readable message before any
  download is attempted.
  - Acceptance: on `windows-latest`, the step fails; no `curl` to a release
    asset is attempted; the message identifies the unsupported platform.

- **REQ-setup-action-5**: The action SHALL verify the downloaded binary
  against its published `<asset>.sha256` sidecar when one exists, and SHALL
  fail the step on a mismatch. When no sidecar exists for a release, it
  SHALL warn and continue rather than fail.
  - Acceptance: a deliberately corrupted binary with a mismatched sidecar
    fails the step with a clear `::error::`; a release lacking a `.sha256`
    file only emits a `::warning::`.

- **REQ-setup-action-6**: Every network call the action makes SHALL pass
  `--retry 3 --retry-all-errors` so that transient network flake does not
  fail the step.
  - Acceptance: `grep -c -- '--retry-all-errors' action.yml` matches every
    `curl` invocation in the script.

- **REQ-setup-action-7**: On success, the action SHALL add the install
  directory to `PATH` via `$GITHUB_PATH`, SHALL expose `version` and `path`
  as step outputs, and SHALL run `fledge --version` as a smoke check before
  the step group completes.
  - Acceptance: a subsequent step in the same job can invoke `fledge`
    unqualified; `steps.<id>.outputs.version` and `.path` are both
    non-empty.
