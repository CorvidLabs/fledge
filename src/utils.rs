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
}
