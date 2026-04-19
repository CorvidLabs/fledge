use anyhow::{Result, bail};
use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub fn parse_version(s: &str) -> Result<Version> {
    let s = s.strip_prefix('v').unwrap_or(s);
    let parts: Vec<&str> = s.split('.').collect();

    if parts.len() != 3 {
        bail!("Invalid version '{}': expected MAJOR.MINOR.PATCH", s);
    }

    let major = parts[0]
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("Invalid major version in '{}'", s))?;
    let minor = parts[1]
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("Invalid minor version in '{}'", s))?;
    let patch = parts[2]
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("Invalid patch version in '{}'", s))?;

    Ok(Version {
        major,
        minor,
        patch,
    })
}

pub fn check_fledge_version(min_version: &str) -> Result<()> {
    let required = parse_version(min_version)?;
    let current = parse_version(env!("CARGO_PKG_VERSION"))?;

    if current < required {
        bail!(
            "This template requires fledge {} or newer (you have {}). \
             Upgrade with: cargo install fledge",
            required,
            current
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_version() {
        let v = parse_version("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn parse_with_v_prefix() {
        let v = parse_version("v0.3.0").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 3);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn parse_invalid_too_few_parts() {
        assert!(parse_version("1.2").is_err());
    }

    #[test]
    fn parse_invalid_non_numeric() {
        assert!(parse_version("1.2.beta").is_err());
    }

    #[test]
    fn parse_invalid_empty() {
        assert!(parse_version("").is_err());
    }

    #[test]
    fn version_ordering_patch() {
        let a = parse_version("0.2.0").unwrap();
        let b = parse_version("0.2.1").unwrap();
        assert!(b > a);
    }

    #[test]
    fn version_ordering_minor() {
        let a = parse_version("0.2.9").unwrap();
        let b = parse_version("0.3.0").unwrap();
        assert!(b > a);
    }

    #[test]
    fn version_ordering_major() {
        let a = parse_version("0.99.99").unwrap();
        let b = parse_version("1.0.0").unwrap();
        assert!(b > a);
    }

    #[test]
    fn version_ordering_equal() {
        let a = parse_version("1.2.3").unwrap();
        let b = parse_version("1.2.3").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn check_compatible() {
        // Current version is 0.2.1, requiring 0.1.0 should pass
        let result = check_fledge_version("0.1.0");
        assert!(result.is_ok());
    }

    #[test]
    fn check_equal_version() {
        let current = env!("CARGO_PKG_VERSION");
        let result = check_fledge_version(current);
        assert!(result.is_ok());
    }

    #[test]
    fn check_incompatible() {
        let result = check_fledge_version("99.0.0");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("99.0.0"));
        assert!(err.contains("Upgrade"));
    }

    #[test]
    fn version_display() {
        let v = parse_version("1.2.3").unwrap();
        assert_eq!(format!("{}", v), "1.2.3");
    }
}
