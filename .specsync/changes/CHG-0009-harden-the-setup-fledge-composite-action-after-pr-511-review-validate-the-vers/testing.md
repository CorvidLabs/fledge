---
change: CHG-0009-harden-the-setup-fledge-composite-action-after-pr-511-review-validate-the-vers
artifact: testing
---

# Testing

## Automated (`.github/workflows/test-action.yml`)

- `install` matrix (`ubuntu-latest`/`macos-latest` × `v1.7.2`/`latest`, via
  `uses: ./`): unchanged from CHG-0008 except that outputs now reach the
  assertion step through `env:`. Guards the happy paths.
- `refuses-unsafe-install` (**new**): asserts that
  `version: ../../../octocat/Hello-World/releases/download/v1` fails
  (REQ-harden-action-1) and that `v0.5.0` fails rather than installing
  unverified (REQ-harden-action-3).
- `windows-unsupported`: unchanged in intent, renamed to "windows is refused
  with a readable message" so the job name no longer reads as "fledge has no
  Windows binary" (REQ-harden-action-5).
- `on:` narrowed to `push` on `main` + `pull_request` on `main`
  (REQ-harden-action-8).

## Manual verification (local, macOS arm64, real `CorvidLabs/fledge` releases)

The `run:` block was extracted byte-for-byte from `action.yml` via
`yaml.safe_load` and executed directly — not simulated — with `RUNNER_OS`,
`RUNNER_ARCH`, `GITHUB_PATH` and `GITHUB_OUTPUT` supplied, a fresh throwaway
`GITHUB_OUTPUT` per case.

**Happy paths — still work after the hardening**

1. Pinned `v1.7.2`: downloaded `fledge-macos-aarch64`, verified its real
   sidecar, installed, `fledge --version` → `fledge 1.7.2`,
   `outputs.version=v1.7.2`, `$GITHUB_PATH` written. exit 0.
2. `latest`, anonymous: resolved to `v1.7.2` via `jq`, installed. exit 0.
3. `latest`, real `gh auth token`: same. exit 0.

**Refusals — each exits 1 with an `::error::` and writes nothing to
`$GITHUB_OUTPUT`**

4. `version: ../../../octocat/Hello-World/releases/download/v1` → rejected
   before any network call (REQ-1).
5. `version: "v1.7.2; echo pwned"` → rejected (REQ-1).
6. `install-dir` containing `../` → rejected (REQ-2).
7. `install-dir` containing a newline → rejected (REQ-2). Without the guard
   this value would have appended a forged `version=` line to
   `$GITHUB_OUTPUT`.
8. `version: v0.5.0` → sidecar fetch 404, install refused: "no sidecar is
   published there — releases before v0.9.1 predate them" (REQ-3).
9. `RUNNER_OS=Windows` → refused before any release-asset request, message
   names the platform, says "yet", and points at
   `fledge-windows-x86_64.exe` (REQ-5).
10. `RUNNER_ARCH=RISCV` → refused, same shape.
11. `latest` + an invalid token → "the token was rejected or its rate limit is
    exhausted (HTTP 401)". Also proves the Bearer header from the stdin config
    file is genuinely sent: an anonymous request would have succeeded (REQ-6).

**Checksum branches unreachable by input alone** — driven by `sed`-patching a
*copy* of the extracted script; the committed script was not modified:

12. Mismatch (binary fetched from one asset, sidecar from another) → rejected,
    both digests printed (REQ-3).
13. Malformed sidecar (sidecar URL pointed at a 200 response that is not a
    digest) → rejected by the 64-hex-char guard (REQ-3).

**Supporting checks**

14. `shellcheck -s bash` on the extracted script: clean, exit 0.
15. `yaml.safe_load` on `action.yml` and `test-action.yml`: both parse.
16. Temp-dir leak check: stray `mktemp -d` count identical before and after
    each failing run — the `trap ... EXIT` fires on the failure paths
    (REQ-7).
17. curl semantics verified directly rather than assumed — see research.md
    §3 (REQ-4, REQ-6).

## Rejection signal

If any input value or network outcome results in an installed binary whose
checksum was not verified; if a `version` containing `..` reaches a URL; if
the token appears in argv; if a failed `latest` lookup reports no HTTP status;
if a failing run leaves its temp directory behind; or if README.md again
promises a guarantee the script does not keep — the change is wrong.
