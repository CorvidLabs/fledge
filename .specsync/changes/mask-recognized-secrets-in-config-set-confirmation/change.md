---
id: mask-recognized-secrets-in-config-set-confirmation
state: draft
type: bug_fix
base_commit: 85d7c4efbca79947afa359504b095c95c486b391
---

# Mask recognized secrets in config set confirmation

## Intent

Mask recognized secrets in config set confirmation

## Affected Canonical Specs

- `main`

## Acceptance Criteria

- For every existing Config::is_secret_key key successful config set persists the exact supplied value and confirms with the key plus *** while omitting the supplied secret from stdout and stderr.
- Non-secret defaults.author still confirms and persists its plaintext value.
- Unknown config keys fail without echoing the supplied value or creating the owned config file.
- Existing config get and currently rendered config list secret values stay masked; the absent deprecated Claude list row is not claimed.
- The same actual CLI regression changes from five secret-output failures and two passing controls on base to seven passing cases on the candidate.

## No-spec Rationale

Not applicable
