---
change: CHG-0008-add-root-level-action-yml-github-action-for-installing-fledge-in-ci
artifact: context
---

# Context

`CorvidLabs/fledge` has no `action.yml` on any branch, so `uses:
CorvidLabs/fledge@v1.7.2` fails with "Can't find 'action.yml'". Sibling repo
`CorvidLabs/spec-sync` ships one at its root, which is why `uses:
CorvidLabs/spec-sync@v5.2.0` already works elsewhere — fledge is the odd one
out.

Consumers currently fall back to curl-piping `install.sh`, whose
`latest_version()` scrapes the unauthenticated
`api.github.com/repos/CorvidLabs/fledge/releases/latest` endpoint. That
endpoint is rate-limited per IP, and GitHub-hosted runners share IPs.
`CorvidLabs/rune` calls it from seven jobs on every push and lost four CI runs
to `could not determine latest version` — on Ruby 3.3, Ruby 3.2, and twice on
Spec Sync — each one indistinguishable from a real failure until someone read
the log. rune has since pinned a direct release-asset download as a
workaround; this change is the real fix, so everyone else gets the same
guarantee without hand-rolling it.

## Design decisions

- **Composite, not JS/Docker.** No build step, no bundling, matches the
  project's own philosophy (plain HTTP, no CLI-to-install) and `spec-sync`'s
  precedent.
- **Pinning makes zero API calls.** A concrete `version` (e.g. `v1.7.2`)
  downloads `https://github.com/CorvidLabs/fledge/releases/download/...`
  directly — that's a release-asset CDN redirect, not `api.github.com`, so it
  isn't subject to the same rate limit at all. Only `version: latest` calls
  the API, and it always attaches `github.token` (or a caller-supplied
  `token`) so that call lands in the authenticated 5000/hour bucket instead
  of the unauthenticated 60/hour-per-IP one.
- **Linux and macOS only, by design, not by omission.** The real v1.7.2
  release actually publishes five binaries including
  `fledge-windows-x86_64.exe` (confirmed via `gh release view`), but this
  action intentionally supports only `fledge-{linux,macos}-{x86_64,aarch64}`
  for its first version — the task scope explicitly calls out Windows as
  unsupported ("Windows fails with a readable message" is a listed
  acceptance criterion, not a bug to fix). Windows support can be a later,
  additive change; the `windows-unsupported` job in `test-action.yml` pins
  down the current contract so a future change has to touch it deliberately.
- **Checksum verification degrades gracefully.** Every current release ships
  an `<asset>.sha256` sidecar; a missing sidecar (a hypothetical release that
  predates them) warns and skips rather than hard-failing, so the action
  doesn't retroactively break old tags.
- **`uses: ./` in the exercising workflow**, not a published ref — the
  workflow must test the code actually on the branch/PR, not whatever `v1`
  last pointed at.

## No spec module

No canonical spec module governs this: `.specsync/config.toml` scans only
`source_dirs = ["src", "templates"]`, and the closest existing specs
(`specs/release/`, `specs/github/`) cover `fledge release`'s internals and
fledge's own GitHub API helper module respectively — neither is about the
repo's own CI surface. `.specsync/sdd.json` does list `action.yml` and
`.github/` under `meaningful_paths`, which is why this still goes through the
verified change lifecycle even without a spec to update.
