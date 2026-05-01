use super::*;
use crate::trust::parse_source_ref;

#[test]
fn unsupported_protocol_version_returns_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    let result = apply_protocol(
        Some("fledge-v2"),
        "my-plugin".to_string(),
        "1.0.0".to_string(),
        tmp.path().to_path_buf(),
        PluginCapabilities::default(),
    );
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("fledge-v2"),
        "error should mention the protocol: {msg}"
    );
    assert!(
        msg.contains("my-plugin"),
        "error should mention the plugin: {msg}"
    );
}

#[test]
fn supported_protocol_fledge_v1_returns_info() {
    let tmp = tempfile::TempDir::new().unwrap();
    let result = apply_protocol(
        Some("fledge-v1"),
        "my-plugin".to_string(),
        "1.0.0".to_string(),
        tmp.path().to_path_buf(),
        PluginCapabilities::default(),
    );
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

#[test]
fn no_protocol_declared_returns_none_for_legacy_fallback() {
    let tmp = tempfile::TempDir::new().unwrap();
    let result = apply_protocol(
        None,
        "legacy-plugin".to_string(),
        "1.0.0".to_string(),
        tmp.path().to_path_buf(),
        PluginCapabilities::default(),
    );
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn resolve_plugin_source_dir_walks_symlink_to_plugin_root() {
    // Mirror the post-install layout:
    //   <root>/plugins/my-plugin/bin/fledge-my-plugin     ← real binary
    //   <root>/plugins/bin/fledge-my-plugin               ← shared symlink
    //
    // resolve_plugin_source_dir(<symlink>) should return
    //   <root>/plugins/my-plugin
    let tmp = tempfile::TempDir::new().unwrap();
    let plugin_root = tmp.path().join("plugins").join("my-plugin");
    let plugin_bin_dir = plugin_root.join("bin");
    std::fs::create_dir_all(&plugin_bin_dir).unwrap();
    let real_binary = plugin_bin_dir.join("fledge-my-plugin");
    std::fs::write(&real_binary, "#!/bin/sh\nexit 0\n").unwrap();

    let shared_bin_dir = tmp.path().join("plugins").join("bin");
    std::fs::create_dir_all(&shared_bin_dir).unwrap();
    let symlink = shared_bin_dir.join("fledge-my-plugin");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real_binary, &symlink).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&real_binary, &symlink).unwrap();

    let resolved = resolve_plugin_source_dir(&symlink).expect("resolve should succeed");
    // Canonicalize the expected path because TempDir may live under /tmp,
    // which is itself a symlink to /private/tmp on macOS.
    let expected = std::fs::canonicalize(&plugin_root).unwrap();
    assert_eq!(resolved, expected);
}

#[test]
fn resolve_plugin_source_dir_handles_non_symlink_path() {
    // Direct PATH-resolved fledge-<name> binaries (not installed via
    // `fledge plugins install`) still get a sensible plugin dir — the
    // parent of the parent of the binary.
    let tmp = tempfile::TempDir::new().unwrap();
    let bin_dir = tmp.path().join("project").join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let bin = bin_dir.join("fledge-direct");
    std::fs::write(&bin, "").unwrap();

    let resolved = resolve_plugin_source_dir(&bin).expect("should resolve");
    let expected = std::fs::canonicalize(tmp.path().join("project")).unwrap();
    assert_eq!(resolved, expected);
}

#[test]
fn default_plugins_are_well_formed() {
    assert!(!DEFAULT_PLUGINS.is_empty());
    for src in DEFAULT_PLUGINS {
        let (owner_repo, git_ref) = parse_source_ref(src);
        assert!(
            owner_repo.contains('/'),
            "DEFAULT_PLUGINS entry '{src}' must be owner/repo"
        );
        if let Some(r) = git_ref {
            assert!(
                r.starts_with('v'),
                "DEFAULT_PLUGINS pinned ref '{r}' should be a version tag (v...)"
            );
        }
    }
}

