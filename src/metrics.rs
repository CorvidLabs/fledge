use anyhow::{Context, Result};
use console::style;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::run::detect_project_type;

pub struct MetricsOptions {
    pub churn: bool,
    pub tests: bool,
    pub json: bool,
    pub limit: usize,
}

#[derive(Debug, Serialize)]
struct MetricsReport {
    project_type: String,
    summary: LocSummary,
    languages: Vec<LanguageStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    churn: Option<Vec<ChurnEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tests: Option<TestStats>,
}

#[derive(Debug, Serialize)]
struct LocSummary {
    files: usize,
    lines: usize,
    code: usize,
    blank: usize,
    comment: usize,
}

#[derive(Debug, Serialize)]
struct LanguageStats {
    language: String,
    files: usize,
    lines: usize,
    code: usize,
    blank: usize,
    comment: usize,
}

#[derive(Debug, Serialize)]
struct ChurnEntry {
    file: String,
    commits: usize,
}

#[derive(Debug, Serialize)]
struct TestStats {
    test_files: usize,
    source_files: usize,
    ratio: f64,
    test_paths: Vec<String>,
}

pub fn run(opts: MetricsOptions) -> Result<()> {
    let project_dir = std::env::current_dir().context("getting current directory")?;
    let project_type = detect_project_type(&project_dir);

    if opts.churn {
        return run_churn(&project_dir, opts.limit, opts.json);
    }

    if opts.tests {
        return run_tests(&project_dir, project_type, opts.json);
    }

    let file_entries = collect_files(&project_dir)?;
    let mut lang_map: HashMap<String, LanguageStats> = HashMap::new();

    for entry in &file_entries {
        let lang = detect_language(&entry.path);
        if lang == "unknown" {
            continue;
        }
        let counts = count_lines(&entry.path, &lang);
        let stats = lang_map
            .entry(lang.clone())
            .or_insert_with(|| LanguageStats {
                language: lang,
                files: 0,
                lines: 0,
                code: 0,
                blank: 0,
                comment: 0,
            });
        stats.files += 1;
        stats.lines += counts.lines;
        stats.code += counts.code;
        stats.blank += counts.blank;
        stats.comment += counts.comment;
    }

    let mut languages: Vec<LanguageStats> = lang_map.into_values().collect();
    languages.sort_by_key(|b| std::cmp::Reverse(b.code));

    let summary = LocSummary {
        files: languages.iter().map(|l| l.files).sum(),
        lines: languages.iter().map(|l| l.lines).sum(),
        code: languages.iter().map(|l| l.code).sum(),
        blank: languages.iter().map(|l| l.blank).sum(),
        comment: languages.iter().map(|l| l.comment).sum(),
    };

    if opts.json {
        let report = MetricsReport {
            project_type: project_type.to_string(),
            summary,
            languages,
            churn: None,
            tests: None,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("\n{}\n", style("fledge metrics").bold());
    println!(
        "  {} {}\n",
        style("Project type:").dim(),
        style(project_type).cyan()
    );

    println!("  {}", style("Lines of Code").bold());
    println!(
        "  {:<16} {:>8} {:>8} {:>8} {:>8} {:>8}",
        style("Language").underlined(),
        style("Files").underlined(),
        style("Lines").underlined(),
        style("Code").underlined(),
        style("Blank").underlined(),
        style("Comment").underlined(),
    );

    for lang in &languages {
        println!(
            "  {:<16} {:>8} {:>8} {:>8} {:>8} {:>8}",
            style(&lang.language).cyan(),
            lang.files,
            lang.lines,
            style(lang.code).green(),
            lang.blank,
            lang.comment,
        );
    }

    println!(
        "  {:<16} {:>8} {:>8} {:>8} {:>8} {:>8}",
        style("Total").bold(),
        summary.files,
        summary.lines,
        style(summary.code).green().bold(),
        summary.blank,
        summary.comment,
    );

    println!();
    Ok(())
}

fn run_churn(project_dir: &Path, limit: usize, json: bool) -> Result<()> {
    let output = Command::new("git")
        .args(["log", "--format=", "--name-only", "--diff-filter=AMRC"])
        .current_dir(project_dir)
        .output()
        .context("running git log for churn analysis")?;

    if !output.status.success() {
        anyhow::bail!("git log failed — are you in a git repository?");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut file_counts: HashMap<String, usize> = HashMap::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        *file_counts.entry(line.to_string()).or_default() += 1;
    }

    let mut entries: Vec<ChurnEntry> = file_counts
        .into_iter()
        .filter(|(path, _)| Path::new(path).exists())
        .map(|(file, commits)| ChurnEntry { file, commits })
        .collect();
    entries.sort_by_key(|b| std::cmp::Reverse(b.commits));
    entries.truncate(limit);

    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    println!("\n{}\n", style("fledge metrics --churn").bold());
    println!(
        "  {:<8} {}",
        style("Commits").underlined(),
        style("File").underlined(),
    );

    for entry in &entries {
        println!(
            "  {:<8} {}",
            style(entry.commits).yellow(),
            style(&entry.file).cyan(),
        );
    }

    println!();
    Ok(())
}

fn run_tests(project_dir: &Path, project_type: &str, json: bool) -> Result<()> {
    let file_entries = collect_files(project_dir)?;
    let mut test_paths = Vec::new();
    let mut source_count = 0usize;

    for entry in &file_entries {
        let lang = detect_language(&entry.path);
        if lang == "unknown" {
            continue;
        }
        if is_test_file(&entry.rel_path, project_type) {
            test_paths.push(entry.rel_path.clone());
        } else {
            source_count += 1;
        }
    }

    let ratio = if source_count > 0 {
        test_paths.len() as f64 / source_count as f64
    } else {
        0.0
    };

    test_paths.sort();

    let stats = TestStats {
        test_files: test_paths.len(),
        source_files: source_count,
        ratio,
        test_paths: test_paths.clone(),
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    println!("\n{}\n", style("fledge metrics --tests").bold());
    println!(
        "  {} {}",
        style("Test files:").dim(),
        style(stats.test_files).green(),
    );
    println!(
        "  {} {}",
        style("Source files:").dim(),
        style(stats.source_files).cyan(),
    );
    println!(
        "  {} {:.1}%",
        style("Test ratio:").dim(),
        style(stats.ratio * 100.0).yellow(),
    );

    if !test_paths.is_empty() {
        println!("\n  {}", style("Test files:").bold());
        for path in &test_paths {
            println!("    {}", style(path).dim());
        }
    }

    println!();
    Ok(())
}

struct FileEntry {
    path: PathBuf,
    rel_path: String,
}

fn collect_files(project_dir: &Path) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    for result in walkdir::WalkDir::new(project_dir)
        .into_iter()
        .filter_entry(|e| !is_ignored_dir(e.file_name().to_str().unwrap_or("")))
    {
        let entry = result?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(project_dir)
            .unwrap_or(entry.path());
        entries.push(FileEntry {
            path: entry.path().to_path_buf(),
            rel_path: rel.to_string_lossy().to_string(),
        });
    }
    Ok(entries)
}

fn is_ignored_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | "node_modules"
            | "target"
            | "vendor"
            | "dist"
            | "build"
            | ".next"
            | "__pycache__"
            | ".tox"
            | "venv"
            | ".venv"
            | "deps"
            | "_build"
            | ".gradle"
            | ".idea"
            | ".vscode"
    )
}

