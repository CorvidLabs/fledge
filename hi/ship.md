---
hi: 1
families: [SHIP]
---

# Shipping

## Intent

Branching, committing, pushing and tagging are the boring parts of shipping, and they should be one-word commands that already follow this project's conventions without me holding them in my head. The guardrails matter more than the convenience: no branch started on top of uncommitted work, no push straight to the default branch, no release cut from a dirty tree, no tag written twice. At the end, the version bump, the changelog and the tag should be a single command rather than a ritual I half-remember.

## Criteria

- **SHIP-1**  Starting a piece of work gives me a branch named the way this project names branches.
  - **SHIP-1.a**  Whatever I type is cleaned up into a valid git branch name.
  - **SHIP-1.b**  I can tie the branch to an issue number.
  - **SHIP-1.c**  The project can define its own branch naming instead of accepting fledge's.
  - **SHIP-1.d**  Starting new work on top of uncommitted changes is refused.
- **SHIP-2**  My commits come out in the project's conventional format without me remembering the prefix.
  - **SHIP-2.a**  The type of change is taken from the branch I am on when I do not say.
  - **SHIP-2.b**  A message I already prefixed correctly is left exactly as I wrote it.
  - **SHIP-2.c**  I can have a model write the commit message from what I staged.
  - **SHIP-2.d**  Committing with nothing staged tells me so instead of making an empty commit.
- **SHIP-3**  Pushing sets up branch tracking so I never have to think about it.
  - **SHIP-3.a**  Pushing straight to the default branch is refused.
  - **SHIP-3.b**  A forced push never clobbers work somebody else pushed in the meantime.
- **SHIP-4**  I can see how far ahead or behind my branch is without leaving fledge.
  - **SHIP-4.a**  I can see how much of my work is still uncommitted.
  - **SHIP-4.b**  Checking where my branch stands works without reaching my git host.
- **SHIP-5**  Cutting a release bumps the version, writes the changelog and tags it in one command.
  - **SHIP-5.a**  The version is found wherever this language keeps it.
  - **SHIP-5.b**  A project that keeps no version file is released by tag alone.
  - **SHIP-5.c**  I can preview the whole release without a single file being touched.
  - **SHIP-5.d**  Releasing from a dirty working tree is refused unless I insist.
  - **SHIP-5.e**  I can have the project's check pipeline run before the release starts.
  - **SHIP-5.f**  A release stops if those checks fail.
  - **SHIP-5.g**  Writing a tag that already exists is refused.
  - **SHIP-5.h**  I can have the release pushed in the same command instead of remembering the tag myself.
- **SHIP-6**  I can read the changelog for any release straight out of the git history.
  - **SHIP-6.a**  I can see what has landed since the last tag.
  - **SHIP-6.b**  Commits are grouped by the kind of change they were.
  - **SHIP-6.c**  A commit that follows no convention still appears instead of being dropped.
