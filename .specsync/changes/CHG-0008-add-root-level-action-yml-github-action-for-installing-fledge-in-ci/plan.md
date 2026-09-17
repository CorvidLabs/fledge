---
change: CHG-0008-add-root-level-action-yml-github-action-for-installing-fledge-in-ci
artifact: plan
---

# Plan

1. Write `action.yml` at the repo root: composite action, single `install`
   step (OS/arch guard → version resolution → download → checksum verify →
   install → PATH/outputs) plus a `fledge --version` smoke-check step.
2. Write `.github/workflows/test-action.yml`: a matrix job over
   `{ubuntu-latest, macos-latest} x {v1.7.2, latest}` using `uses: ./` (not a
   published ref, so it always tests the current branch), plus a
   `windows-unsupported` job that asserts the action fails cleanly on
   `windows-latest`.
3. Add a `## GitHub Actions` section to `README.md`, right after `## Install`,
   showing the pinned form first per the task's requirement.
4. Add a "Moving the `v1` tag" subsection to `CONTRIBUTING.md`'s Release
   Process, documenting `git tag -f v1 v<version> && git push origin v1
   --force` as a manual post-release step.
5. Locally verify the checksum-mismatch and checksum-missing branches of the
   install script in isolation (can't corrupt a real GitHub release asset to
   test this end-to-end in CI) — see testing.md.
6. Run `fledge lanes run check` (fmt + lint + test); confirm `specsync check`
   is unaffected (no `src/`/`templates/` changes).
7. Take this through the change lifecycle: definition approval → implement →
   verify → present evidence → closing approval → accept → merge → archive.
8. Note for follow-up (not part of this change's diff): once merged, a
   maintainer creates and pushes the `v1` tag pointing at the merge commit,
   per the new CONTRIBUTING.md note. That's real repo history, `git tag`
   creation should be authorized explicitly. Also expect a `fledge review`
   pass on the diff before opening the PR.
