# Search — Requirements

This module is a library — its requirements are about the helpers it exposes. The user-facing search commands (`templates search`, `lanes search`, `plugins search`) define their own requirements in their respective specs.

## Functional Requirements

1. **FR-1**: Build a GitHub Search query string from `(topic, optional keyword, optional author)` — works for any topic, not just `fledge-template`
2. **FR-2**: Parse a GitHub Search API response into a typed `Vec<SearchResult>` with `(owner, name, description, stars, url, topics)`
3. **FR-3**: Tolerate missing fields in the API response (description → "No description", stars → 0, topics → empty)
4. **FR-4**: Format star counts compactly (`42`, `1.5k`, `123k`)
5. **FR-5**: Percent-encode strings for URL query parameters per RFC 3986
6. **FR-6**: Skip items with no `owner.login` rather than failing the whole parse

## Non-Functional Requirements

1. **NFR-1**: No async runtime dependency — callers use the blocking `ureq` client
2. **NFR-2**: No `serde` round-trip for the API response — direct `serde_json::Value` indexing keeps the dependency surface minimal
