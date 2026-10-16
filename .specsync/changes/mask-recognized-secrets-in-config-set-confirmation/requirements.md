---
change: mask-recognized-secrets-in-config-set-confirmation
artifact: requirements
---

# Requirements

## Existing basis

Existing specs/config/requirements.md forbids token output in CLI output or errors. REQ-config-003 requires scalar Set persistence. Existing hi/setup.md SETUP-2.g requires masked printed API keys. This proposal adds no hi criteria or secret-key classification.

## Proposed observable contract

Successful config set SHALL save the supplied value before confirming success. For every key already recognized by Config::is_secret_key, its confirmation SHALL print the key and *** instead of the supplied secret. The synthetic supplied marker SHALL be absent from both output streams.

Non-secret defaults.author SHALL remain visible and persist unchanged. Unknown keys SHALL fail without echoing their supplied value or writing the owned config file. Get and currently rendered List secret values SHALL remain masked. Deprecated ai.claude.api_key currently has no List row; no new row is proposed.

The five explicit native criteria record these outcomes and actual before/after sensitivity. They are agent-proposed definition contents, not human approval.

## Limits

No encrypted storage, secure input, process-argument or shell-history protection, schema/path change, new key classes, dependency upgrade or broad credential audit is proposed. Unrelated error paths are not certified.
