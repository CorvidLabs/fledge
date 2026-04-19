use anyhow::Result;
use console::style;

use crate::github;

#[derive(Debug)]
pub enum PrsAction {
    List {
        state: String,
        limit: usize,
        json: bool,
    },
    View {
        number: u64,
        json: bool,
    },
}

pub fn run(action: PrsAction) -> Result<()> {
    let config = crate::config::Config::load()?;
    let token = config.github_token();
    let (owner, repo) = github::detect_repo()?;

    match action {
        PrsAction::List { state, limit, json } => {
            list(&owner, &repo, &state, limit, json, token.as_deref())
        }
        PrsAction::View { number, json } => view(&owner, &repo, number, json, token.as_deref()),
    }
}

fn list(
    owner: &str,
    repo: &str,
    state: &str,
    limit: usize,
    json: bool,
    token: Option<&str>,
) -> Result<()> {
    let per_page = limit.min(100).to_string();
    let params = vec![
        ("state", state),
        ("per_page", per_page.as_str()),
        ("sort", "updated"),
        ("direction", "desc"),
    ];

    let path = format!("/repos/{}/{}/pulls", owner, repo);
    let items = github::github_api_get(&path, token, &params)?;

    let items = items
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("unexpected API response"))?;

    if json {
        let json_out = serde_json::to_string_pretty(&items)?;
        println!("{}", json_out);
        return Ok(());
    }

    if items.is_empty() {
        println!("No {} pull requests found in {}/{}.", state, owner, repo);
        return Ok(());
    }

    println!(
        "{}\n",
        style(format!(
            "{} pull requests in {}/{}:",
            capitalize(state),
            owner,
            repo
        ))
        .bold()
    );

    for pr in items {
        print_pr_line(pr);
    }

    Ok(())
}

fn view(owner: &str, repo: &str, number: u64, json: bool, token: Option<&str>) -> Result<()> {
    let path = format!("/repos/{}/{}/pulls/{}", owner, repo, number);
    let pr = github::github_api_get(&path, token, &[])?;

    if json {
        let json_out = serde_json::to_string_pretty(&pr)?;
        println!("{}", json_out);
        return Ok(());
    }

    let title = pr["title"].as_str().unwrap_or("(no title)");
    let state = pr["state"].as_str().unwrap_or("unknown");
    let body = pr["body"].as_str().unwrap_or("(no description)");
    let author = pr["user"]["login"].as_str().unwrap_or("unknown");
    let created = pr["created_at"].as_str().unwrap_or("");
    let draft = pr["draft"].as_bool().unwrap_or(false);
    let head = pr["head"]["ref"].as_str().unwrap_or("?");
    let base = pr["base"]["ref"].as_str().unwrap_or("?");
    let additions = pr["additions"].as_u64().unwrap_or(0);
    let deletions = pr["deletions"].as_u64().unwrap_or(0);
    let changed_files = pr["changed_files"].as_u64().unwrap_or(0);
    let comments = pr["comments"].as_u64().unwrap_or(0);
    let review_comments = pr["review_comments"].as_u64().unwrap_or(0);
    let merged = pr["merged"].as_bool().unwrap_or(false);
    let labels: Vec<&str> = pr["labels"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|l| l["name"].as_str()).collect())
        .unwrap_or_default();

    let state_str = if merged {
        style("merged").magenta().bold()
    } else if draft {
        style("draft").dim().bold()
    } else {
        match state {
            "open" => style("open").green().bold(),
            "closed" => style("closed").red().bold(),
            _ => style(state).dim(),
        }
    };

    println!(
        "{} {} {}",
        style(format!("#{}", number)).cyan().bold(),
        title,
        state_str
    );
    println!(
        "  {} by {} {}",
        style("Opened").dim(),
        style(author).cyan(),
        style(github::format_relative_time(created)).dim()
    );
    println!(
        "  {} {} → {}",
        style("Branch:").dim(),
        style(head).cyan(),
        style(base).dim()
    );

    if !labels.is_empty() {
        println!("  {} {}", style("Labels:").dim(), labels.join(", "));
    }

    println!(
        "  {} {} files changed, {}, {}",
        style("Diff:").dim(),
        changed_files,
        style(format!("+{}", additions)).green(),
        style(format!("-{}", deletions)).red()
    );

    let total_comments = comments + review_comments;
    if total_comments > 0 {
        println!("  {} {}", style("Comments:").dim(), total_comments);
    }

    println!();
    println!("{}", body);

    Ok(())
}

fn print_pr_line(pr: &serde_json::Value) {
    let number = pr["number"].as_u64().unwrap_or(0);
    let title = pr["title"].as_str().unwrap_or("(no title)");
    let state = pr["state"].as_str().unwrap_or("unknown");
    let draft = pr["draft"].as_bool().unwrap_or(false);
    let updated = pr["updated_at"].as_str().unwrap_or("");
    let head = pr["head"]["ref"].as_str().unwrap_or("?");

    let state_icon = if draft {
        style("◌").dim()
    } else {
        match state {
            "open" => style("●").green(),
            "closed" => style("●").red(),
            _ => style("●").dim(),
        }
    };

    println!(
        "  {} {} {}  {} → {}  {}",
        state_icon,
        style(format!("#{:<5}", number)).cyan(),
        title,
        style(head).dim(),
        style("base").dim(),
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
        assert_eq!(capitalize("all"), "All");
    }
}
