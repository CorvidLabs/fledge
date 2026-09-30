---
id: publish-to-crates-io-from-the-release-workflow-and-verify-the-registry
state: archived
type: bug_fix
base_commit: 31d5f45f7bca1b71e374e41792fabeb6359a9de4
---

# Publish to crates.io from the release workflow and verify the registry

## Intent

Publish to crates.io from the release workflow and verify the registry

## Affected Canonical Specs

- None

## Acceptance Criteria

- release.yml carries a publish job that runs cargo publish on a v* tag after the release job, fails loudly with an actionable message when CARGO_REGISTRY_TOKEN is absent rather than skipping, and then reads https://crates.io/api/v1/crates/fledge back and compares max_version to the tag, retrying while the index catches up; AGENTS.md documents both the job and the read-the-registry-back rule; the workflow remains valid YAML with jobs test, build, release, publish

## No-spec Rationale

release.yml and AGENTS.md are outside source_dirs (src, templates); no canonical spec covers CI workflows or agent docs
