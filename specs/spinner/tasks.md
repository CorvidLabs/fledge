---
spec: spinner.spec.md
---

## Tasks

- [x] Define Theme struct with frames and interval
- [x] Create 10 spinner themes (5 emoji, 5 Unicode/ASCII)
- [x] Implement platform-aware random_index (Unix /dev/urandom, fallback SystemTime)
- [x] Implement Spinner::start with random theme selection
- [x] Implement Spinner::finish with line clearing
- [x] Integrate spinner into all long-running commands
- [x] Write unit tests for random_index bounds and theme invariants
- [x] Write spec

## Gaps

- No fallback for terminals that don't support Unicode/emoji (themes assume UTF-8)
- No way to disable spinners (e.g. for piped output / non-TTY)

## Review Sign-offs

- **Product**: done
- **QA**: done
- **Design**: n/a
- **Dev**: done
