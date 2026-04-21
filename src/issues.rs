use anyhow::Result;
use console::style;

use crate::github;

#[derive(Debug)]
pub enum IssuesAction {
    List {
        state: String,
        limit: usize,
        json: bool,
        label: Option<String>,
    },
    View {
        number: u64,
        json: bool,
    },
}

pub fn run(action: IssuesAction) -> Result<()> {
    let config = crate::config::Config::load()?;
    let token = config.github_token();
    let (owner, repo) = github::detect_repo()?;

    match action {
        IssuesAction::List {
            state,
            limit,
            json,
            label,
        } => list(
            &owner,
            &repo,
            &state,
            limit,
            json,
            label.as_deref(),
            token.as_deref(),
        ),
        IssuesAction::View { number, json } => view(&owner, &repo, number, json, token.as_deref()),
    }
}

fn list(
    owner: &str,
    repo: &str,
    state: &str,
    limit: usize,
    json: bool,
    label: Option<&str>,
    token: Option<&str>,
) -> Result<()> {
    let per_page = limit.min(100).to_string();
    let mut params = vec![
        ("state", state),
        ("per_page", per_page.as_str()),
        ("sort", "updated"),
        ("direction", "desc"),
    ];

    if let Some(l) = label {
        params.push(("labels", l));
    }

    let sp = crate::spinner::Spinner::start("Fetching issues:");
    let path = format!("/repos/{}/{}/issues", owner, repo);
    let items = github::github_api_get(&path, token, &params);
    sp.finish();
    let items = items?;

    let items = items
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("unexpected API response"))?;

    // Filter out pull requests (GitHub API returns PRs in the issues endpoint)
    let issues: Vec<&serde_json::Value> = items
        .iter()
        .filter(|item| item.get("pull_request").is_none())
        .collect();

    if json {
        let json_out = serde_json::to_string_pretty(&issues)?;
        println!("{}", json_out);
        return Ok(());
    }

    if issues.is_empty() {
        println!("No {} issues found in {}/{}.", state, owner, repo);
        return Ok(());
    }

    println!(
        "{}\n",
        style(format!(
            "{} issues in {}/{}:",
            capitalize(state),
            owner,
            repo
        ))
        .bold()
    );

    for issue in &issues {
        print_issue_line(issue);
    }

    Ok(())
}

fn view(owner: &str, repo: &str, number: u64, json: bool, token: Option<&str>) -> Result<()> {
    let sp = crate::spinner::Spinner::start(&format!("Fetching issue #{}:", number));
    let path = format!("/repos/{}/{}/issues/{}", owner, repo, number);
    let issue = github::github_api_get(&path, token, &[]);
    sp.finish();
    let issue = issue?;

    if issue.get("pull_request").is_some() {
        anyhow::bail!(
            "#{} is a pull request, not an issue. Use `fledge prs {}`.",
            number,
            number
        );
    }

    if json {
        let json_out = serde_json::to_string_pretty(&issue)?;
        println!("{}", json_out);
        return Ok(());
    }

    let title = issue["title"].as_str().unwrap_or("(no title)");
    let state = issue["state"].as_str().unwrap_or("unknown");
    let body = issue["body"].as_str().unwrap_or("(no description)");
    let author = issue["user"]["login"].as_str().unwrap_or("unknown");
    let created = issue["created_at"].as_str().unwrap_or("");
    let labels: Vec<&str> = issue["labels"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|l| l["name"].as_str()).collect())
        .unwrap_or_default();
    let comments = issue["comments"].as_u64().unwrap_or(0);

    let state_styled = match state {
        "open" => style(state).green().bold(),
        "closed" => style(state).red().bold(),
        _ => style(state).dim(),
    };

    println!(
        "{} {} {}",
        style(format!("#{}", number)).cyan().bold(),
        title,
        state_styled
    );
    println!(
        "  {} by {} {}",
        style("Opened").dim(),
        style(author).cyan(),
        style(github::format_relative_time(created)).dim()
    );

    if !labels.is_empty() {
        println!("  {} {}", style("Labels:").dim(), labels.join(", "));
    }
    if comments > 0 {
        println!("  {} {}", style("Comments:").dim(), comments);
    }

    println!();
    println!("{}", body);

    Ok(())
}

fn print_issue_line(issue: &serde_json::Value) {
    let number = issue["number"].as_u64().unwrap_or(0);
    let title = issue["title"].as_str().unwrap_or("(no title)");
    let state = issue["state"].as_str().unwrap_or("unknown");
    let updated = issue["updated_at"].as_str().unwrap_or("");
    let labels: Vec<&str> = issue["labels"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|l| l["name"].as_str()).collect())
        .unwrap_or_default();

    let state_icon = match state {
        "open" => style("🟢").green(),
        "closed" => style("🔴").red(),
        _ => style("⚪").dim(),
    };

    let label_str = if labels.is_empty() {
        String::new()
    } else {
        format!(" {}", style(labels.join(", ")).dim())
    };

    println!(
        "  {} {} {}{}  {}",
        state_icon,
        style(format!("#{:<5}", number)).cyan(),
        title,
        label_str,
        style(github::format_relative_time(updated)).dim()
    );
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalize_word() {
        assert_eq!(capitalize("open"), "Open");
        assert_eq!(capitalize("closed"), "Closed");
        assert_eq!(capitalize(""), "");
    }
}