#[test]
fn install_action_rejects_source_with_defaults() {
    let err = install_action(Some("someone/foo"), false, true, false)
        .unwrap_err()
        .to_string();
    assert!(err.contains("--defaults"));
}

#[test]
fn install_action_requires_source_or_defaults() {
    let err = install_action(None, false, false, false)
        .unwrap_err()
        .to_string();
    assert!(err.contains("--defaults"));
}

#[test]
fn update_plugins_rejects_name_with_defaults() {
    let err = update_plugins(Some("foo"), true, false)
        .unwrap_err()
        .to_string();
    assert!(err.contains("--defaults"));
}

#[test]
fn default_source_match_recognizes_shorthand() {
    // Mirrors the closure in `update_plugins` — keep the two in sync.
    // Must match stored sources in bare shorthand, normalized URL, or
    // URL-without-.git form, even when DEFAULT_PLUGINS entries carry
    // an `@ref` suffix.
    let is_default = |source: &str| -> bool {
        DEFAULT_PLUGINS.iter().any(|d| {
            let (base, _) = parse_source_ref(d);
            source == base
                || source == *d
                || source == normalize_source(d)
                || source.trim_end_matches(".git").ends_with(base)
        })
    };
    assert!(is_default("CorvidLabs/fledge-plugin-github"));
    assert!(is_default(
        "https://github.com/CorvidLabs/fledge-plugin-github.git"
    ));
    assert!(is_default(
        "https://github.com/CorvidLabs/fledge-plugin-github"
    ));
    assert!(!is_default("CorvidLabs/fledge-plugin-figma"));
    assert!(!is_default("someone/random-plugin"));
}

#[test]
fn normalize_github_shorthand() {
    assert_eq!(
        normalize_source("someone/fledge-deploy"),
        "https://github.com/someone/fledge-deploy.git"
    );
}

#[test]
fn normalize_github_shorthand_with_ref() {
    assert_eq!(
        normalize_source("someone/fledge-deploy@v1.0.0"),
        "https://github.com/someone/fledge-deploy.git"
    );
}

#[test]
fn normalize_full_url() {
    let url = "https://github.com/someone/fledge-deploy.git";
    assert_eq!(normalize_source(url), url);
}

#[test]
fn normalize_ssh_url() {
    let url = "git@github.com:someone/fledge-deploy.git";
    assert_eq!(normalize_source(url), url);
}

#[test]
fn extract_name_from_github_shorthand() {
    assert_eq!(
        extract_name_from_source("someone/fledge-deploy"),
        "fledge-deploy"
    );
}

#[test]
fn extract_name_with_ref() {
    assert_eq!(
        extract_name_from_source("someone/fledge-deploy@v1.0.0"),
        "fledge-deploy"
    );
}

#[test]
fn extract_name_from_full_url() {
    assert_eq!(
        extract_name_from_source("https://github.com/someone/fledge-deploy.git"),
        "fledge-deploy"
    );
}

#[test]
fn extract_name_plain() {
    assert_eq!(extract_name_from_source("my-plugin"), "my-plugin");
}

#[test]
fn plugin_dir_is_under_config() {
    let dir = plugins_dir();
    assert!(dir.to_string_lossy().contains("fledge"));
    assert!(dir.to_string_lossy().contains("plugins"));
}

#[test]
fn bin_dir_is_under_plugins() {
    let dir = plugin_bin_dir();
    assert!(dir.ends_with("plugins/bin"));
}

#[test]
fn empty_registry_has_no_plugins() {
    let registry = PluginsRegistry {
        plugins: Vec::new(),
    };
    assert!(registry.plugins.is_empty());
}

