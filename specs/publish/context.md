# Publish — Context

## Problem

Publishing fledge artifacts (templates, lanes, plugins) to GitHub requires multiple manual steps: create a repo, add the right topic for discovery, set description, push files. Each artifact type wants the same plumbing but with a different topic and validation gate.

## Solution

`src/publish.rs` is a library of helpers — `get_authenticated_user`, `check_repo_exists`, `create_github_repo`, `set_repo_topic`, `push_directory` — consumed by the user-facing publish surfaces in their respective callers (`fledge templates publish` in `main.rs`, `fledge lanes publish` in `lanes.rs`, `fledge plugins publish` in `plugin.rs`). Each caller drives the flow with the topic and validation that suit it.

## Design Decisions

- **Library, not subcommand**: As of v3, this module exposes no `run` or `PublishOptions` — keeps validation and prompting close to where the artifact-specific knowledge lives
- **GitHub REST API via ureq**, consistent with `search.rs` (no async runtime)
- **Token via env, not persisted git remote**: `push_directory` uses an in-memory `http.extraheader` injection so the token never lands in `.git/config`. The remote URL is reset to a clean form after the push.
- **Topic add is additive**: `set_repo_topic` merges the new topic into the existing topic set rather than replacing the whole list
- **Caller drives validation**: e.g. `fledge templates publish` calls `validate::run` before publishing; `lanes publish` calls `validate_lanes`
- Uses git CLI for push operations (already a dependency via remote module)
- Caller prompts for confirmation when updating an existing repo (destructive)
