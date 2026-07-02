use console::style;
use serde::Serialize;
use std::path::Path;

use crate::config::TrustConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
#[clap(rename_all = "lowercase")]
pub enum TrustTier {
    Local,
    Official,
    Team,
    Unverified,
}

impl TrustTier {
    pub fn label(&self) -> &'static str {
        match self {
            TrustTier::Local => "local",
            TrustTier::Official => "official",
            TrustTier::Team => "team",
            TrustTier::Unverified => "unverified",
        }
    }

    pub fn styled_label(&self) -> console::StyledObject<&'static str> {
        match self {
            TrustTier::Local => style("local").magenta(),
            TrustTier::Official => style("official").green().bold(),
            TrustTier::Team => style("team").cyan(),
            TrustTier::Unverified => style("unverified").yellow(),
        }
    }
}

const OFFICIAL_ORGS: &[&str] = &["CorvidLabs", "corvidlabs"];

/// GitHub usernames of human members of the CorvidLabs org. Their *personal*
/// repos (e.g. `0xLeif/fledge-plugin-foo`) classify as `Team` — sources from
/// the CorvidLabs org itself classify as `Official` via `OFFICIAL_ORGS` above.
///
/// Compared case-insensitively (GitHub treats `0xLeif` and `0xleif` as the
/// same user). Users can extend the team tier at runtime via `trust.orgs` and
/// `trust.users` in config.toml — see `load_trust_config()`.
const TEAM_MEMBERS: &[&str] = &["0xGaspar", "0xLeif", "Kyntrin", "tofu-ux"];

fn is_team_member(owner: &str) -> bool {
    TEAM_MEMBERS.iter().any(|m| m.eq_ignore_ascii_case(owner))
}

fn load_trust_config() -> TrustConfig {
    crate::config::Config::load()
        .map(|c| c.trust)
        .unwrap_or_default()
}

fn is_config_org(owner: &str, config: &TrustConfig) -> bool {
    config.orgs.iter().any(|o| o.eq_ignore_ascii_case(owner))
}

fn is_config_user(owner: &str, config: &TrustConfig) -> bool {
    config.users.iter().any(|u| u.eq_ignore_ascii_case(owner))
}

pub fn parse_source_ref(source: &str) -> (&str, Option<&str>) {
    if source.starts_with("git@") {
        if let Some(rest) = source.strip_prefix("git@") {
            if let Some((_, after)) = rest.rsplit_once('@') {
                if !after.is_empty() {
                    let split_pos = source.len() - after.len() - 1;
                    return (&source[..split_pos], Some(after));
                }
            }
        }
        return (source, None);
    }
    match source.rsplit_once('@') {
        Some((before, after))
            if !after.is_empty() && !before.is_empty() && !after.contains('/') =>
        {
            (before, Some(after))
        }
        _ => (source, None),
    }
}

pub fn determine_trust_tier(source: &str) -> TrustTier {
    let path = Path::new(source);
    if source.starts_with("local:")
        || path.is_absolute()
        || source.starts_with("./")
        || source.starts_with("../")
        || source.starts_with(".\\")
        || source.starts_with("..\\")
        || source.starts_with('/')
        || source.starts_with('\\')
        || source == "."
        || source == ".."
    {
        return TrustTier::Local;
    }
    let config = load_trust_config();
    classify_source(source, &config)
}

pub fn determine_trust_tier_from_owner(owner: &str) -> TrustTier {
    let config = load_trust_config();
    classify_owner(owner, &config)
}

fn classify_source(source: &str, config: &TrustConfig) -> TrustTier {
    let (base, _) = parse_source_ref(source);

    let normalized = if base.starts_with("https://github.com/") {
        base.strip_prefix("https://github.com/")
            .unwrap_or(base)
            .trim_end_matches(".git")
    } else if base.starts_with("git@github.com:") {
        base.strip_prefix("git@github.com:")
            .unwrap_or(base)
            .trim_end_matches(".git")
    } else {
        base
    };

    // A legitimate owner/repo never contains "." or ".." path segments. Such a
    // segment lets git/curl collapse the clone URL client-side (RFC 3986
    // dot-segment removal), so "CorvidLabs/../attacker/evil" would classify on
    // the official "CorvidLabs" org yet actually fetch "attacker/evil" — a
    // trust-tier spoof. Never grant such a source any trust.
    if path_has_traversal_segment(normalized) {
        return TrustTier::Unverified;
    }

    if let Some((org, _)) = normalized.split_once('/') {
        if OFFICIAL_ORGS.contains(&org) {
            return TrustTier::Official;
        }
        if is_team_member(org) || is_config_org(org, config) || is_config_user(org, config) {
            return TrustTier::Team;
        }
    }

    TrustTier::Unverified
}