#[test]
fn registry_roundtrip() {
    let registry = PluginsRegistry {
        plugins: vec![PluginEntry {
            name: "fledge-test".to_string(),
            source: "someone/fledge-test".to_string(),
            version: "1.0.0".to_string(),
            installed: "2026-04-20".to_string(),
            commands: vec!["test-cmd".to_string()],
            pinned_ref: None,
            capabilities: None,
        }],
    };
    let serialized = toml::to_string_pretty(&registry).unwrap();
    let deserialized: PluginsRegistry = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.plugins.len(), 1);
    assert_eq!(deserialized.plugins[0].name, "fledge-test");
    assert_eq!(deserialized.plugins[0].commands, vec!["test-cmd"]);
    assert!(deserialized.plugins[0].pinned_ref.is_none());
    assert!(deserialized.plugins[0].capabilities.is_none());
}

#[test]
fn registry_roundtrip_with_pinned_ref() {
    let registry = PluginsRegistry {
        plugins: vec![PluginEntry {
            name: "fledge-test".to_string(),
            source: "someone/fledge-test".to_string(),
            version: "1.0.0".to_string(),
            installed: "2026-04-20".to_string(),
            commands: vec!["test-cmd".to_string()],
            pinned_ref: Some("v1.0.0".to_string()),
            capabilities: None,
        }],
    };
    let serialized = toml::to_string_pretty(&registry).unwrap();
    let deserialized: PluginsRegistry = toml::from_str(&serialized).unwrap();
    assert_eq!(
        deserialized.plugins[0].pinned_ref,
        Some("v1.0.0".to_string())
    );
}

