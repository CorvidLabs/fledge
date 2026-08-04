---
change: CHG-0005-fix-parse-source-ref-rejecting-refs-containing-a-slash
artifact: context
---

# Context

`parse_source_ref` splits a `source@ref` string into a base and an optional git ref, but
`src/trust.rs` refused the split whenever the ref portion contained `/`. This guard exists
to avoid misparsing a credential URL (`https://user:pass@host/path`) as `repo` + ref
`host/path`, but it overshoots: it also rejects legitimate branch refs containing `/`,
which is a common naming convention (`chore/...`, `feature/...`, `docs/...`). A source like
`owner/repo@chore/0.2.0-launch-prep` was left unsplit, and the whole string — ref included
— got globbed into the clone URL, producing a malformed URL and a failed install.

The fix distinguishes the two cases by position rather than by whether the ref contains a
slash: a credential `@` always sits inside the URL's authority component (before any path
separator following the scheme); a trailing ref `@` sits after the full base is already
formed. Checking for a `/` between the scheme and the split point (rather than in the ref
itself) correctly rejects only genuine credential URLs while accepting slash-containing
refs everywhere else, including full clone URLs with embedded credentials plus a trailing
ref.
