# Search — Context

## Problem

Multiple parts of fledge need to discover community-tagged GitHub repos: templates (`fledge-template` topic), lanes (`fledge-lane`), and plugins (`fledge-plugin`). Each surface has slightly different display and filtering needs, but the underlying mechanism — query the GitHub Search API by topic + optional keyword/author, parse the response, format results — is the same.

## Solution

Keep the GitHub-topic-search mechanism in `src/search.rs` as a library of helpers (`build_search_query_ex`, `parse_search_response`, `format_stars`, `urlencod`). The user-facing surfaces live in their respective callers — `fledge templates search`, `fledge lanes search`, `fledge plugins search` — each wiring the helpers to a different topic and rendering results in its own way.

## Design Decisions

- **Library, not subcommand**: As of v3, this module exposes no `run` or `SearchOptions` — the user-facing commands (e.g. `fledge templates search`) live in their respective callers (`main.rs`, `lanes.rs`, `plugin.rs`) and call the helpers directly. This split keeps the topic-specific behavior (display strings, post-filters, trust-tier computation) close to where it's used.
- **GitHub topics over a central registry**: avoids maintaining state; template/lane/plugin authors just add the relevant topic to their repo
- **ureq over reqwest**: codebase is fully synchronous; `ureq` is a blocking client that avoids pulling in an async runtime
- **Stars as default sort**: most-starred repos surface community-validated entries first
- **Works without auth**: unauthenticated GitHub search allows 10 requests/minute (sufficient for interactive CLI use); a configured token raises this to 30/minute

## Prior Art

- `cargo search` — searches crates.io
- `npm search` — searches npm registry
- `gh search repos` — searches GitHub
