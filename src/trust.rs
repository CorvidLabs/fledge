use console::style;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustTier {
    Official,
    Team,
    Unverified,
}

impl TrustTier {
    pub fn label(&self) -> &'static str {
        match self {
            TrustTier::Official => "official",
            TrustTier::Team => "team",
            TrustTier::Unverified => "unverified",
        }
    }

    pub fn styled_label(&self) -> console::StyledObject<&'static str> {
        match self {
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
/// same user). Adding or removing a member is a code change and requires a
/// PR — there is no runtime configuration of this list.
const TEAM_MEMBERS: &[&str] = &["0xLeif"];

fn is_team_member(owner: &str) -> bool {
    TEAM_MEMBERS.iter().any(|m| m.eq_ignore_ascii_case(owner))
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

    if let Some((org, _)) = normalized.split_once('/') {
        if OFFICIAL_ORGS.contains(&org) {
            return TrustTier::Official;
        }
        if is_team_member(org) {
            return TrustTier::Team;
        }
    }

    TrustTier::Unverified
}

pub fn determine_trust_tier_from_owner(owner: &str) -> TrustTier {
    if OFFICIAL_ORGS.contains(&owner) {
        TrustTier::Official
    } else if is_team_member(owner) {
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
}
