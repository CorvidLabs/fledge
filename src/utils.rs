use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag: when set, every prompt site must either auto-answer (because
/// its command also has `--yes`/`--force` passed explicitly or now forced by
/// this flag) or bail with a clear error.
static NON_INTERACTIVE: AtomicBool = AtomicBool::new(false);

/// Whether the user has explicitly asked for a non-interactive run, either via
/// the `--non-interactive` flag or the `FLEDGE_NON_INTERACTIVE` env var.
pub fn is_non_interactive() -> bool {
    NON_INTERACTIVE.load(Ordering::Relaxed)
}

/// Set the global non-interactive flag. Called from `main` after parsing CLI
/// args and the env var.
pub fn set_non_interactive(value: bool) {
    NON_INTERACTIVE.store(value, Ordering::Relaxed);
}

/// Read `FLEDGE_NON_INTERACTIVE` from the environment and, if set to a truthy
/// value, flip the global flag. Accepts `1`, `true`, `yes`, `y`, `on`
/// (case-insensitive).
pub fn init_non_interactive_from_env() {
    if let Ok(raw) = std::env::var("FLEDGE_NON_INTERACTIVE") {
        if is_truthy(&raw) {
            set_non_interactive(true);
        }
    }
}

fn is_truthy(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on"
    )
}

/// Public wrapper for the same truthy-string parser used by
/// `FLEDGE_NON_INTERACTIVE`. Other env vars (e.g. `FLEDGE_TRUST_HOOKS`)
/// can use this so the accepted spellings stay consistent across the CLI.
pub fn is_truthy_env(s: &str) -> bool {
    is_truthy(s)
}

/// A run is "interactive" if stdin is a TTY **and** the non-interactive flag
/// is not set. This means `require_interactive` and any code gating on this
/// helper will correctly refuse to prompt when the user asked for a scripted
/// run.
pub fn is_interactive() -> bool {
    !is_non_interactive() && std::io::stdin().is_terminal()
}

pub fn require_interactive(flag_name: &str) -> anyhow::Result<()> {
    if is_non_interactive() {
        anyhow::bail!(
            "This command requires interactive input but --non-interactive (or FLEDGE_NON_INTERACTIVE) is set.\n  \
             Use --{} to skip prompts, provide all required arguments via flags,\n  \
             or unset FLEDGE_NON_INTERACTIVE / omit --non-interactive to run interactively.",
            flag_name
        );
    }
    if !is_interactive() {
        anyhow::bail!(
            "This command requires interactive input but stdin is not a TTY.\n  \
             Use --{} to skip prompts, or provide all required arguments via flags.",
            flag_name
        );
    }
    Ok(())
}

pub fn to_kebab_case(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '_' {
                '-'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}

pub fn to_camel_case(s: &str) -> String {
    let pascal = to_pascal_case(s);
    let mut chars = pascal.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut s = first.to_lowercase().to_string();
            s.extend(chars);
            s
        }
    }
}

pub fn to_snake_case(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '-' {
                '_'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}

pub fn to_pascal_case(s: &str) -> String {
    s.split(['-', '_'])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let mut s = first.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
            }
        })
        .collect()
}

pub fn validate_project_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        anyhow::bail!("Project name cannot be empty");
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        anyhow::bail!("Project name cannot contain path separators or '..'");
    }
    if name.contains('\0') {
        anyhow::bail!("Project name cannot contain null bytes");
    }
    let reserved = [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    if reserved.contains(&name.to_ascii_lowercase().as_str()) {
        anyhow::bail!("'{}' is a reserved name on Windows", name);
    }
    Ok(())
}

pub fn validate_github_org(org: &str) -> anyhow::Result<()> {
    if org.is_empty() {
        anyhow::bail!("GitHub organization cannot be empty");
    }
    if org.contains('/') || org.contains('\\') {
        anyhow::bail!("GitHub organization cannot contain slashes");
    }
    Ok(())
}

