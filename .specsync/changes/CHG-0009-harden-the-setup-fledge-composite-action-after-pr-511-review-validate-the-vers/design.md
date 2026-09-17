---
change: CHG-0009-harden-the-setup-fledge-composite-action-after-pr-511-review-validate-the-vers
artifact: design
---

# Design

The shape from CHG-0008 is unchanged: one composite action, one shell step
plus a smoke-check step, `set -euo pipefail`, no JS or Docker. This change
adds a validation prologue, hardens the checksum stage, and corrects two
messages. `jq` joins `curl`/`bash`/`sha256sum`-or-`shasum` as an assumed
dependency — preinstalled on every GitHub-hosted runner.

## Revised control flow

```
validate version    (latest | ^v?X.Y.Z(-|+suffix)?$, else ::error:: + exit 1)  ← new
validate install-dir (no `..` segment, no line break, else ::error:: + exit 1) ← new
        │
        ▼
guard RUNNER_OS  (Linux|macOS, else ::error:: + exit 1)   ← message corrected
guard RUNNER_ARCH (X64|ARM64, else ::error:: + exit 1)
        │
        ▼
mktemp -d + trap 'rm -rf "$tmp"' EXIT                     ← new
        │
        ▼
version == "latest"?
  yes → GET api.github.com/.../releases/latest
        curl --config - from stdin, carrying the Bearer header   ← was argv
        capture %{http_code}; non-200 → ::error:: naming the status  ← was `|| true`
        jq -r '.tag_name'                                        ← was grep|cut
        re-validate the resolved tag                             ← new
  no  → version is used as-is (no network call)
        │
        ▼
curl -fL the release asset  (--retry 3 --retry-all-errors)
        │
        ▼
curl the sidecar (--retry 3, NO --retry-all-errors), capture %{http_code}
  non-200 → ::error:: + exit 1        ← was ::warning:: + continue
  not 64 hex chars → ::error:: + exit 1   ← new
  mismatch → ::error:: + exit 1
        │
        ▼
install -m 0755; append to $GITHUB_PATH; write $GITHUB_OUTPUT
```

## Why an allowlist rather than escaping or normalizing

The value is not quoted into a shell command — it is interpolated into a URL,
and the normalization happens inside curl, on the client, before the request
leaves the runner. There is nothing to escape: `%2e%2e` would not help a
consumer who wants a legitimate tag, and rejecting only `..` would leave the
input free to point at any other path shape github.com happens to serve. A
tag-shaped allowlist is the only form that says what the input is *for*. The
same regex is reapplied to the API-resolved tag so both entry points converge
on one rule.

`install-dir` gets a narrower guard rather than the same allowlist, because a
directory path legitimately has no fixed shape. Two concrete hazards are
closed: a `..` segment writing a 0755 binary outside the intended tree, and a
line break injecting extra lines into `$GITHUB_PATH`/`$GITHUB_OUTPUT` (an
`install-dir` of `ok\nversion=fake` would otherwise forge a step output).

## Why the checksum skip is removed rather than narrowed

A skip branch that fires on a failed fetch is indistinguishable from a skip
branch that fires on an attack: whoever can substitute the binary can also
make the sidecar request fail, and a `::warning::` does not fail the build.
Narrowing it — 404-only, or below a version floor — keeps the unverified-install
code path alive to serve v0.3.0–v0.5.0, three pre-1.0 releases. Removing it
costs those three releases and nothing else (see research.md). The status code
is still captured, but only to make the error accurate: a 404 says "no sidecar
is published there — releases before v0.9.1 predate them", anything else
reports the status and curl's exit code.

`--retry 3` on the sidecar fetch is deliberately *without* `--retry-all-errors`:
`--retry` alone covers the transient statuses (408, 429, 5xx) but not 404, so
a genuinely absent sidecar returns its verdict in ~0.3s instead of after the
full backoff. The binary download keeps `--retry-all-errors`, where a 404 is
fatal anyway.

## Why `-w '%{http_code}'` with `-f`, not one or the other

`-f` alone gives an exit code but no status, so 401, 403, 429 and 500 are
indistinguishable — which is the diagnostic CHG-0008 exists to provide, lost.
`-w` alone would make curl treat a 404 body as a successful transfer. Together,
curl still writes the status to stdout when `--fail` aborts it (verified: exit
22/56 with `404` on stdout; a DNS failure gives `000`), so both signals are
available and the error can name the real cause.

## Test workflow

Adds a third job, `refuses-unsafe-install`, holding both negative cases in one
runner: the traversal `version`, and `v0.5.0` as a live sidecar-less fixture.
Using a real release rather than a mock means the missing-sidecar path is
exercised against production data. Both use `continue-on-error: true` and an
explicit outcome assertion, so the happy-path matrix keeps its unambiguous
"green means it worked" meaning.

`on: push` is narrowed to `main`; combined with `pull_request` to `main`, a
branch push now produces one run instead of two. Step outcomes and outputs are
passed into assertion steps through `env:` rather than interpolated into the
`run:` block, matching how the action itself already handles its inputs.

## Deferred: Windows support

The asset exists, so this is genuinely available — but it needs `.exe`
handling, `install` vs `cp`+`chmod` under Git Bash, and `cygpath -w` for
`$GITHUB_PATH` and `outputs.path`, because an MSYS-style `/c/...` entry is not
usable from `pwsh` steps and would half-work in a way that is worse than a
clean refusal. Left as an additive follow-up; this change makes the refusal
message tell the truth in the meantime.
