---
spec: prompts.spec.md
---

## Automated Testing

| Test File | Type | What It Covers |
|-----------|------|----------------|
| `src/prompts.rs` (inline) | Unit | `to_snake_case` (hyphens, uppercase, empty, single char), `to_pascal_case` (hyphens, underscores, mixed, empty, single char), `render_default` (plain strings, variable interpolation, missing variable errors) |

## Manual Testing

- [x] `fledge init my-app` shows template selector with name and description
- [x] Author auto-fills from config when `defaults.author` is set
- [x] Author auto-fills from `git config user.name` when config author is unset
- [x] Author prompts interactively when neither config nor git is available
- [x] GitHub org auto-fills from config when `defaults.github_org` is set
- [x] GitHub org prompts with "CorvidLabs" default when config is unset
- [x] Custom template prompts appear after core prompts
- [x] Custom prompt with Tera default (e.g., `"A {{ project_name }} project"`) renders correctly

## Edge Cases & Boundary Conditions

| Scenario | Expected Behavior |
|----------|-------------------|
| `to_snake_case("")` | Returns empty string |
| `to_snake_case("A")` | Returns `"a"` |
| `to_pascal_case("")` | Returns empty string |
| `to_pascal_case("a")` | Returns `"A"` |
| `to_pascal_case("my-cool_project")` | Returns `"MyCoolProject"` |
| Prompt default with `{{ missing_var }}` | `render_default` returns error |
| Prompt default without `{{` | Returned as-is (no Tera parsing) |
| Template with no custom prompts | Only core variables collected |
| Empty template list passed to `select_template` | dialoguer shows empty list (edge case, prevented by init) |
