---
change: CHG-0008-add-root-level-action-yml-github-action-for-installing-fledge-in-ci
artifact: design
---

# Design

## Shape

A single-file composite action (`runs.using: composite`), one shell step,
matching `spec-sync`'s own root-level `action.yml` precedent. No JS bundle,
no Docker image, no dependency beyond `curl`/`bash`/`sha256sum`-or-`shasum`,
all already present on GitHub-hosted Linux and macOS runners.

## Control flow (single `install` step, `set -euo pipefail`)

```
guard RUNNER_OS  (Linux|macOS, else ::error:: + exit 1)
guard RUNNER_ARCH (X64|ARM64, else ::error:: + exit 1)
        │
        ▼
version == "latest"?
  yes → GET api.github.com/.../releases/latest, with Bearer token if set
        (Accept + X-GitHub-Api-Version headers; --retry-all-errors)
        empty result → ::error:: + exit 1
  no  → version is used as-is (no network call yet)
        │
        ▼
curl -fL the release asset directly from github.com/.../releases/download/
  fails → ::error:: naming the exact URL + exit 1
        │
        ▼
curl the "<asset>.sha256" sidecar
  found    → compare against sha256sum/shasum of the downloaded file
             mismatch → ::error:: + exit 1
  missing  → ::warning:: (old release, pre-checksums) — do NOT fail
        │
        ▼
install -m 0755 into install-dir; append install-dir to $GITHUB_PATH;
write version/path to $GITHUB_OUTPUT
```

A second step (`fledge --version`) runs after PATH is updated, as the smoke
check — kept as its own step (not folded into `install`) so its failure is
visually distinct in the Actions log from an install failure.

## Inputs / outputs surface

| Input         | Default            | Notes                                                   |
|---------------|---------------------|----------------------------------------------------------|
| `version`     | `latest`            | Pin this in CI — see REQ-setup-action-2                  |
| `token`       | `${{ github.token }}` | Only read when `version: latest`                        |
| `install-dir` | `` (→ `$HOME/.local/bin`) | Avoids needing `sudo` on either runner OS          |

| Output    | Source                                    |
|-----------|--------------------------------------------|
| `version` | The resolved tag (pinned value, or the API result for `latest`) |
| `path`    | `<install-dir>/fledge`                     |

## Why no jq / no Node

Matches `install.sh`'s existing `grep '"tag_name"' | cut -d'"' -f4` approach
exactly — same dependency footprint, same failure mode already field-tested
by that script, no new external dependency for a composite action to assume
is present on every consumer's runner image.

## Exercising workflow shape

`test-action.yml` has two jobs, not one: the happy-path matrix
(`install`) and a dedicated `windows-unsupported` regression guard using
`continue-on-error: true` + an explicit outcome check, rather than baking a
"this is expected to fail" branch into the matrix itself — keeps the
happy-path matrix's pass/fail meaning unambiguous (green means "it worked"
everywhere in that job), while still running the negative case on every push.

## Alternatives considered

- **JS/TypeScript action**: rejected — needs a build step and a committed
  `dist/`, which is exactly the kind of tooling fledge itself avoids
  (AGENTS.md: "plain HTTP ... no CLI to install").
  `spec-sync`'s own composite action is the closer precedent to follow.
- **Reusing `install.sh` via `curl | bash` from inside the composite
  action**: rejected as the implementation strategy — it would just move the
  exact rate-limiting bug this change exists to fix one layer down, and
  `install.sh` has no checksum verification at all (confirmed by reading it
  in full). The action's script intentionally diverges from `install.sh`
  where `install.sh` is weaker (adds checksum verification, adds the
  authenticated-latest path) while keeping the same asset-naming and
  tag-resolution conventions.
