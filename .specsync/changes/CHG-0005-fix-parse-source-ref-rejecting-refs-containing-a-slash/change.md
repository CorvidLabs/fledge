---
id: CHG-0005-fix-parse-source-ref-rejecting-refs-containing-a-slash
state: accepted
type: bug_fix
base_commit: 13855367dfb7a019425bdb063f42712d0e79ecd5
---

# Fix parse_source_ref rejecting refs containing a slash

## Intent

Fix parse_source_ref rejecting refs containing a slash

## Affected Canonical Specs

- `trust`

## Acceptance Criteria

- parse_source_ref splits a trailing @ref containing '/' (e.g. a branch name) into base and ref for both bare owner/repo and full URL forms, including credentialed URLs; credential URLs with no ref still refuse to split

## No-spec Rationale

Not applicable
