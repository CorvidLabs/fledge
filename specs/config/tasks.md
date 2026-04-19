---
spec: config.spec.md
---

## Tasks

- [x] Define Config, Defaults, TemplatesConfig, GitHubConfig structs
- [x] Implement load/save with TOML serialization
- [x] Implement get/set/unset for scalar keys
- [x] Implement add_to_list/remove_from_list for templates.paths and templates.repos
- [x] Add author_or_git() fallback to git config
- [x] Add github_token() with env var precedence
- [x] Add tilde expansion for extra_template_paths
- [x] Add is_valid_key() validation
- [x] Wire up CLI subcommands (list, get, set, unset, add, remove)
- [x] Add error guidance when using wrong verb for key type
- [x] Unit tests for all public methods

## Gaps

- No validation that template paths actually exist when added
- No config file locking for concurrent access

## Review Sign-offs

- **Product**: done
- **QA**: done
- **Design**: n/a
- **Dev**: done
