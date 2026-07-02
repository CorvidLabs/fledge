---
module: ask
version: 7
status: active
files:
  - src/ask.rs

db_tables: []
depends_on:
  - spec
  - llm
  - config
---

# Ask

## Purpose

Ask questions about your codebase using AI. Builds a spec-augmented prompt (compact index of every module plus optional full bundles for named modules) and sends it to the active LLM provider over HTTP — `anthropic`, any OpenAI-compatible endpoint (`openai` and the gateways), or an Ollama-speaking endpoint. As of 1.5.0 there is no CLI shell-out: every provider except Ollama is served by the [`corvid-ai`](https://crates.io/crates/corvid-ai) crate, and Ollama keeps fledge's native HTTP client. The question composition is provider-agnostic; the provider is resolved from CLI override > `FLEDGE_AI_PROVIDER` env > `ai.provider` config > **auto-detect** (the first configured provider with a key, falling back to keyless local Ollama). When several providers are configured and the session is interactive (and `--json` is not set), `ask` prompts the user to pick a provider + model; non-TTY / CI / `--json` fall through to the deterministic auto-detect.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point for the ask command |
| `AskOptions` | Options struct with question, json, with_specs, no_spec_index, provider, model |

### Structs & Enums

| Type | Description |
|------|-------------|
| `AskOptions` | `{ question: String, json: bool, with_specs: Vec<String>, no_spec_index: bool, provider: Option<String>, model: Option<String> }` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(AskOptions) -> Result<()>` | Builds the spec-augmented prompt, resolves the provider, invokes it |
| `build_spec_context` | `(&Path, &[String], bool) -> Result<Option<String>>` | (private) Assemble the spec-context block (index + requested bundles) |
| `expand_with_specs` | `(&[String], &Path) -> Result<Vec<String>>` | (private) Flatten comma-separated names; `"all"` expands to every module |
| `build_prompt` | `(&str, bool, Option<&str>) -> String` | (private) Final prompt = preamble + optional spec context + question |

## Invariants

1. Requires the active provider to be reachable over HTTP — no CLI is shelled out. `anthropic` and the OpenAI-compatible providers need an API key (`<PROVIDER>_API_KEY` env or `ai.<provider>.api_key`); `ollama` needs a reachable Ollama-speaking endpoint (local daemon, or an API key for cloud). `fledge ai status` / `fledge doctor` report which provider is active and whether it's configured.
2. Question is joined from multiple args (no quotes required)
3. `--json` outputs `{question, answer, provider, model}` structured response
4. `--provider <name>` overrides env and config; validated at parse time by a clap `value_parser` (accepted: `anthropic`, `openai`, `openrouter`, `gemini`, `deepseek`, `groq`, `mistral`, `xai`, `together`, `ollama`, and the deprecated `claude` alias of `anthropic`; typos rejected with a clap error)
5. `--model <name>` overrides env and config
6. Prompt composition is provider-agnostic: the exact same text flows to whichever provider is active
7. By default, a compact spec index is always prepended to the prompt (one line per module: name, version, status, files, first-paragraph purpose). Skipped only when `--no-spec-index` is passed.
8. `--with-specs <names>` (comma-separated or repeated) loads full spec + existing companion files for each named module. `"all"` expands to every spec in the project and supersedes any other names in the same invocation.
9. When no specs exist in the project and no `--with-specs` flag is passed, the prompt is unchanged from the pre-spec-index behavior. When `--with-specs <name>` is passed against a project with no specs (or an unknown name), the command bails with a clear error rather than silently succeeding.
10. Spec loading never silently swallows a user-requested bundle: any `--with-specs <name>` that fails to resolve bails with `loading spec bundle for '<name>'`. The ambient index (when `--no-spec-index` is not set) is best-effort — a malformed frontmatter on an unrelated spec is skipped from the index so one bad spec can't break `ask`.
11. Module names passed to `--with-specs` are validated to prevent path traversal: `/`, `\`, `..`, `.`, and empty strings are rejected before any filesystem access.

## Behavioral Examples

### ask — default (index auto-included)
```
$ fledge ask "how does the work module build branch names?"
● Thinking [ollama (llama3.3)]:

[the model reads the index, knows there's a `work` module, cites specs/work/work.spec.md in its answer]
```

### ask — with full spec + companions for a module
```
$ fledge ask --with-specs work "why does the work module sanitize branch names this way?"
● Thinking [anthropic (claude-sonnet-4-6)]:

[the model has the full spec, context.md design decisions, and requirements.md in its prompt]
```

### ask — multiple specs, comma or repeated
```
$ fledge ask --with-specs work,trust "how do these modules interact?"
$ fledge ask --with-specs work --with-specs trust "how do these modules interact?"
```

### ask — nuclear option
```
$ fledge ask --with-specs all "which modules touch GitHub?"
```

### ask — skip the index (saves tokens)
```
$ fledge ask --no-spec-index "quick syntax question: how do I declare an async trait?"
```

### ask — json
```
$ fledge ask --json "what is the release workflow?"
{
  "schema_version": 1,
  "action": "ask",
  "question": "what is the release workflow?",
  "answer": "...",
  "provider": "ollama",
  "model": "llama3.3"
}
```

### ask — no question
```
$ fledge ask
error: Please provide a question. Usage: fledge ask <question>
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Missing API key | Active provider (`anthropic` or an OpenAI-compatible provider) has no key in env/config | Bail with API-key setup guidance (surfaced from `llm` / `corvid-ai`) |
| Ollama endpoint unreachable | Active provider `ollama` and the POST to `<host>/api/generate` fails | Bail with the full URL and a "(is the Ollama server running?)" hint |
| Unknown provider | `--provider` value outside the accepted set | clap rejects at parse time with a "possible values" hint |
| No question provided | Empty args | Bail with usage hint |
| `--with-specs <name>` where `specs/<name>/` does not exist | Unknown module | Bail with the looked-at path |

## Dependencies

- Active LLM provider over HTTP — `anthropic`, an OpenAI-compatible endpoint, or an Ollama-speaking endpoint (see `llm` spec); no CLI dependency
- `spec` module — `collect_index`, `render_index_markdown`, `load_module_bundle`, `all_module_names`
- `llm` module — `build_provider`, `ProviderOverride`, `describe`
- `ai` module — `pick_when_multiple` for the interactive provider+model pick when several are configured
- `config` module — `ai.*` section for provider + model resolution

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 7 | 2026-07-02 | Correct stale Claude-CLI references left over from before the 1.5.0 HTTP-provider migration. Purpose, invariants, error cases, and dependencies now describe the `corvid-ai`-backed HTTP providers (`anthropic` / OpenAI-compatible / `ollama`) with **auto-detect** as the default and an interactive pick when several are configured — no CLI shell-out and no `claude --version` check. Invariant 4 lists the real clap `value_parser` provider set; behavioral examples use the actual `● Thinking [<provider (model)>]:` spinner. No code change |
| 6 | 2026-04-26 | Doc sync, behavioral example updated to show the post-tier-D envelope shape with `schema_version`/`action`/`provider`/`model`. No code change |
| 5 | 2026-04-26 | Tier-D 1.0 envelope: `ask --json` now wraps output as `{schema_version: 1, action: "ask", question, answer, provider, model}`. Previously emitted bare `{question, answer, provider, model}`. Closes a gap where tier C (#274) only migrated plugins/lanes/templates |
| 4 | 2026-04-23 | Provider abstraction: Claude CLI is no longer hardcoded. New `--provider` and `--model` flags. JSON output gains `provider` and `model` fields. Runs through `llm::build_provider` with config / env / override precedence. |
| 3 | 2026-04-23 | Default-on spec index in prompt; add `--with-specs` for full spec+companion bundles; add `--no-spec-index` escape hatch. Depends on `spec` module helpers. |
| 2 | 2026-04-21 | Add json field to AskOptions |
| 1 | 2026-04-19 | Initial spec |
