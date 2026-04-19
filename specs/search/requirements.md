# Search — Requirements

## Functional Requirements

1. **FR-1**: Search GitHub for repositories with the `fledge-template` topic
2. **FR-2**: Accept optional keyword to narrow search results
3. **FR-3**: Display results as a formatted table (name, description, stars, usage hint)
4. **FR-4**: Support `--json` flag for machine-readable output
5. **FR-5**: Support `--limit N` flag to control result count (default 20)
6. **FR-6**: Use `github.token` from config if available for higher rate limits
7. **FR-7**: Show helpful error when rate-limited, suggesting token configuration

## Non-Functional Requirements

1. **NFR-1**: Search should complete within 5 seconds on reasonable connections
2. **NFR-2**: No async runtime dependency — use blocking HTTP client
3. **NFR-3**: Graceful degradation when offline or rate-limited