#[test]
fn registry_roundtrip_with_capabilities() {
    let registry = PluginsRegistry {
        plugins: vec![PluginEntry {
            name: "fledge-deploy".to_string(),
            source: "someone/fledge-deploy".to_string(),
            version: "1.0.0".to_string(),
            installed: "2026-04-22".to_string(),
            commands: vec!["deploy".to_string()],
            pinned_ref: None,
            capabilities: Some(PluginCapabilities {
                exec: true,
                store: true,
                metadata: false,
            }),
        }],
    };
    let serialized = toml::to_string_pretty(&registry).unwrap();
    let deserialized: PluginsRegistry = toml::from_str(&serialized).unwrap();
    let caps = deserialized.plugins[0].capabilities.as_ref().unwrap();
    assert!(caps.exec);
    assert!(caps.store);
    assert!(!caps.metadata);
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
fn parse_source_ref_with_branch() {
    let (base, git_ref) = parse_source_ref("someone/fledge-deploy@main");
    assert_eq!(base, "someone/fledge-deploy");
    assert_eq!(git_ref, Some("main"));
}

#[test]
fn parse_source_ref_full_url_with_tag() {
    let (base, git_ref) = parse_source_ref("https://github.com/someone/fledge-deploy.git@v2.0.0");
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
fn validate_plugin_name_rejects_dotdot() {
    assert!(validate_plugin_name("..").is_err());
}

#[test]
fn validate_plugin_name_rejects_hidden() {
    assert!(validate_plugin_name(".secret").is_err());
}

#[test]
fn validate_plugin_name_rejects_slashes() {
    assert!(validate_plugin_name("../etc").is_err());
}

#[test]
fn validate_plugin_name_accepts_normal() {
    assert!(validate_plugin_name("fledge-deploy").is_ok());
}

#[test]
fn validate_command_name_rejects_slashes() {
    assert!(validate_command_name("../evil").is_err());
    assert!(validate_command_name("foo/bar").is_err());
}

#[test]
fn validate_command_name_rejects_dot_prefix() {
    assert!(validate_command_name(".hidden").is_err());
}

#[test]
fn validate_command_name_rejects_dash_prefix() {
    assert!(validate_command_name("-flag").is_err());
}

#[test]
fn validate_command_name_accepts_normal() {
    assert!(validate_command_name("deploy").is_ok());
    assert!(validate_command_name("my-tool").is_ok());
    assert!(validate_command_name("tool_v2").is_ok());
}

#[test]
fn parse_plugin_manifest() {
    let manifest_str = r#"
[plugin]
name = "fledge-deploy"
version = "0.1.0"
description = "Deploy to cloud"
author = "someone"

[[commands]]
name = "deploy"
description = "Deploy the project"
binary = "fledge-deploy"
"#;
    let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
    assert_eq!(manifest.plugin.name, "fledge-deploy");
    assert_eq!(manifest.plugin.version, "0.1.0");
    assert_eq!(manifest.commands.len(), 1);
    assert_eq!(manifest.commands[0].name, "deploy");
}

#[test]
fn parse_minimal_manifest() {
    let manifest_str = r#"
[plugin]
name = "fledge-minimal"
version = "0.1.0"
"#;
    let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
    assert_eq!(manifest.plugin.name, "fledge-minimal");
    assert!(manifest.commands.is_empty());
    assert!(!manifest.capabilities.exec);
    assert!(!manifest.capabilities.store);
    assert!(!manifest.capabilities.metadata);
}

#[test]
fn parse_manifest_with_capabilities() {
    let manifest_str = r#"
[plugin]
name = "fledge-deploy"
version = "0.1.0"
protocol = "fledge-v1"

[capabilities]
exec = true
store = true
metadata = false

[[commands]]
name = "deploy"
binary = "fledge-deploy"
"#;
    let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
    assert!(manifest.capabilities.exec);
    assert!(manifest.capabilities.store);
    assert!(!manifest.capabilities.metadata);
}

#[test]
fn parse_manifest_partial_capabilities() {
    let manifest_str = r#"
[plugin]
name = "fledge-stats"
version = "0.1.0"
protocol = "fledge-v1"

[capabilities]
store = true
"#;
    let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
    assert!(!manifest.capabilities.exec);
    assert!(manifest.capabilities.store);
    assert!(!manifest.capabilities.metadata);
}

#[test]
fn parse_manifest_multiple_commands() {
    let manifest_str = r#"
[plugin]
name = "fledge-cloud"
version = "0.2.0"

[[commands]]
name = "deploy"
description = "Deploy"
binary = "bin/deploy"

[[commands]]
name = "rollback"
description = "Rollback"
binary = "bin/rollback"
"#;
    let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
    assert_eq!(manifest.commands.len(), 2);
    assert_eq!(manifest.commands[0].name, "deploy");
    assert_eq!(manifest.commands[1].name, "rollback");
}

#[test]
fn resolve_nonexistent_plugin() {
    assert!(resolve_plugin_command("definitely-not-installed-xyz").is_none());
}

#[test]
fn which_nonexistent() {
    assert!(which_fledge_plugin("definitely-not-installed-xyz").is_none());
}

#[test]
fn install_dir_with_tempdir() {
    use std::fs;
    let tmp = tempfile::tempdir().unwrap();
    let plugin_dir = tmp.path().join("test-plugin");
    fs::create_dir_all(&plugin_dir).unwrap();

    let manifest = r#"
[plugin]
name = "test-plugin"
version = "0.1.0"
"#;
    fs::write(plugin_dir.join("plugin.toml"), manifest).unwrap();

    let content = fs::read_to_string(plugin_dir.join("plugin.toml")).unwrap();
    let parsed: PluginManifest = toml::from_str(&content).unwrap();
    assert_eq!(parsed.plugin.name, "test-plugin");
}

#[test]
fn registry_path_exists() {
    let path = registry_path();
    assert!(path.to_string_lossy().contains("plugins.toml"));
}

#[test]
fn plugins_dir_structure() {
    let pd = plugins_dir();
    let bd = plugin_bin_dir();
    assert!(bd.starts_with(&pd));
}

#[test]
fn detect_rust_build() {
    use std::fs;
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"x\"").unwrap();
    let result = detect_build_command(tmp.path());
    assert!(result.is_some());
    let (lang, cmd) = result.unwrap();
    assert_eq!(lang, "Rust");
    assert_eq!(cmd[0], "cargo");
}

#[test]
fn detect_swift_build() {
    use std::fs;
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Package.swift"), "// swift").unwrap();
    let result = detect_build_command(tmp.path());
    assert!(result.is_some());
    let (lang, _) = result.unwrap();
    assert_eq!(lang, "Swift");
}

#[test]
fn detect_go_build() {
    use std::fs;
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("go.mod"), "module x").unwrap();
    let result = detect_build_command(tmp.path());
    assert!(result.is_some());
    let (lang, _) = result.unwrap();
    assert_eq!(lang, "Go");
}

