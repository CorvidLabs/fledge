use anyhow::{Result, bail};
use console::style;
use std::process::Command;

pub struct AskOptions {
    pub question: String,
}

pub fn run(options: AskOptions) -> Result<()> {
    ensure_claude_cli()?;

    println!("{} Thinking...\n", style("🔵").cyan().bold(),);

    let prompt = format!(
        "You are a helpful assistant answering questions about a codebase.\n\
        The user is in a project directory and wants to understand their code.\n\
        Be concise and use markdown formatting.\n\
        \n\
        Question: {}",
        options.question
    );

    let status = Command::new("claude").args(["--print", &prompt]).status()?;

    if !status.success() {
        bail!("claude CLI exited with an error.");
    }

    Ok(())
}

fn ensure_claude_cli() -> Result<()> {
    if Command::new("claude")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_err()
    {
        bail!(
            "Claude CLI is not installed. Install it from https://docs.anthropic.com/en/docs/claude-code and run `claude` to authenticate."
        );
    }
    Ok(())
}