/// True if any "/"- or "\\"-separated segment of `path` is "." or "..",
/// checked against both the raw string and a percent-decoded copy. git and
/// curl percent-decode a URL before resolving it, so `%2e%2e` / `%2f` / `%5c`
/// collapse to `..` / `/` / `\` client-side just like literal segments — we
/// must decode too or the check is trivially bypassed.
fn path_has_traversal_segment(path: &str) -> bool {
    fn has_dot_segment(path: &str) -> bool {
        path.split(['/', '\\']).any(|seg| seg == "." || seg == "..")
    }
    has_dot_segment(path) || has_dot_segment(&percent_decode(path))
}

/// Decode `%XX` percent-escapes (case-insensitive hex) to their bytes and
/// render the result lossily as UTF-8. Incomplete or non-hex `%` sequences are
/// left verbatim. A single pass — matching what git/curl's transport does —
/// not a general-purpose URL decoder. Used only to normalize a source before
/// the traversal check.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// True if a plugin/template source contains a "." or ".." segment in its
/// owner/repo path. git/curl collapse such segments client-side so the fetched
/// repo differs from the one classified for trust; the install path rejects
/// these outright. Local "./"/"../" paths are handled by `determine_trust_tier`
/// before classification, so in practice this only inspects remote-style
/// sources (shorthand `owner/repo` or clone URLs).
pub fn source_has_path_traversal(source: &str) -> bool {
    let (base, _) = parse_source_ref(source);
    let normalized = base
        .strip_prefix("https://github.com/")
        .or_else(|| base.strip_prefix("http://github.com/"))
        .or_else(|| base.strip_prefix("git@github.com:"))
        .unwrap_or(base)
        .trim_end_matches(".git");
    path_has_traversal_segment(normalized)
}

