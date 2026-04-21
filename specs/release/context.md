# Release — Context

## Problem

Releasing a project involves multiple manual steps: bumping version numbers in one or more files, generating a changelog, creating a git tag, and pushing. Each language ecosystem has its own version file format, and it's easy to forget a step or tag the wrong version.

## Solution

A single `fledge release` command that handles the entire workflow. It detects the project language, bumps all relevant version files, generates a changelog from conventional commits, creates an annotated tag, and optionally pushes. Supports 8+ language ecosystems and falls back to tag-only releases for languages that use git tags as their version source (Go, Swift).