#[test]
fn detect_node_build() {
    use std::fs;
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("package.json"), "{}").unwrap();
    let result = detect_build_command(tmp.path());
    assert!(result.is_some());
    let (lang, _) = result.unwrap();
    assert_eq!(lang, "Node");
}

#[test]
fn detect_no_build_system() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(detect_build_command(tmp.path()).is_none());
}

#[test]
fn parse_manifest_with_build_hook() {
    let manifest_str = r#"
[plugin]
name = "fledge-compiled"
version = "0.1.0"

[[commands]]
name = "compiled"
binary = "target/release/fledge-compiled"

[hooks]
build = "cargo build --release"
post_install = "scripts/setup.sh"
"#;
    let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
    assert_eq!(
        manifest.hooks.build.as_deref(),
        Some("cargo build --release")
    );
    assert_eq!(
        manifest.hooks.post_install.as_deref(),
        Some("scripts/setup.sh")
    );
}

#[test]
fn parse_manifest_with_lifecycle_hooks() {
    let manifest_str = r#"
[plugin]
name = "fledge-lint"
version = "0.1.0"

[hooks]
pre_init = "scripts/pre-init.sh"
post_work_start = "scripts/setup-hooks.sh"
pre_push = "scripts/lint-all.sh"
"#;
    let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
    assert_eq!(
        manifest.hooks.pre_init.as_deref(),
        Some("scripts/pre-init.sh")
    );
    assert_eq!(
        manifest.hooks.post_work_start.as_deref(),
        Some("scripts/setup-hooks.sh")
    );
    assert_eq!(
        manifest.hooks.pre_push.as_deref(),
        Some("scripts/lint-all.sh")
    );
}

#[test]
fn parse_manifest_lifecycle_hooks_default_none() {
    let manifest_str = r#"
[plugin]
name = "fledge-simple"
version = "0.1.0"
"#;
    let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
    assert!(manifest.hooks.pre_init.is_none());
    assert!(manifest.hooks.post_work_start.is_none());
    assert!(manifest.hooks.pre_push.is_none());
}

#[test]
fn create_plugin_scaffolds_files() {
    use std::fs;
    let tmp = tempfile::TempDir::new().unwrap();
    create_plugin("my-plugin", tmp.path(), Some("Test plugin"), true, false).unwrap();

    let target = tmp.path().join("my-plugin");
    assert!(target.join("plugin.toml").exists());
    assert!(target.join("README.md").exists());
    assert!(target.join(".gitignore").exists());
    assert!(target.join("bin").is_dir());
    assert!(target.join("bin/my-plugin").exists());

    let content = fs::read_to_string(target.join("plugin.toml")).unwrap();
    let manifest: PluginManifest = toml::from_str(&content).unwrap();
    assert_eq!(manifest.plugin.name, "my-plugin");
    assert_eq!(manifest.plugin.version, "0.1.0");
    assert_eq!(manifest.commands.len(), 1);
}

