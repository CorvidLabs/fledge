---
change: publish-to-crates-io-from-the-release-workflow-and-verify-the-registry
artifact: tasks
---

# Tasks

- [x] Confirm the gap: crates.io at 1.7.0, `v1.7.1` and `v1.7.2` both tagged
- [x] Establish the cause — read the Release runs for both tags (green), and confirm no
      `cargo publish` exists in any workflow
- [x] Confirm publishing is undocumented in `AGENTS.md`, `CONTRIBUTING.md` and `docs/`
- [x] Add a `publish` job to `release.yml`, gated after `release`, failing loudly when
      `CARGO_REGISTRY_TOKEN` is absent
- [x] Add the registry read-back with retry, comparing `max_version` to the tag
- [x] Document both in `AGENTS.md`, including the manual-publish check
- [x] Verify the workflow still parses and the job graph is correct
