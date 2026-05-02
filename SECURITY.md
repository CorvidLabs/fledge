# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | Yes              |
| 0.17.x  | Best effort      |
| < 0.17  | No               |

Once 1.0 ships, only 1.0.x receives security fixes. Until then, the latest
0.x release is the supported line.

## Reporting a Vulnerability

If you discover a security vulnerability, **do not open a public issue**.

Instead, please report it privately via [GitHub Security Advisories](https://github.com/CorvidLabs/fledge/security/advisories/new) or email the maintainers directly.

Include:
- A description of the vulnerability and its potential impact
- Steps to reproduce
- Any suggested fix (optional but appreciated)

We aim to acknowledge reports within 48 hours and provide a fix or mitigation plan within 7 days.

## Security Model

### Template Rendering

- Templates are rendered through Tera (Jinja2-style) in a sandboxed context
- Path traversal is blocked — templates cannot write outside the project directory
- **Local** templates (built-in or under a configured `extra_paths`) are
  presumed user-authored. `--yes` (or `FLEDGE_NON_INTERACTIVE=1`) auto-confirms
  their `post_create` hooks, on the same trust footing as the rest of the
  template content
- **Remote** templates fetched from GitHub get a stricter consent rule.
  `--yes` does **not** authorize their hooks — `--yes` skips routine prompts
  (template-variable defaults, etc.), but arbitrary shell execution from a
  third-party source needs explicit consent. Pass `--trust-hooks` (or set
  `FLEDGE_TRUST_HOOKS=1`) to authorize hook execution for the run; otherwise
  the prompt fires interactively, or hooks are skipped in non-interactive
  mode with a hint pointing at the right flag
- The dry-run path always lists the hooks that would run (regardless of
  trust) so the user can audit before consenting

### GitHub Integration

- Tokens are read from `FLEDGE_GITHUB_TOKEN`, `GITHUB_TOKEN`, or `~/.config/fledge/config.toml` (in that order)
- Tokens are never logged, displayed, or included in error messages
- All GitHub API calls use HTTPS

### Plugins

- Plugins are external executables installed from GitHub repos
- Plugin installation requires explicit user action (`fledge plugins install`)
- Plugin binaries are symlinked to the platform config directory (see
  [File Locations](#file-locations) below)
- **Plugins run as unsandboxed processes with the same permissions as the
  user.** A plugin binary can read any file the user can read, write to any
  directory the user can write to, and make network requests — regardless of
  its declared capabilities. Capabilities gate the fledge-v1 *protocol*
  (exec/store/metadata RPC messages), not the process itself. Treat
  installing a plugin as equivalent to running arbitrary code
- The `fledge-v1` plugin protocol exposes three opt-in capabilities — `exec`,
  `store`, and `metadata` — that default to `false`. Each is presented for
  explicit user approval at install time and persisted in `plugins.toml`
- **`exec` grants full shell access — there is no sandbox.** A plugin with
  `exec = true` can run any shell command via `sh -c <command>` (Unix) or
  `cmd /C <command>` (Windows). The optional `cwd` parameter is validated
  to stay within the project root or the plugin's own directory, but the
  command string itself is unfiltered — `cat /etc/passwd`, `curl`, absolute
  paths, and `cd /` all work. Treat granting `exec` the same as granting
  the plugin full access to your system as your user
- Stdout/stderr from `exec` are each capped at 10 MB; plugin state at 1 MB
  total / 64 KB per value / 256 keys; prompt/cancel timeouts at 5 minutes

### File Locations

Plugin storage uses the platform config directory (`dirs::config_dir()`):

| Platform | Base path |
|----------|-----------|
| macOS    | `~/Library/Application Support/fledge/` |
| Linux    | `~/.config/fledge/` |
| Windows  | `%APPDATA%\fledge\` |

Under that base:
- `plugins/` — installed plugin directories
- `plugins/bin/` — symlinked binaries
- `plugins.toml` — plugin registry
- `config.toml` — global fledge config

### Dependencies

- `cargo audit` runs in CI to check for known vulnerabilities
- Dependencies are kept up to date — run `fledge deps --audit` to check locally

## Scope

The following are in scope for security reports:

- Path traversal or file write outside project boundaries
- Command injection via template variables or plugin names
- Token leakage (GitHub tokens appearing in logs, errors, or output)
- Arbitrary code execution without user consent (e.g., hooks running without confirmation)

The following are out of scope:

- Issues requiring physical access to the machine
- Social engineering
- Denial of service against the CLI itself
