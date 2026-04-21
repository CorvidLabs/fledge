# Doctor: Environment Diagnostics

`fledge doctor` checks your environment for issues that might cause problems. Run it when something isn't working right, or proactively before starting a new project.

## Usage

```bash
fledge doctor
fledge doctor --json
```

## What It Checks

| Check | What it looks for |
|-------|-------------------|
| **Rust toolchain** | `cargo`, `rustc`, `clippy`, `rustfmt` |
| **Node.js** | `node`, `npm`, `bun`, `yarn`, `pnpm` |
| **Go** | `go` |
| **Python** | `python3`, `pip`, `ruff` |
| **Git** | `git`, configured user name and email |
| **GitHub** | Token present (`GITHUB_TOKEN` or config) |
| **Config** | Valid `~/.config/fledge/config.toml` |
| **Templates** | Configured template paths and repos are accessible |

## Output

Doctor reports each check as passing, warning, or failing:

- **Pass**: tool is installed and working
- **Warn**: tool is missing but only needed for specific features
- **Fail**: something is broken that will cause errors

## When to Run It

- Before your first `fledge init` to make sure your environment is ready
- When `fledge run` can't find the right command for your project type
- When GitHub commands (`issues`, `prs`, `checks`, `work pr`) fail with auth errors
- After upgrading your toolchain or switching machines

## JSON Output

```bash
fledge doctor --json
```

Returns a structured report for scripting or CI checks. Each entry includes the check name, status, version (if applicable), and any diagnostic message.