fn detect_language(path: &Path) -> String {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "rs" => "Rust",
        "ts" | "tsx" => "TypeScript",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "py" => "Python",
        "go" => "Go",
        "rb" => "Ruby",
        "java" => "Java",
        "kt" | "kts" => "Kotlin",
        "swift" => "Swift",
        "c" => "C",
        "cpp" | "cc" | "cxx" => "C++",
        "h" | "hpp" => "C/C++ Header",
        "cs" => "C#",
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "scss" | "sass" => "SCSS",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "md" => "Markdown",
        "sh" | "bash" | "zsh" => "Shell",
        "sql" => "SQL",
        "ex" | "exs" => "Elixir",
        "erl" | "hrl" => "Erlang",
        "zig" => "Zig",
        "lua" => "Lua",
        "r" | "R" => "R",
        "dart" => "Dart",
        "vue" => "Vue",
        "svelte" => "Svelte",
        _ => "unknown",
    }
    .to_string()
}

struct LineCounts {
    lines: usize,
    code: usize,
    blank: usize,
    comment: usize,
}

fn count_lines(path: &Path, language: &str) -> LineCounts {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            return LineCounts {
                lines: 0,
                code: 0,
                blank: 0,
                comment: 0,
            };
        }
    };

    let comment_prefix = match language {
        "Rust" | "TypeScript" | "JavaScript" | "Go" | "Java" | "Kotlin" | "Swift" | "C" | "C++"
        | "C/C++ Header" | "C#" | "Dart" | "Zig" => "//",
        "Python" | "Ruby" | "Shell" | "Elixir" | "R" | "YAML" | "TOML" => "#",
        "Lua" | "SQL" => "--",
        "Erlang" => "%",
        "HTML" | "CSS" | "SCSS" | "Markdown" | "JSON" | "Vue" | "Svelte" => "",
        _ => "",
    };

    let mut lines = 0usize;
    let mut code = 0usize;
    let mut blank = 0usize;
    let mut comment = 0usize;

    for line in content.lines() {
        lines += 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank += 1;
        } else if !comment_prefix.is_empty() && trimmed.starts_with(comment_prefix) {
            comment += 1;
        } else {
            code += 1;
        }
    }

    LineCounts {
        lines,
        code,
        blank,
        comment,
    }
}

