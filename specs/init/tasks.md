---
spec: init.spec.md
---

## Tasks

- [x] Implement core init flow: resolve template → prompt variables → render → git init → hooks → summary
- [x] Add template selection by name (`--template`)
- [x] Add interactive template selector when no template specified
- [x] Add remote template support (`--template owner/repo`)
- [x] Add remote template collection support (repos with multiple templates)
- [x] Add `--dry-run` flag with preview output
- [x] Add `--no-git` flag to skip git initialization
- [x] Add `--no-install` flag to skip post-create hooks
- [x] Add `--yes` flag to auto-confirm remote hooks
- [x] Add `--refresh` flag to force re-fetch remote templates
- [x] Add hook security: prompt for confirmation on remote template hooks
- [x] Handle CI environments (auto-configure git user if missing)
- [x] Unit tests for template resolution, git init, hook execution

## Gaps

- No progress indicator for remote template downloads
- No template compatibility checking (min_fledge_version is parsed but not enforced)

## Review Sign-offs

- **Product**: done
- **QA**: done
- **Design**: n/a
- **Dev**: done
