---
spec: create_template.spec.md
---

## Unit Tests

| Test | What it verifies |
|------|-----------------|
| `scaffold_creates_expected_files` | All expected files exist after scaffolding |
| `scaffold_manifest_is_valid_toml` | Generated template.toml parses as TemplateManifest |
| `scaffold_manifest_without_hooks_or_prompts` | Hooks/prompts sections excluded when not requested |
| `scaffold_fails_if_target_exists` | Error when target directory already exists |
| `manifest_render_globs_are_correct` | Render globs in manifest match user input |

## Integration Test Ideas

- Run `fledge create-template test-tpl` then `fledge init my-project -t ./test-tpl` to verify round-trip
- Verify `fledge list` discovers a scaffolded template from an extra path

## Manual Testing

```bash
cargo run -- create-template my-test-template
ls -la my-test-template/
cat my-test-template/template.toml
cargo run -- init test-project -t ./my-test-template
```
