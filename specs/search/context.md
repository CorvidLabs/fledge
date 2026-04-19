# Search — Context

## Problem

Users have no way to discover community templates. They must manually find repos, copy URLs, and add them to config. This makes the template ecosystem hard to grow.

## Solution

Add `fledge search` command that queries the GitHub Search API for repos with the `fledge-template` topic. Results show the owner/repo reference that can be passed directly to `fledge init -t owner/repo`.

## Design Decisions

- **GitHub topics over registry**: Using GitHub topics (`fledge-template`) as the discovery mechanism avoids maintaining a central registry. Template authors just add the topic to their repo.
- **ureq over reqwest**: The codebase is fully synchronous. `ureq` is a blocking HTTP client that avoids pulling in an async runtime.
- **Stars as default sort**: Most-starred repos surface community-validated templates first.
- **Works without auth**: Unauthenticated GitHub search API allows 10 requests/minute. Sufficient for interactive CLI use. Token raises this to 30/minute.

## Prior Art

- `cargo search` — searches crates.io
- `npm search` — searches npm registry
- `gh search repos` — searches GitHub
