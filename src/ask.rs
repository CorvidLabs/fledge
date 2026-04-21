use anyhow::{bail, Result};
use std::process::Command;

pub struct AskOptions {
    pub question: String,
    pub json: bool,
}

pub fn run(options: AskOptions) -> Result<()> {
    crate::github::ensure_claude_cli()?;

    let prompt = build_prompt(&options.question, options.json);

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

fn build_prompt(question: &str, json: bool) -> String {
    let mut prompt = String::from(
        "You are a helpful assistant answering questions about a codebase.\n\
        The user is in a project directory and wants to understand their code.\n\
        Be concise and use markdown formatting.\n",
    );
    if json {
        prompt.push_str("Return your answer as plain text (it will be wrapped in JSON).\n");
    }
    prompt.push_str(&format!("\nQuestion: {question}"));
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_contains_question() {
        let prompt = build_prompt("how does init work?", false);
        assert!(prompt.contains("how does init work?"));
        assert!(prompt.contains("Question:"));
    }

    #[test]
    fn build_prompt_json_flag_adds_instruction() {
        let prompt = build_prompt("test", true);
        assert!(prompt.contains("plain text"));
    }

    #[test]
    fn build_prompt_no_json_flag_omits_instruction() {
        let prompt = build_prompt("test", false);
        assert!(!prompt.contains("plain text"));
    }

    #[test]
    fn ask_options_stores_question() {
        let opts = AskOptions {
            question: "what is this?".to_string(),
            json: false,
        };
        assert_eq!(opts.question, "what is this?");
        assert!(!opts.json);
    }
}
