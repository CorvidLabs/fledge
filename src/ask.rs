use anyhow::{Result, bail};
use std::process::Command;

pub struct AskOptions {
    pub question: String,
}

pub fn run(options: AskOptions) -> Result<()> {
    ensure_claude_cli()?;

    let prompt = format!(
        "You are a helpful assistant answering questions about a codebase.\n\
        The user is in a project directory and wants to understand their code.\n\
        Be concise and use markdown formatting.\n\
        \n\
        Question: {}",
        options.question
    );

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

    print!("{}", String::from_utf8_lossy(&output.stdout));

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
