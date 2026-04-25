# Search — Tasks

- [x] Add `ureq` dependency to Cargo.toml
- [x] Create `src/search.rs` with the topic-search helpers (`build_search_query_ex`, `parse_search_response`, `format_stars`, `urlencod`)
- [x] Wire helpers into `templates`, `lanes`, and `plugins` callers
- [x] Add unit tests for JSON parsing, query building, star formatting, URL encoding
- [x] (v0.15) Strip the `run` / `SearchOptions` / `search_github_ex` user-facing surface — module is a library now
- [x] (v0.15.2) Re-expose templates flavor via `fledge templates search` (handler in `main.rs`, calls these helpers)
- [x] Verify with `cargo test`, `cargo clippy`, `cargo fmt`
