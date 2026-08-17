//! Tests for the isolation guarantees `TempEnv` advertises.
//!
//! Everything here drives the *spawned* `fledge` binary, which is the whole
//! point: the `#[cfg(test)]` thread-local redirection used by unit tests stops
//! at the process boundary, so the guarantees an integration test relies on
//! have to be proven through a real subprocess.
//!
//! The `FLEDGE_TEST_GITHUB_*` redirection is a debug-build-only hook (see
//! `github::test_endpoint_override`). Tests that depend on it skip themselves
//! under `cargo test --release` rather than fall through to the real
//! github.com; `endpoint_override_is_absent_from_release_builds` is what covers
//! that direction.

mod common;
use common::*;

use std::path::Path;
use std::process::Command;

/// A GitHub `/search/repositories` payload for a fictional template repo.
const SEARCH_FIXTURE: &str = r#"{"items":[{
    "name":"fledge-mock-template",
    "owner":{"login":"loopback-only"},
    "description":"served by the loopback mock, not github.com",
    "stargazers_count":3,
    "html_url":"http://127.0.0.1/loopback-only/fledge-mock-template",
    "topics":["fledge-template"]
}]}"#;

// ──────────────────────────────────────────────────────────
// The REST base really is redirected through the spawned binary
// ──────────────────────────────────────────────────────────

#[test]
fn github_api_base_override_routes_a_spawned_command_to_the_mock() {
    if !github_redirection_supported() {
        return;
    }
    let server = MockHttp::start(SEARCH_FIXTURE);
    let env = TempEnv::new().with_github_api_base(server.url());

    let output = env.run(&["templates", "search", "mock", "--json"]);
    assert!(
        output.status.success(),
        "templates search failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let results = parsed["results"].as_array().expect("results array");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["owner"], "loopback-only");
    assert_eq!(results[0]["name"], "fledge-mock-template");

    // Proof the request went to the loopback mock rather than api.github.com.
    let requests = server.requests();
    assert_eq!(requests.len(), 1, "requests: {requests:?}");
    assert!(
        requests[0].starts_with("GET /search/repositories?"),
        "unexpected request: {}",
        requests[0]
    );
}

#[test]
fn default_temp_env_points_github_at_a_dead_port() {
    if !github_redirection_supported() {
        return;
    }
    // No `with_github_api_base`: the default is a closed loopback port, so a
    // GitHub-touching command fails locally instead of reaching the network.
    let env = TempEnv::new();
    let output = env.run(&["templates", "search", "rust", "--json"]);
    assert!(
        !output.status.success(),
        "search should fail against a dead port, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("127.0.0.1") || stderr.contains("failed") || stderr.contains("parsing"),
        "expected a local connection failure, got: {stderr}"
    );
}

#[test]
fn endpoint_override_is_absent_from_release_builds() {
    // The other direction of the same contract: the hook exists only in debug
    // builds, so a release binary keeps the production endpoints no matter
    // what the environment says. `github_redirection_supported()` is the
    // single place that knowledge lives; the unit tests in `src/github.rs`
    // assert the constant fallback itself.
    assert_eq!(github_redirection_supported(), cfg!(debug_assertions));
}

// Rejection of a non-loopback override (`https://evil.example`, the
// `http://127.0.0.1@evil.example` userinfo trick, …) is asserted by the unit
// tests in `src/github.rs`, not here: a rejected value falls back to the real
// endpoint by construction, so the only way to observe it through a spawned
// binary would be to let a request reach api.github.com.

// ──────────────────────────────────────────────────────────
// Remote template fetch: a real `git clone`, from a local bare repo
// ──────────────────────────────────────────────────────────

/// `git clone` is a subprocess, so no HTTP mock can stand in for github.com.
/// Redirecting the git remote base at a directory of bare repos does the job
/// instead: the production clone path runs unmodified and never leaves disk.
///
/// Unix-only: the clone lands in `dirs::cache_dir()`, which follows
/// `XDG_CACHE_HOME`/`HOME` on Linux and macOS but a known-folder API on
/// Windows that no environment variable can redirect — running it there would
/// write into the developer's real cache.
#[cfg(not(windows))]
#[test]
fn templates_init_clones_a_remote_template_from_a_local_bare_repo() {
    if !github_redirection_supported() {
        return;
    }
    let remotes = tempfile::tempdir().unwrap();
    make_bare_template_repo(remotes.path(), "loopback-only", "fledge-mock-template");

    let env = TempEnv::new().with_github_remote_base(remotes.path().to_str().unwrap());
    let out_dir = tempfile::tempdir().unwrap();
    let output = env.run(&[
        "templates",
        "init",
        "mock-project",
        "--template",
        "loopback-only/fledge-mock-template",
        "--output",
        out_dir.path().to_str().unwrap(),
        "--no-git",
        "--yes",
        "--json",
    ]);
    assert!(
        output.status.success(),
        "templates init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["action"], "init");
    assert_eq!(parsed["template"]["name"], "fledge-mock-template");

    let project = out_dir.path().join("mock-project");
    let readme = std::fs::read_to_string(project.join("README.md")).unwrap();
    assert!(
        readme.contains("mock-project"),
        "template was not rendered: {readme}"
    );
}

/// Build `<root>/<owner>/<repo>.git` as a bare repo holding a one-file
/// template, so `{remote_base}/{owner}/{repo}.git` resolves to it.
#[cfg(not(windows))]
fn make_bare_template_repo(root: &Path, owner: &str, repo: &str) {
    let work = tempfile::tempdir().unwrap();
    let work = work.path();

    std::fs::write(
        work.join("template.toml"),
        "[template]\nname = \"fledge-mock-template\"\n\
         description = \"loopback template\"\n\n\
         [files]\nrender = [\"**/*.md\"]\nignore = [\"template.toml\"]\n",
    )
    .unwrap();
    std::fs::write(work.join("README.md.tera"), "# {{ project_name }}\n").unwrap();

    git(work, &["init", "-q"]);
    git(work, &["add", "."]);
    git(work, &["commit", "-q", "-m", "template"]);

    let bare = root.join(owner).join(format!("{repo}.git"));
    std::fs::create_dir_all(bare.parent().unwrap()).unwrap();
    git(
        work,
        &["clone", "-q", "--bare", ".", bare.to_str().unwrap()],
    );
}

#[cfg(not(windows))]
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        // Same identity isolation `TempEnv` gives the binary: these commits
        // must not need (or read) the developer's git config.
        .env("GIT_AUTHOR_NAME", "fledge test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "fledge test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .env("GIT_CONFIG_GLOBAL", dir.join("nonexistent-gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("spawn git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
