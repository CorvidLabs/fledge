# Doctor: Environment Diagnostics

`fledge doctor` checks your environment for issues that might cause problems. Run it when something isn't working right, or proactively before starting a new project.

## Usage

```bash
fledge doctor
fledge doctor --json
```

## What It Checks

Doctor reports four sections:

### `fledge`

- `fledge config` — does `~/.config/fledge/config.toml` parse cleanly?

### `Git`

- `git` is installed and on `PATH`
- The current directory is a git repository
- A remote is configured
- Working tree is clean (uncommitted changes are reported as a fixable issue)

### `AI`

- `claude` CLI is installed (powers `fledge review` and `fledge ask` when the active provider is `claude`)
- `ollama` binary is installed (powers the `ollama` provider)
- The active provider's reachability — when Ollama is active, doctor probes `<host>/api/tags` with a 3-second timeout to distinguish "daemon down" from "not installed"

### `Toolchains` *(informational)*

Probes 16 toolchains across the major language ecosystems:

| Group | Probed |
|-------|--------|
| Rust | `rustc`, `cargo` |
| Node.js | `node`, `npm`, `pnpm`, `bun`, `yarn` |
| Python | `python3`, `uv`, `poetry` |
| Go | `go` |
| Ruby | `ruby` |
| Swift | `swift` |
| JVM | `java`, `gradle`, `mvn` |

The Toolchains section is **informational** — missing entries render dimmed (`· tool (not installed)`) and don't pollute the pass/fail totals. A Python project shouldn't fail because Swift is absent, so doctor reports the toolchain inventory without treating absence as failure.

## Output

```
$ fledge doctor

  fledge
    ✅ fledge config 0.15.2 — loaded

  Git
    ✅ git 2.50.1
    ✅ repository — initialized
    ✅ remote — origin ➡️ git@github.com:CorvidLabs/fledge.git
    ✅ working tree — clean

  AI
    ✅ claude 2.1.119
    ✅ ollama 0.21.2
    ✅ Active provider: ollama — ollama is the active provider (model: gpt-oss:120b-cloud, host: http://localhost:11434)

  Toolchains
    ✅ rustc 1.93.0
    ✅ cargo 1.93.0
    ✅ node 25.5.0
    ✅ bun 1.3.12
    · pnpm (not installed)
    · yarn (not installed)
    ✅ python3 3.14.3
    · uv (not installed)
    ✅ swift 6.3
    · go (not installed)

  7 checks passed, 0 issues found
```

Pass/fail totals only count the non-informational sections.

## When to Run It

- Before your first `fledge templates init` to make sure your environment is ready
- When `fledge run` can't find the right command for your project type — the `Toolchains` section will tell you what's missing
- When AI commands fail — the `AI` section distinguishes "Claude not installed" from "Ollama daemon down" from "wrong provider configured"
- After upgrading your toolchain or switching machines

## JSON Output

```bash
fledge doctor --json
```

Returns a structured report:

```json
{
  "sections": [
    {
      "name": "fledge",
      "checks": [{"name": "fledge config", "status": "ok", "version": "0.15.2", "detail": "loaded", "fix": null}]
    },
    {
      "name": "Toolchains",
      "checks": [
        {"name": "rustc", "status": "ok", "version": "1.93.0", "detail": null, "fix": null},
        {"name": "pnpm", "status": "missing", "version": null, "detail": "not installed", "fix": null}
      ],
      "informational": true
    }
  ],
  "passed": 7,
  "failed": 0
}
```

The `informational: true` field marks sections (currently just `Toolchains`) whose check results don't contribute to `passed`/`failed`. Filter on it when scripting if you want to ignore environmental noise.