fn is_test_file(rel_path: &str, project_type: &str) -> bool {
    let path = Path::new(rel_path);
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let file_stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");

    if rel_path.contains("/test/")
        || rel_path.contains("/tests/")
        || rel_path.contains("/__tests__/")
        || rel_path.starts_with("test/")
        || rel_path.starts_with("tests/")
        || rel_path.starts_with("__tests__/")
    {
        return true;
    }

    match project_type {
        "rust" => file_name.ends_with("_test.rs") || rel_path.contains("/tests/"),
        "node" => {
            file_name.contains(".test.")
                || file_name.contains(".spec.")
                || file_stem.ends_with("_test")
        }
        "go" => file_name.ends_with("_test.go"),
        "python" => file_name.starts_with("test_") || file_name.ends_with("_test.py"),
        "ruby" => file_name.ends_with("_test.rb") || file_name.ends_with("_spec.rb"),
        "java" | "gradle" | "maven" => {
            file_name.ends_with("Test.java") || file_name.ends_with("Tests.java")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_language_rust() {
        assert_eq!(detect_language(Path::new("src/main.rs")), "Rust");
    }

    #[test]
    fn detect_language_typescript() {
        assert_eq!(detect_language(Path::new("src/app.ts")), "TypeScript");
        assert_eq!(detect_language(Path::new("src/app.tsx")), "TypeScript");
    }

    #[test]
    fn detect_language_unknown() {
        assert_eq!(detect_language(Path::new("file.xyz")), "unknown");
    }

    #[test]
    fn count_lines_basic() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.rs");
        std::fs::write(
            &file,
            "// comment\nfn main() {\n    println!(\"hi\");\n}\n\n",
        )
        .unwrap();

        let counts = count_lines(&file, "Rust");
        assert_eq!(counts.lines, 5);
        assert_eq!(counts.code, 3);
        assert_eq!(counts.blank, 1);
        assert_eq!(counts.comment, 1);
    }

    #[test]
    fn count_lines_python_comments() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.py");
        std::fs::write(&file, "# comment\ndef foo():\n    pass\n").unwrap();

        let counts = count_lines(&file, "Python");
        assert_eq!(counts.code, 2);
        assert_eq!(counts.comment, 1);
    }

    #[test]
    fn count_lines_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("empty.rs");
        std::fs::write(&file, "").unwrap();

        let counts = count_lines(&file, "Rust");
        assert_eq!(counts.lines, 0);
        assert_eq!(counts.code, 0);
    }

    #[test]
    fn count_lines_nonexistent_file() {
        let counts = count_lines(Path::new("/nonexistent/file.rs"), "Rust");
        assert_eq!(counts.lines, 0);
    }

    #[test]
    fn is_test_file_rust() {
        assert!(!is_test_file("src/main.rs", "rust"));
        assert!(is_test_file("tests/integration.rs", "rust"));
        assert!(is_test_file("src/foo_test.rs", "rust"));
    }

    #[test]
    fn is_test_file_node() {
        assert!(is_test_file("src/app.test.ts", "node"));
        assert!(is_test_file("src/app.spec.js", "node"));
        assert!(is_test_file("__tests__/app.ts", "node"));
        assert!(!is_test_file("src/app.ts", "node"));
    }

    #[test]
    fn is_test_file_go() {
        assert!(is_test_file("pkg/foo_test.go", "go"));
        assert!(!is_test_file("pkg/foo.go", "go"));
    }

    #[test]
    fn is_test_file_python() {
        assert!(is_test_file("test_foo.py", "python"));
        assert!(is_test_file("foo_test.py", "python"));
        assert!(is_test_file("tests/test_bar.py", "python"));
        assert!(!is_test_file("foo.py", "python"));
    }

    #[test]
    fn is_test_file_generic_dirs() {
        assert!(is_test_file("test/something.rs", "rust"));
        assert!(is_test_file("tests/something.go", "go"));
    }

    #[test]
    fn is_ignored_dir_common() {
        assert!(is_ignored_dir("node_modules"));
        assert!(is_ignored_dir("target"));
        assert!(is_ignored_dir(".git"));
        assert!(!is_ignored_dir("src"));
    }

    #[test]
    fn collect_files_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/lib.rs"), "pub fn foo() {}").unwrap();

        let entries = collect_files(dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn collect_files_skips_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        std::fs::create_dir(dir.path().join("target")).unwrap();
        std::fs::write(dir.path().join("target/debug.rs"), "").unwrap();

        let entries = collect_files(dir.path()).unwrap();
        assert_eq!(entries.len(), 1);
    }
}
