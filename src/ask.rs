use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

use crate::spec;

pub struct AskOptions {
    pub question: String,
    pub json: bool,
    pub with_specs: Vec<String>,
    pub no_spec_index: bool,
}

pub fn run(options: AskOptions) -> Result<()> {
    crate::github::ensure_claude_cli()?;

    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let spec_context = build_spec_context(&root, &options.with_specs, options.no_spec_index)?;
    let prompt = build_prompt(&options.question, options.json, spec_context.as_deref());

    let sp = crate::spinner::Spinner::start("Thinking:");

    let output = Command::new("claude").args(["--print", &prompt]).output()?;

    sp.finish();
    println!();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            eprintln!("{stderr}");
        }
        bail!("claude CLI exited with an error.");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    if options.json {
        let response = serde_json::json!({
            "question": options.question,
            "answer": stdout.trim(),
        });
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        print!("{stdout}");
    }

    Ok(())
}

fn build_spec_context(
    root: &Path,
    with_specs: &[String],
    no_index: bool,
) -> Result<Option<String>> {
    let needs_index = !no_index;
    let needs_bundles = !with_specs.is_empty();

    if !needs_index && !needs_bundles {
        return Ok(None);
    }

    let mut context = String::new();

    if needs_index {
        // Ambient context: a broken .specsync/ or malformed spec shouldn't
        // break `ask`. Silently fall back to no index in that case.
        if let Ok(entries) = spec::collect_index(root) {
            if !entries.is_empty() {
                context.push_str(&spec::render_index_markdown(&entries));
                context.push('\n');
            }
        }
    }

    if needs_bundles {
        let expanded = expand_with_specs(with_specs, root)?;
        for name in &expanded {
            let bundle = spec::load_module_bundle(root, name)
                .with_context(|| format!("loading spec bundle for '{name}'"))?;
            context.push_str(&bundle);
        }
    }

    if context.is_empty() {
        Ok(None)
    } else {
        Ok(Some(context))
    }
}

fn expand_with_specs(with_specs: &[String], root: &Path) -> Result<Vec<String>> {
    let mut names: Vec<String> = Vec::new();
    let mut include_all = false;
    for raw in with_specs {
        for part in raw.split(',') {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.eq_ignore_ascii_case("all") {
                include_all = true;
                continue;
            }
            names.push(trimmed.to_string());
        }
    }
    if include_all {
        names = spec::all_module_names(root).unwrap_or_default();
    } else {
        names.sort();
        names.dedup();
    }
    Ok(names)
}

fn build_prompt(question: &str, json: bool, spec_context: Option<&str>) -> String {
    let mut prompt = String::from(
        "You are a helpful assistant answering questions about a codebase.\n\
        The user is in a project directory and wants to understand their code.\n\
        Be concise and use markdown formatting.\n",
    );
    if json {
        prompt.push_str("Return your answer as plain text (it will be wrapped in JSON).\n");
    }
    if let Some(ctx) = spec_context {
        prompt.push_str(
            "\nThe project maintains formal specs under `specs/<module>/`. \
             Treat the context below as authoritative — prefer it over guessing from file names.\n\n",
        );
        prompt.push_str(ctx);
        prompt.push('\n');
    }
    prompt.push_str(&format!("\nQuestion: {question}"));
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_contains_question() {
        let prompt = build_prompt("how does init work?", false, None);
        assert!(prompt.contains("how does init work?"));
        assert!(prompt.contains("Question:"));
    }

    #[test]
    fn build_prompt_json_flag_adds_instruction() {
        let prompt = build_prompt("test", true, None);
        assert!(prompt.contains("plain text"));
    }

    #[test]
    fn build_prompt_no_json_flag_omits_instruction() {
        let prompt = build_prompt("test", false, None);
        assert!(!prompt.contains("plain text"));
    }

    #[test]
    fn build_prompt_includes_spec_context_when_provided() {
        let ctx = "## Available specs\n- foo v1\n";
        let prompt = build_prompt("q", false, Some(ctx));
        assert!(prompt.contains("Available specs"));
        assert!(prompt.contains("foo v1"));
    }

    #[test]
    fn build_prompt_omits_spec_block_when_none() {
        let prompt = build_prompt("q", false, None);
        assert!(!prompt.contains("Available specs"));
    }

    #[test]
    fn ask_options_stores_question() {
        let opts = AskOptions {
            question: "what is this?".to_string(),
            json: false,
            with_specs: Vec::new(),
            no_spec_index: false,
        };
        assert_eq!(opts.question, "what is this?");
        assert!(!opts.json);
    }

    #[test]
    fn expand_with_specs_handles_comma_and_dedup() {
        let root = std::path::PathBuf::from(".");
        let names = vec![
            "foo,bar".to_string(),
            "bar".to_string(),
            " baz ".to_string(),
        ];
        let expanded = expand_with_specs(&names, &root).unwrap();
        assert_eq!(expanded, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn expand_with_specs_empty_input_returns_empty() {
        let root = std::path::PathBuf::from(".");
        let expanded = expand_with_specs(&[], &root).unwrap();
        assert!(expanded.is_empty());
    }

    #[test]
    fn expand_with_specs_all_returns_every_module() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let specsync = tmp.path().join(".specsync");
        fs::create_dir_all(&specsync).unwrap();
        fs::write(
            specsync.join("config.toml"),
            "specs_dir = \"specs\"\nrequired_sections = []\n",
        )
        .unwrap();
        for name in ["cat", "bat", "ant"] {
            let dir = tmp.path().join(format!("specs/{name}"));
            fs::create_dir_all(&dir).unwrap();
            let spec = format!(
                "---\nmodule: {name}\nversion: 1\nstatus: active\nfiles: []\ndb_tables: []\ndepends_on: []\n---\n\n## Purpose\n\nP.\n"
            );
            fs::write(dir.join(format!("{name}.spec.md")), spec).unwrap();
        }

        let expanded = expand_with_specs(&["all".to_string()], tmp.path()).unwrap();
        assert_eq!(expanded, vec!["ant", "bat", "cat"]);
    }

    #[test]
    fn build_spec_context_bails_on_missing_with_specs_even_in_empty_project() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        // Empty project — no .specsync, no specs
        let err = build_spec_context(tmp.path(), &["ghost".to_string()], false).unwrap_err();
        assert!(err.to_string().contains("loading spec bundle for 'ghost'"));
    }

    #[test]
    fn build_spec_context_returns_none_when_nothing_requested() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let ctx = build_spec_context(tmp.path(), &[], true).unwrap();
        assert!(ctx.is_none());
    }
}
