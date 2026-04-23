# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | Yes       |
| < 1.0   | No        |

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
- Remote template hooks (`post_create` commands) always require user confirmation before execution, unless `--yes` is explicitly passed

### GitHub Integration

- Tokens are read from `FLEDGE_GITHUB_TOKEN`, `GITHUB_TOKEN`, or `~/.config/fledge/config.toml` (in that order)
- Tokens are never logged, displayed, or included in error messages
- All GitHub API calls use HTTPS

### Plugins

- Plugins are external executables installed from GitHub repos
- Plugin installation requires explicit user action (`fledge plugins install`)
- Plugin binaries are symlinked to `~/.config/fledge/plugins/bin/`
- Plugins run with the same permissions as the user

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
