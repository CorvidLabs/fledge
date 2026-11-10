---
change: mask-recognized-secrets-in-config-set-confirmation
artifact: context
---

# Context

This is an agent-prepared UNAPPROVED proposal. It records neither human scope approval nor implementation acceptance, review or signature. It restores existing config secrecy intent and captures no new hi criteria.

On pinned base85d7c4efbca79947afa359504b095c95c486b391, ConfigAction::Set saved its input and printed the raw value, including all five keys already recognized by Config::is_secret_key. Get/List and the interactive secret editor already mask their rendered secret values.

The candidate owns only src/config_cmds.rs, tests/config_secret_output.rs, specs/main/main.spec.md and specs/main/testing.md. It changes the confirmation boundary and preserves exact persistence. PR538's dependency upgrade and PR539/540 scope remain separate. Actual seven-case baseline failed five secret cases and passed two controls; the same target passes all seven after the fix.

Main canonical candidate is already version17 while the pinned base is16. Native workflowv2 creation/answers initially left deltas/ empty. Metadata preparation does not reset canonical bytes, force an additional version or claim materialization. The coordinator must inspect and reconcile the actual delta/version state before later authorized lifecycle advance.