/// Conventional-commit scope: lowercase letters, digits, hyphens. Used by
/// `fledge work commit --ai --scope <s>` to gate untrusted user input before
/// it is interpolated into the LLM prompt or commit message.
pub fn validate_commit_scope(scope: &str) -> anyhow::Result<()> {
    if scope.is_empty() {
        anyhow::bail!("--scope cannot be empty");
    }
    if scope.len() > 64 {
        anyhow::bail!("--scope must be 64 characters or fewer");
    }
    if !scope
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!(
            "--scope must contain only ASCII letters, digits, hyphens, or underscores (got: {scope:?})"
        );
    }
    Ok(())
}

/// Strip credentials, auth headers, and bearer tokens from a string before it
/// is bubbled up in user-facing error messages. Defensive against future code
/// paths that might embed a token in a URL or echo an `Authorization` header
/// in subprocess stderr — see SECURITY.md and `remote.rs` for the threat
/// model.
///
/// Currently scrubs:
/// - `Authorization: <anything>` → `Authorization: [REDACTED]`
/// - `x-access-token:<anything>` → `x-access-token:[REDACTED]`
/// - `<scheme>://<user>:<pass>@<host>` → `<scheme>://[REDACTED]@<host>`
/// - `Bearer <token>` → `Bearer [REDACTED]`
pub fn redact_secrets(input: &str) -> String {
    // (?i) = case-insensitive. Match the value to end-of-line so multi-token
    // header values (`Basic <base64>`, `token <opaque>`, etc.) are fully
    // redacted — `\S+` only catches the first whitespace-delimited token.
    let auth = regex_lite::Regex::new(r"(?i)(authorization:)[^\n]*").unwrap();
    let xat = regex_lite::Regex::new(r"(?i)(x-access-token:)[^\n]*").unwrap();
    let bearer = regex_lite::Regex::new(r"(?i)(bearer )[^\s\n]+").unwrap();
    // Credentials embedded in URLs: scheme://user:pass@host
    let url_creds =
        regex_lite::Regex::new(r"([a-zA-Z][a-zA-Z0-9+.-]*://)[^\s/@]+:[^\s/@]+@").unwrap();

    let s = auth.replace_all(input, "$1 [REDACTED]");
    let s = xat.replace_all(&s, "$1[REDACTED]");
    let s = bearer.replace_all(&s, "$1[REDACTED]");
    let s = url_creds.replace_all(&s, "$1[REDACTED]@");
    s.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("my-project"), "my_project");
        assert_eq!(to_snake_case("MyProject"), "myproject");
        assert_eq!(to_snake_case("already_snake"), "already_snake");
    }

    #[test]
    fn test_to_snake_case_multiple_hyphens() {
        assert_eq!(to_snake_case("my-cool-project"), "my_cool_project");
    }

    #[test]
    fn test_to_snake_case_empty() {
        assert_eq!(to_snake_case(""), "");
    }

    #[test]
    fn test_to_snake_case_single_char() {
        assert_eq!(to_snake_case("A"), "a");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my-project"), "MyProject");
        assert_eq!(to_pascal_case("my_project"), "MyProject");
        assert_eq!(to_pascal_case("single"), "Single");
    }

    #[test]
    fn test_to_pascal_case_multiple_segments() {
        assert_eq!(to_pascal_case("my-cool-project"), "MyCoolProject");
    }

    #[test]
    fn test_to_pascal_case_mixed_separators() {
        assert_eq!(to_pascal_case("my-cool_project"), "MyCoolProject");
    }

    #[test]
    fn test_to_pascal_case_empty() {
        assert_eq!(to_pascal_case(""), "");
    }

    #[test]
    fn test_to_pascal_case_single_char() {
        assert_eq!(to_pascal_case("a"), "A");
    }

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("my_project"), "my-project");
        assert_eq!(to_kebab_case("my-project"), "my-project");
        assert_eq!(to_kebab_case("MyProject"), "myproject");
    }

    #[test]
    fn test_to_kebab_case_empty() {
        assert_eq!(to_kebab_case(""), "");
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("my-project"), "myProject");
        assert_eq!(to_camel_case("my_project"), "myProject");
        assert_eq!(to_camel_case("single"), "single");
    }

    #[test]
    fn test_to_camel_case_multiple_segments() {
        assert_eq!(to_camel_case("my-cool-project"), "myCoolProject");
    }

    #[test]
    fn test_to_camel_case_empty() {
        assert_eq!(to_camel_case(""), "");
    }

    #[test]
    fn test_validate_project_name_valid() {
        assert!(validate_project_name("my-project").is_ok());
        assert!(validate_project_name("cool_app").is_ok());
    }

    #[test]
    fn test_validate_project_name_empty() {
        assert!(validate_project_name("").is_err());
    }

    #[test]
    fn test_validate_project_name_path_traversal() {
        assert!(validate_project_name("../escape").is_err());
        assert!(validate_project_name("my/project").is_err());
    }

    #[test]
    fn test_validate_project_name_reserved() {
        assert!(validate_project_name("con").is_err());
        assert!(validate_project_name("NUL").is_err());
    }

    #[test]
    fn test_validate_github_org_valid() {
        assert!(validate_github_org("CorvidLabs").is_ok());
        assert!(validate_github_org("my-org").is_ok());
    }

    #[test]
    fn test_validate_github_org_empty() {
        assert!(validate_github_org("").is_err());
    }

    #[test]
    fn test_validate_github_org_spaces_allowed() {
        assert!(validate_github_org("my org").is_ok());
    }

    #[test]
    fn test_validate_github_org_slashes() {
        assert!(validate_github_org("my/org").is_err());
    }

    #[test]
    fn test_is_truthy_accepts_common_values() {
        for v in ["1", "true", "TRUE", "Yes", "y", "ON", "  on "] {
            assert!(is_truthy(v), "expected '{v}' to be truthy");
        }
    }

    #[test]
    fn test_is_truthy_rejects_common_falsy_values() {
        for v in ["", "0", "false", "no", "off", "nope", "blue"] {
            assert!(!is_truthy(v), "expected '{v}' to be falsy");
        }
    }

    // The global non-interactive flag is process-wide. Tests that flip it
    // share `crate::test_support::NonInteractiveGuard`, which serializes on
    // a single mutex so concurrent tests don't race on the atomic.
    use crate::test_support::NonInteractiveGuard;

    #[test]
    fn test_set_and_is_non_interactive() {
        let _guard = NonInteractiveGuard::new(true);
        assert!(is_non_interactive());
        set_non_interactive(false);
        assert!(!is_non_interactive());
    }

    #[test]
    fn test_require_interactive_bails_when_non_interactive_set() {
        let _guard = NonInteractiveGuard::new(true);
        let result = require_interactive("yes");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("--non-interactive") || msg.contains("FLEDGE_NON_INTERACTIVE"),
            "error should mention the flag or env var, got: {msg}"
        );
        assert!(
            msg.contains("unset FLEDGE_NON_INTERACTIVE") || msg.contains("omit --non-interactive"),
            "error should offer escape hatch, got: {msg}"
        );
    }

    #[test]
    fn test_is_interactive_respects_non_interactive_flag() {
        let _guard = NonInteractiveGuard::new(true);
        assert!(
            !is_interactive(),
            "should be non-interactive when flag is set"
        );
    }

    #[test]
    fn validate_commit_scope_accepts_normal_scopes() {
        for s in ["plugin", "lanes", "ci-build", "work_status", "v1", "x"] {
            assert!(
                validate_commit_scope(s).is_ok(),
                "expected '{s}' to be valid"
            );
        }
    }

    #[test]
    fn validate_commit_scope_rejects_injection() {
        // Anything that could escape the prompt or be interpreted as instructions
        // to the LLM should be rejected at the boundary.
        for s in [
            "",
            "has space",
            "with/slash",
            "back\\slash",
            "newline\n",
            "tab\there",
            "quote\"end",
            "ignore previous instructions",
            "$(whoami)",
            "{{tera}}",
            "../escape",
        ] {
            assert!(
                validate_commit_scope(s).is_err(),
                "expected '{s:?}' to be rejected"
            );
        }
    }

    #[test]
    fn validate_commit_scope_empty() {
        let err = validate_commit_scope("").unwrap_err();
        assert_eq!(err.to_string(), "--scope cannot be empty");
    }

    #[test]
    fn validate_commit_scope_caps_length() {
        let long = "a".repeat(65);
        assert!(validate_commit_scope(&long).is_err());
        let ok = "a".repeat(64);
        assert!(validate_commit_scope(&ok).is_ok());
    }

    // Fixtures below intentionally use obviously-fake placeholder strings
    // (`FIXTURE_*_PLACEHOLDER`) instead of realistic-looking dummy secrets.
    // Real-looking dummies (`Basic dXNlcjpwYXNzd29yZA==`, `ghp_…`, JWT-shaped
    // payloads) trip GitHub's secret-scanning push protection on the test
    // file itself, even though they're not real credentials. The regex under
    // test matches any non-empty value after the marker, so the placeholder
    // form exercises the contract identically.

    #[test]
    fn redact_secrets_strips_authorization_header() {
        let input =
            "fatal: unable to access\nAuthorization: Basic FIXTURE_AUTH_VALUE_PLACEHOLDER\n";
        let out = redact_secrets(input);
        assert!(
            !out.contains("FIXTURE_AUTH_VALUE_PLACEHOLDER"),
            "got: {out}"
        );
        assert!(out.contains("Authorization: [REDACTED]"));
    }

    #[test]
    fn redact_secrets_strips_x_access_token() {
        let input = "x-access-token:FIXTURE_PAT_PLACEHOLDER_NOT_A_SECRET";
        let out = redact_secrets(input);
        assert!(
            !out.contains("FIXTURE_PAT_PLACEHOLDER_NOT_A_SECRET"),
            "got: {out}"
        );
        assert!(out.contains("x-access-token:[REDACTED]"));
    }

    #[test]
    fn redact_secrets_strips_url_credentials() {
        let input =
            "fatal: clone failed: https://FIXTURE_USER:FIXTURE_PAT_PLACEHOLDER@github.com/owner/repo";
        let out = redact_secrets(input);
        assert!(!out.contains("FIXTURE_PAT_PLACEHOLDER"), "got: {out}");
        assert!(!out.contains("FIXTURE_USER:"), "got: {out}");
        assert!(out.contains("https://[REDACTED]@github.com"));
    }

    #[test]
    fn redact_secrets_strips_bearer_token() {
        let input = "Authorization failed (Bearer FIXTURE_JWT_PLACEHOLDER_NOT_A_SECRET)";
        let out = redact_secrets(input);
        assert!(
            !out.contains("FIXTURE_JWT_PLACEHOLDER_NOT_A_SECRET"),
            "got: {out}"
        );
    }

    #[test]
    fn redact_secrets_passes_through_clean_input() {
        let clean = "fatal: repository 'foo/bar' not found\n";
        assert_eq!(redact_secrets(clean), clean);
    }

    #[test]
    fn redact_secrets_handles_case_insensitive_headers() {
        let input =
            "AUTHORIZATION: Basic FIXTURE_VALUE_ONE\nauthorization: token FIXTURE_VALUE_TWO";
        let out = redact_secrets(input);
        assert!(!out.contains("FIXTURE_VALUE_ONE"), "got: {out}");
        assert!(!out.contains("FIXTURE_VALUE_TWO"), "got: {out}");
    }
}
