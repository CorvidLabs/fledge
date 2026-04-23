---
spec: watch.spec.md
---

## Testing

### Unit Tests

- `ignore_git_directory` — .git paths are ignored
- `ignore_target_directory` — target paths are ignored
- `ignore_node_modules` — node_modules paths are ignored
- `ignore_fledge_directory` — .fledge paths are ignored
- `ignore_pycache` — __pycache__ paths are ignored
- `do_not_ignore_regular_path` — source files are not ignored
- `do_not_ignore_similar_names` — similar but different names (target_dir, .github) not ignored
- `extension_filter_matches_rs` — extension matching works for .rs and .toml
- `extension_filter_empty_matches_all` — empty filter matches everything
- `extension_filter_no_ext_file` — files without extensions don't match a filter
- `parse_extensions_basic` — comma-separated parsing
- `parse_extensions_with_dots` — dot-prefixed extensions stripped
- `parse_extensions_with_spaces` — whitespace trimmed
- `parse_extensions_empty` — empty string returns empty vec
- `parse_extensions_single` — single extension
- `format_duration_millis` — sub-second formatting
- `format_duration_seconds` — seconds formatting
- `format_duration_minutes` — minutes formatting
- `format_duration_zero` — zero duration
- `ignore_dirs_list_is_complete` — all expected dirs present
- `watch_options_defaults` — struct construction
- `combined_ignore_and_extension_filter` — combined filtering logic