fn classify_owner(owner: &str, config: &TrustConfig) -> TrustTier {
    if OFFICIAL_ORGS.contains(&owner) {
        TrustTier::Official
    } else if is_team_member(owner) || is_config_org(owner, config) || is_config_user(owner, config)
    {
        TrustTier::Team
    } else {
        TrustTier::Unverified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_shorthand() {
        assert_eq!(
            determine_trust_tier("CorvidLabs/fledge-plugin-deploy"),
            TrustTier::Official
        );
    }

    #[test]
    fn official_full_url() {
        assert_eq!(
            determine_trust_tier("https://github.com/CorvidLabs/fledge-plugin-deploy"),
            TrustTier::Official
        );
    }

    #[test]
    fn official_ssh_url() {
        assert_eq!(
            determine_trust_tier("git@github.com:CorvidLabs/fledge-plugin-deploy.git"),
            TrustTier::Official
        );
    }

    #[test]
    fn official_with_ref() {
        assert_eq!(
            determine_trust_tier("CorvidLabs/fledge-plugin-deploy@v1.0.0"),
            TrustTier::Official
        );
    }

    #[test]
    fn official_case_insensitive() {
        assert_eq!(
            determine_trust_tier("corvidlabs/fledge-plugin-deploy"),
            TrustTier::Official
        );
    }

    #[test]
    fn dotdot_source_never_official() {
        // Trust-tier spoof: first segment is an official org, but git/curl
        // collapse the URL to a different repo. Must NOT be classified Official.
        assert_eq!(
            determine_trust_tier("CorvidLabs/../attacker/evil"),
            TrustTier::Unverified
        );
        assert_eq!(
            determine_trust_tier("https://github.com/CorvidLabs/../attacker/evil.git"),
            TrustTier::Unverified
        );
        assert_eq!(
            determine_trust_tier("git@github.com:CorvidLabs/../attacker/evil.git"),
            TrustTier::Unverified
        );
    }

    #[test]
    fn source_traversal_detection() {
        assert!(source_has_path_traversal("CorvidLabs/../attacker/evil"));
        assert!(source_has_path_traversal(
            "https://github.com/CorvidLabs/../attacker/evil.git"
        ));
        // Legitimate sources must not be flagged.
        assert!(!source_has_path_traversal(
            "CorvidLabs/fledge-plugin-deploy"
        ));
        assert!(!source_has_path_traversal("owner/repo@v1.0.0"));
        assert!(!source_has_path_traversal(
            "https://github.com/CorvidLabs/fledge.git"
        ));
    }

    #[test]
    fn percent_encoded_traversal_rejected() {
        // git/curl decode %2e/%2f/%5c before resolving, so a percent-encoded
        // spoof must be caught just like a literal `..` (Gemini review, #434).
        assert!(source_has_path_traversal("CorvidLabs/%2e%2e/attacker/evil"));
        assert!(source_has_path_traversal("CorvidLabs/%2E%2E/attacker/evil")); // uppercase hex
        assert!(source_has_path_traversal("CorvidLabs%2f..%2fattacker")); // encoded slash
        assert!(source_has_path_traversal("CorvidLabs%5c..%5cattacker")); // encoded backslash
        assert_eq!(
            determine_trust_tier("https://github.com/CorvidLabs/%2e%2e/attacker/evil"),
            TrustTier::Unverified
        );
        // A literal percent that is not a traversal escape must not false-positive.
        assert!(!source_has_path_traversal("CorvidLabs/fledge-%25-plugin"));
    }

    #[test]
    fn unverified_third_party() {
        assert_eq!(
            determine_trust_tier("someuser/fledge-plugin-thing"),
            TrustTier::Unverified
        );
    }

    #[test]
    fn unverified_full_url() {
        assert_eq!(
            determine_trust_tier("https://github.com/someuser/fledge-plugin-thing"),
            TrustTier::Unverified
        );
    }

    #[test]
    fn owner_based_official() {
        assert_eq!(
            determine_trust_tier_from_owner("CorvidLabs"),
            TrustTier::Official
        );
    }

    #[test]
    fn owner_based_unverified() {
        assert_eq!(
            determine_trust_tier_from_owner("someuser"),
            TrustTier::Unverified
        );
    }

    #[test]
    fn parse_source_ref_with_tag() {
        let (base, git_ref) = parse_source_ref("someone/fledge-deploy@v1.2.0");
        assert_eq!(base, "someone/fledge-deploy");
        assert_eq!(git_ref, Some("v1.2.0"));
    }

    #[test]
    fn parse_source_ref_without_tag() {
        let (base, git_ref) = parse_source_ref("someone/fledge-deploy");
        assert_eq!(base, "someone/fledge-deploy");
        assert!(git_ref.is_none());
    }

    #[test]
    fn parse_source_ref_full_url_with_tag() {
        let (base, git_ref) =
            parse_source_ref("https://github.com/someone/fledge-deploy.git@v2.0.0");
        assert_eq!(base, "https://github.com/someone/fledge-deploy.git");
        assert_eq!(git_ref, Some("v2.0.0"));
    }

    #[test]
    fn parse_source_ref_credential_url_no_split() {
        let (base, git_ref) = parse_source_ref("https://user:token@github.com/owner/repo.git");
        assert_eq!(base, "https://user:token@github.com/owner/repo.git");
        assert!(git_ref.is_none());
    }

    #[test]
    fn labels() {
        assert_eq!(TrustTier::Official.label(), "official");
        assert_eq!(TrustTier::Team.label(), "team");
        assert_eq!(TrustTier::Unverified.label(), "unverified");
    }

    #[test]
    fn team_member_shorthand() {
        assert_eq!(
            determine_trust_tier("0xLeif/fledge-plugin-thing"),
            TrustTier::Team
        );
    }

    #[test]
    fn team_member_case_insensitive() {
        assert_eq!(
            determine_trust_tier("0xleif/fledge-plugin-thing"),
            TrustTier::Team
        );
    }

    #[test]
    fn team_member_full_url() {
        assert_eq!(
            determine_trust_tier("https://github.com/0xLeif/fledge-plugin-thing"),
            TrustTier::Team
        );
    }

    #[test]
    fn team_member_with_ref() {
        assert_eq!(
            determine_trust_tier("0xLeif/fledge-plugin-thing@v0.1.0"),
            TrustTier::Team
        );
    }

    #[test]
    fn owner_based_team() {
        assert_eq!(determine_trust_tier_from_owner("0xLeif"), TrustTier::Team);
        assert_eq!(determine_trust_tier_from_owner("0xleif"), TrustTier::Team);
    }

    #[test]
    fn official_takes_precedence_over_team() {
        // Defensive: if a member's username were ever added to OFFICIAL_ORGS,
        // the official tier wins. Documents the precedence rule.
        assert_eq!(determine_trust_tier("CorvidLabs/repo"), TrustTier::Official);
    }

    #[test]
    fn every_team_member_classifies_as_team() {
        // Smoke test: catches typos in TEAM_MEMBERS by classifying each entry.
        for member in TEAM_MEMBERS {
            let source = format!("{member}/some-repo");
            assert_eq!(
                determine_trust_tier(&source),
                TrustTier::Team,
                "expected {source} to classify as Team"
            );
        }
    }

    #[test]
    fn config_org_classifies_as_team() {
        let config = TrustConfig {
            orgs: vec!["my-company".to_string()],
            users: vec![],
        };
        assert_eq!(
            classify_source("my-company/fledge-plugin-foo", &config),
            TrustTier::Team
        );
    }

    #[test]
    fn config_user_classifies_as_team() {
        let config = TrustConfig {
            orgs: vec![],
            users: vec!["corvid-agent".to_string()],
        };
        assert_eq!(
            classify_source("corvid-agent/fledge-plugin-codegolf", &config),
            TrustTier::Team
        );
    }

    #[test]
    fn config_org_case_insensitive() {
        let config = TrustConfig {
            orgs: vec!["MyCompany".to_string()],
            users: vec![],
        };
        assert_eq!(
            classify_source("mycompany/fledge-plugin-foo", &config),
            TrustTier::Team
        );
    }

    #[test]
    fn config_user_case_insensitive() {
        let config = TrustConfig {
            orgs: vec![],
            users: vec!["Corvid-Agent".to_string()],
        };
        assert_eq!(
            classify_source("corvid-agent/fledge-plugin-foo", &config),
            TrustTier::Team
        );
    }

    #[test]
    fn config_org_full_url() {
        let config = TrustConfig {
            orgs: vec!["my-company".to_string()],
            users: vec![],
        };
        assert_eq!(
            classify_source("https://github.com/my-company/fledge-plugin-foo", &config),
            TrustTier::Team
        );
    }

    #[test]
    fn config_user_with_ref() {
        let config = TrustConfig {
            orgs: vec![],
            users: vec!["corvid-agent".to_string()],
        };
        assert_eq!(
            classify_source("corvid-agent/fledge-plugin-foo@v1.0.0", &config),
            TrustTier::Team
        );
    }

    #[test]
    fn config_does_not_grant_official() {
        let config = TrustConfig {
            orgs: vec!["my-company".to_string()],
            users: vec![],
        };
        assert_eq!(
            classify_source("my-company/fledge-plugin-foo", &config),
            TrustTier::Team
        );
        assert_ne!(
            classify_source("my-company/fledge-plugin-foo", &config),
            TrustTier::Official
        );
    }

    #[test]
    fn official_takes_precedence_over_config() {
        let config = TrustConfig {
            orgs: vec!["CorvidLabs".to_string()],
            users: vec![],
        };
        assert_eq!(
            classify_source("CorvidLabs/fledge-plugin-foo", &config),
            TrustTier::Official
        );
    }

    #[test]
    fn config_owner_based_team() {
        let config = TrustConfig {
            orgs: vec!["my-company".to_string()],
            users: vec!["corvid-agent".to_string()],
        };
        assert_eq!(classify_owner("my-company", &config), TrustTier::Team);
        assert_eq!(classify_owner("corvid-agent", &config), TrustTier::Team);
    }

    #[test]
    fn empty_config_no_effect() {
        let config = TrustConfig::default();
        assert_eq!(
            classify_source("someuser/fledge-plugin-thing", &config),
            TrustTier::Unverified
        );
    }
}