#[test]
fn create_plugin_fails_if_exists() {
    use std::fs;
    let tmp = tempfile::TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("existing")).unwrap();
    let result = create_plugin("existing", tmp.path(), None, true, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn validate_valid_plugin() {
    let tmp = tempfile::TempDir::new().unwrap();
    create_plugin("test-plugin", tmp.path(), Some("Test"), true, false).unwrap();

    let result = validate_plugin(&tmp.path().join("test-plugin"), false, false);
    assert!(result.is_ok());
}

#[test]
fn validate_missing_plugin_toml() {
    let tmp = tempfile::TempDir::new().unwrap();
    let result = validate_plugin(tmp.path(), false, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No plugin.toml"));
}

#[test]
fn validate_empty_name_is_error() {
    use std::fs;
    let tmp = tempfile::TempDir::new().unwrap();
    fs::write(
        tmp.path().join("plugin.toml"),
        r#"
[plugin]
name = ""
version = "0.1.0"
"#,
    )
    .unwrap();

    let result = validate_plugin(tmp.path(), false, false);
    assert!(result.is_err());
}

#[test]
fn validate_missing_binary_is_error() {
    use std::fs;
    let tmp = tempfile::TempDir::new().unwrap();
    fs::write(
        tmp.path().join("plugin.toml"),
        r#"
[plugin]
name = "test"
version = "0.1.0"

[[commands]]
name = "test"
description = "Test"
binary = "bin/nonexistent"
"#,
    )
    .unwrap();

    let result = validate_plugin(tmp.path(), false, false);
    assert!(result.is_err());
}

#[test]
fn validate_missing_binary_with_build_hook_is_warning() {
    use std::fs;
    let tmp = tempfile::TempDir::new().unwrap();
    fs::write(
        tmp.path().join("plugin.toml"),
        r#"
[plugin]
name = "test"
version = "0.1.0"
description = "Test"
author = "tester"

[[commands]]
name = "test"
description = "Test"
binary = "target/release/test"

[hooks]
build = "cargo build --release"
"#,
    )
    .unwrap();

    // non-strict: passes with warning
    let result = validate_plugin(tmp.path(), false, false);
    assert!(result.is_ok());

    // strict: fails on warning
    let result = validate_plugin(tmp.path(), true, false);
    assert!(result.is_err());
}

#[test]
fn validate_json_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    create_plugin("json-test", tmp.path(), Some("Test"), true, false).unwrap();

    let result = validate_plugin(&tmp.path().join("json-test"), false, true);
    assert!(result.is_ok());
}

#[test]
fn trust_tier_official_github_shorthand() {
    use crate::trust::{determine_trust_tier, TrustTier};
    assert_eq!(
        determine_trust_tier("CorvidLabs/fledge-plugin-deploy"),
        TrustTier::Official
    );
}

#[test]
fn trust_tier_official_full_url() {
    use crate::trust::{determine_trust_tier, TrustTier};
    assert_eq!(
        determine_trust_tier("https://github.com/CorvidLabs/fledge-plugin-deploy.git"),
        TrustTier::Official
    );
}

#[test]
fn trust_tier_official_ssh_url() {
    use crate::trust::{determine_trust_tier, TrustTier};
    assert_eq!(
        determine_trust_tier("git@github.com:CorvidLabs/fledge-plugin-deploy.git"),
        TrustTier::Official
    );
}

#[test]
fn trust_tier_official_with_ref() {
    use crate::trust::{determine_trust_tier, TrustTier};
    assert_eq!(
        determine_trust_tier("CorvidLabs/fledge-plugin-deploy@v1.0.0"),
        TrustTier::Official
    );
}

#[test]
fn trust_tier_official_lowercase() {
    use crate::trust::{determine_trust_tier, TrustTier};
    assert_eq!(
        determine_trust_tier("corvidlabs/fledge-plugin-deploy"),
        TrustTier::Official
    );
}

#[test]
fn trust_tier_unverified_third_party() {
    use crate::trust::{determine_trust_tier, TrustTier};
    assert_eq!(
        determine_trust_tier("someone/fledge-plugin-cool"),
        TrustTier::Unverified
    );
}

#[test]
fn trust_tier_unverified_full_url() {
    use crate::trust::{determine_trust_tier, TrustTier};
    assert_eq!(
        determine_trust_tier("https://github.com/random-user/fledge-deploy.git"),
        TrustTier::Unverified
    );
}

#[test]
fn trust_tier_unverified_no_org() {
    use crate::trust::{determine_trust_tier, TrustTier};
    assert_eq!(determine_trust_tier("local-plugin"), TrustTier::Unverified);
}

#[test]
fn trust_tier_label_strings() {
    use crate::trust::TrustTier;
    assert_eq!(TrustTier::Official.label(), "official");
    assert_eq!(TrustTier::Team.label(), "team");
    assert_eq!(TrustTier::Unverified.label(), "unverified");
}

#[test]
fn hooks_has_any_detects_build() {
    let hooks = PluginHooks {
        build: Some("cargo build".into()),
        ..Default::default()
    };
    assert!(hooks.has_any());
}

#[test]
fn hooks_has_any_detects_lifecycle() {
    let hooks = PluginHooks {
        pre_push: Some("./check.sh".into()),
        ..Default::default()
    };
    assert!(hooks.has_any());
}

#[test]
fn hooks_has_any_false_when_empty() {
    let hooks = PluginHooks::default();
    assert!(!hooks.has_any());
}

#[test]
fn hooks_iter_defined_returns_all_set_hooks() {
    let hooks = PluginHooks {
        build: Some("make".into()),
        pre_push: Some("lint".into()),
        post_install: Some("setup.sh".into()),
        ..Default::default()
    };
    let items = hooks.iter_defined();
    assert_eq!(items.len(), 3);
    assert!(items.contains(&("build", "make")));
    assert!(items.contains(&("pre_push", "lint")));
    assert!(items.contains(&("post_install", "setup.sh")));
}

#[test]
fn hooks_iter_defined_empty_when_none() {
    let hooks = PluginHooks::default();
    assert!(hooks.iter_defined().is_empty());
}

#[test]
fn parse_manifest_with_hooks() {
    let toml_str = r#"
[plugin]
name = "test-hooks"
version = "1.0.0"

[hooks]
build = "cargo build --release"
post_install = "scripts/setup.sh"
pre_push = "./lint.sh"
"#;
    let manifest: PluginManifest = toml::from_str(toml_str).unwrap();
    assert!(manifest.hooks.has_any());
    assert_eq!(
        manifest.hooks.build.as_deref(),
        Some("cargo build --release")
    );
    assert_eq!(
        manifest.hooks.post_install.as_deref(),
        Some("scripts/setup.sh")
    );
    assert_eq!(manifest.hooks.pre_push.as_deref(), Some("./lint.sh"));
    assert!(manifest.hooks.post_work_start.is_none());
}

#[test]
fn parse_manifest_hooks_and_capabilities_together() {
    let toml_str = r#"
[plugin]
name = "full-plugin"
version = "2.0.0"
protocol = "fledge-v1"

[hooks]
build = "make"
pre_init = "./init-check.sh"

[capabilities]
exec = true
store = false
"#;
    let manifest: PluginManifest = toml::from_str(toml_str).unwrap();
    assert!(manifest.hooks.has_any());
    assert!(manifest.capabilities.exec);
    assert!(!manifest.capabilities.store);
    assert_eq!(manifest.hooks.iter_defined().len(), 2);
}

#[test]
#[cfg(unix)]
fn run_hook_handles_quoted_args_with_spaces() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::TempDir::new().unwrap();
    let script = tmp.path().join("check.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\n[ \"$1\" = \"hello world\" ] || exit 1\n",
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let hook = format!("{} 'hello world'", script.display());
    let result = run_hook(tmp.path(), &hook, "test");
    assert!(
        result.is_ok(),
        "hook with quoted args should succeed: {result:?}"
    );
}

#[test]
fn run_hook_rejects_mismatched_quotes() {
    let tmp = tempfile::TempDir::new().unwrap();
    let result = run_hook(tmp.path(), "echo 'unclosed", "test");
    assert!(result.is_err(), "mismatched quotes should produce an error");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("parsing"),
        "error should mention parsing: {msg}"
    );
}
