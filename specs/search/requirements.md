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

## Durable Requirements

### REQ-search-001

The implementation SHALL satisfy the following criterion: **FR-1**: Build a GitHub Search query string from `(topic, optional keyword, optional author)` — works for any topic, not just `fledge-template`

Acceptance Criteria

- **FR-1**: Build a GitHub Search query string from `(topic, optional keyword, optional author)` — works for any topic, not just `fledge-template`

### REQ-search-002

The implementation SHALL satisfy the following criterion: **FR-2**: Parse a GitHub Search API response into a typed `Vec<SearchResult>` with `(owner, name, description, stars, url, topics)`

Acceptance Criteria

- **FR-2**: Parse a GitHub Search API response into a typed `Vec<SearchResult>` with `(owner, name, description, stars, url, topics)`

### REQ-search-003

The implementation SHALL satisfy the following criterion: **FR-3**: Tolerate missing fields in the API response (description → "No description", stars → 0, topics → empty)

Acceptance Criteria

- **FR-3**: Tolerate missing fields in the API response (description → "No description", stars → 0, topics → empty)

### REQ-search-004

The implementation SHALL satisfy the following criterion: **FR-4**: Format star counts compactly (`42`, `1.5k`, `123k`)

Acceptance Criteria

- **FR-4**: Format star counts compactly (`42`, `1.5k`, `123k`)

### REQ-search-005

The implementation SHALL satisfy the following criterion: **FR-5**: Percent-encode strings for URL query parameters per RFC 3986

Acceptance Criteria

- **FR-5**: Percent-encode strings for URL query parameters per RFC 3986

### REQ-search-006

The implementation SHALL satisfy the following criterion: **FR-6**: Skip items with no `owner.login` rather than failing the whole parse

Acceptance Criteria

- **FR-6**: Skip items with no `owner.login` rather than failing the whole parse

### REQ-search-007

The implementation SHALL satisfy the following criterion: **NFR-1**: No async runtime dependency — callers use the blocking `ureq` client

Acceptance Criteria

- **NFR-1**: No async runtime dependency — callers use the blocking `ureq` client

### REQ-search-008

The implementation SHALL satisfy the following criterion: **NFR-2**: No `serde` round-trip for the API response — direct `serde_json::Value` indexing keeps the dependency surface minimal

Acceptance Criteria

- **NFR-2**: No `serde` round-trip for the API response — direct `serde_json::Value` indexing keeps the dependency surface minimal
