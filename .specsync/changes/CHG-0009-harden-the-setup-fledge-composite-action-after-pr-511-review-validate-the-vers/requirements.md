---
change: CHG-0009-harden-the-setup-fledge-composite-action-after-pr-511-review-validate-the-vers
artifact: requirements
---

# Requirements

These amend CHG-0008's REQ-setup-action-4, -5 and -6 and add four new
obligations. CHG-0008's REQ-1, -2, -3 and -7 are unchanged and still hold.

- **REQ-harden-action-1**: The `version` input SHALL be rejected before it is
  used, unless it is `latest` or matches
  `^v?[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$`. A tag resolved from the
  API SHALL be validated the same way.
  - Acceptance: `version: ../../../octocat/Hello-World/releases/download/v1`
    fails with `::error::Invalid version` and makes no network call;
    `v1.7.2`, `1.7.2`, `v1.8.0-rc.1` and `latest` are all accepted.

- **REQ-harden-action-2**: The `install-dir` input SHALL be rejected when it
  contains a `..` path segment or a line break.
  - Acceptance: `install-dir: <workspace>/../escape/bin` fails; an
    `install-dir` containing a newline fails, since it would otherwise inject
    additional lines into `$GITHUB_PATH` and `$GITHUB_OUTPUT`.

- **REQ-harden-action-3** (replaces CHG-0008 REQ-setup-action-5): The action
  SHALL verify the downloaded binary against its published `<asset>.sha256`
  sidecar and SHALL fail the step whenever that verification cannot be
  completed — sidecar unfetchable for any reason, response not a sha256
  digest, or digest mismatch. There SHALL be no input and no network outcome
  that results in an installed but unverified binary.
  - Acceptance: `version: v0.5.0` (a real release publishing
    `fledge-linux-x86_64` with no sidecar) fails rather than warning and
    continuing; a 200 response that is not 64 hex characters fails; a digest
    mismatch fails naming both digests.

- **REQ-harden-action-4** (replaces CHG-0008 REQ-setup-action-6): Every
  network call SHALL retry transient failures. The sidecar fetch SHALL NOT
  retry a `404`, so an absent checksum is reported as absent rather than
  consuming the retry backoff first.
  - Acceptance: the binary download and the API lookup pass
    `--retry-all-errors`; the sidecar fetch passes `--retry 3` without it and
    returns its 404 verdict in well under a second.

- **REQ-harden-action-5** (amends CHG-0008 REQ-setup-action-4): The
  unsupported-platform error SHALL state that *the action* does not support
  the platform yet, and SHALL NOT imply that fledge publishes no binary for
  it. It SHALL point at the releases page where that binary is available.
  - Acceptance: on `windows-latest` the step still fails before any
    release-asset request, and the message names the platform, says "yet", and
    references `fledge-windows-x86_64.exe`.

- **REQ-harden-action-6** (extends CHG-0008 REQ-setup-action-3): The `latest`
  lookup SHALL parse `tag_name` with `jq`, SHALL report the HTTP status when
  the request does not return 200, and SHALL NOT place the token in the
  process arguments of any command.
  - Acceptance: an invalid token produces an error naming HTTP 401 (which also
    proves the Bearer header was sent); a rate-limited response is
    distinguishable from a network failure in the log; the token appears only
    on a curl config file read from stdin.

- **REQ-harden-action-7**: The temporary download directory SHALL be removed
  on every exit path, including failures.
  - Acceptance: running each failing branch leaves the system temp-directory
    count unchanged.

- **REQ-harden-action-8**: `test-action.yml` SHALL cover the refusal paths as
  regression guards, and SHALL NOT produce two runs for a single branch push.
  - Acceptance: a `refuses-unsafe-install` job asserts that both a traversal
    `version` and the sidecar-less `v0.5.0` fail; `on:` is `push` to `main`
    plus `pull_request` to `main`.

- **REQ-harden-action-9**: `README.md` SHALL NOT state a checksum guarantee
  stronger than the implementation provides.
  - Acceptance: the `## GitHub Actions` section describes verification as
    mandatory and says releases before v0.9.1 cannot be installed this way,
    which is what the script does.
