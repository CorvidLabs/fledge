---
change: CHG-0009-harden-the-setup-fledge-composite-action-after-pr-511-review-validate-the-vers
artifact: context
---

# Context

CHG-0008 added the root-level `action.yml` composite action. PR #511 review
(0xGaspar, `CHANGES_REQUESTED`) found two blockers, one factual error in the
error text and docs, and four nits. This change is the response. It amends the
behavior CHG-0008 specified — CHG-0008's own record is frozen at what was
accepted then, which is why this is a separate workspace rather than an edit
to it.

## What the review found

1. **`version` was unvalidated and reached a URL curl path-normalizes.** The
   input is interpolated into
   `https://github.com/CorvidLabs/fledge/releases/download/${version}/...`,
   and curl resolves dot segments client-side before sending. Verified against
   real github.com: requesting `.../releases/download/../../../octocat/Hello-World/x`
   sends `GET /CorvidLabs/octocat/Hello-World/x`. With one more `../`, a
   `version` of `../../../../attacker/repo/releases/download/v1` resolves to
   `/attacker/repo/releases/download/v1/fledge-linux-x86_64`, which the action
   then `install -m 0755`es and executes via its own `fledge --version` smoke
   check.

2. **Checksum verification silently degraded to none.** The sidecar fetch's
   `else` branch emitted a `::warning::` and continued, on the theory that the
   release predated sidecars. That branch could not distinguish a genuine
   absence from a request that was made to fail, and a `::warning::` does not
   fail a build — so anyone able to tamper with the binary could also suppress
   its verification. README.md meanwhile claimed "every download is
   checksum-verified", a guarantee the code did not deliver.

3. **The Windows message implied no Windows binary exists.** It read "fledge
   publishes no binary for Windows via this action". Every release ships
   `fledge-windows-x86_64.exe` and its sidecar. Combined with the test job
   named "windows fails with a readable message" and the README's platform
   list, a reader would reasonably conclude the binary is unavailable.

4. Nits: fragile `grep`/`cut` JSON parsing with a `|| true` that collapsed
   auth failure, rate limiting and network loss into one message; the Bearer
   token in argv; `mktemp -d` leaking on failure paths; an unfiltered
   `on: push` giving every branch push two workflow runs.

## Design decisions

- **Untrusted input stops at the action boundary, not in the consumer's
  workflow.** The traversal only bites a consumer who feeds untrusted data
  into `version`, but `pull_request_target` keyed off a branch name, label, or
  PR title is the standard way that happens, and this is a public action
  inviting third-party use. A tag-shaped allowlist costs one `grep -Eq`.

- **No skip path at all, rather than a smarter skip path.** The review offered
  three options: capture the status and treat 404 as legitimate, gate the skip
  on a version floor, or add an `allow-unverified` input. All three keep a
  code path that installs unverified binaries. A survey of all 39 releases
  shows that path protects nothing: sidecars are universal from **v0.9.1**
  onward, v0.6.0–v0.9.0 publish no assets at all, and only v0.3.0–v0.5.0 have
  binaries without them. The status is still captured, but only so the error
  can say *why* — a 404 is reported as "no sidecar published there".

- **Windows: fix the message, defer the support.** Wiring Windows up needs
  `.exe` handling plus `cygpath` translation for `$GITHUB_PATH` and
  `outputs.path`, since an MSYS-style `/c/...` entry is not usable from `pwsh`
  steps. That is a platform's worth of new surface and is better done
  deliberately as its own change. What was actually wrong here was the claim,
  so the claim is what this change fixes, in the error text and in README.md.

- **`jq` is fine inside an action.** CHG-0008 chose `grep '"tag_name"' | cut`
  to mirror `install.sh`'s dependency footprint. `jq` is preinstalled on every
  GitHub-hosted runner, so that argument does not apply here, and the string
  slicing needed a `|| true` that destroyed exactly the diagnostic this action
  exists to provide.

## No spec module

Unchanged from CHG-0008: `.specsync/config.toml` scans only
`source_dirs = ["src", "templates"]`, and no canonical spec covers the repo's
own CI/distribution surface. `.specsync/sdd.json` lists `action.yml` and
`.github/` under `meaningful_paths`, which is why this still goes through the
verified change lifecycle.

The `public_contract` interview answer is `no`, matching CHG-0008, and that is
narrower than it may read. The *action's* observable behavior does change — a
`version` value that used to be passed through is now rejected, and a
sidecar-less release that used to install with a warning now fails. But
`public_contract` in this interview asks whether a **canonical spec contract**
moves, and there is no spec module owning this surface for one to move. The
behavior changes are captured in requirements (REQ-harden-action-1 and -3),
in the README, and in this record; `no_spec_change` is the accurate answer to
the question actually being asked, not a claim that nothing observable
changed.
