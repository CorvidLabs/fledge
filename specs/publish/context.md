# Publish — Context

## Problem

Publishing a fledge template to GitHub requires multiple manual steps: create a repo, add the `fledge-template` topic, set description, push files. This friction discourages template sharing.

## Solution

A single `fledge publish` command that validates the template, creates/updates the GitHub repo with proper metadata, and pushes all files. Completes the create → publish → discover lifecycle alongside `fledge create-template` and `fledge search`.

## Design Decisions

- Uses GitHub REST API (v3) via ureq, consistent with the search module
- Template name from template.toml becomes the repo name (predictable, searchable)
- Always adds `fledge-template` topic for discoverability via `fledge search`
- Uses git CLI for push operations (already a dependency via remote module)
- Prompts for confirmation when updating an existing repo (destructive operation)
