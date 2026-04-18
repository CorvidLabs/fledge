use anyhow::Result;
use dialoguer::{Input, Select, theme::ColorfulTheme};

use crate::config::Config;
use crate::templates::Template;

pub fn select_template(templates: &[Template]) -> Result<usize> {
    let items: Vec<String> = templates
        .iter()
        .map(|t| format!("{:<14} {}", t.name, t.description))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a template")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(selection)
}

pub fn prompt_variables(
    template: &Template,
    project_name: &str,
    config: &Config,
) -> Result<tera::Context> {
    let mut ctx = tera::Context::new();

    // Core variables
    ctx.insert("project_name", project_name);
    ctx.insert("project_name_snake", &to_snake_case(project_name));
    ctx.insert("project_name_pascal", &to_pascal_case(project_name));

    // Date variables
    let now = chrono::Local::now();
    ctx.insert("year", &now.format("%Y").to_string());
    ctx.insert("date", &now.format("%Y-%m-%d").to_string());

    // Author — from config, git, or prompt
    let author = match config.author_or_git() {
        Some(a) => a,
        None => Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Author name")
            .interact_text()?,
    };
    ctx.insert("author", &author);

    // GitHub org — from config or prompt
    let github_org = match config.github_org() {
        Some(org) => org,
        None => Input::with_theme(&ColorfulTheme::default())
            .with_prompt("GitHub organization")
            .default("CorvidLabs".to_string())
            .interact_text()?,
    };
    ctx.insert("github_org", &github_org);

    // License
    ctx.insert("license", &config.license());

    // Template-specific prompts
    for (key, prompt_def) in &template.manifest.prompts {
        let theme = ColorfulTheme::default();
        let value: String = if let Some(ref default) = prompt_def.default {
            let rendered = render_default(default, &ctx).unwrap_or_else(|_| default.clone());
            Input::with_theme(&theme)
                .with_prompt(&prompt_def.message)
                .default(rendered)
                .interact_text()?
        } else {
            Input::with_theme(&theme)
                .with_prompt(&prompt_def.message)
                .interact_text()?
        };
        ctx.insert(key, &value);
    }

    Ok(ctx)
}

fn render_default(template: &str, ctx: &tera::Context) -> Result<String> {
    if !template.contains("{{") {
        return Ok(template.to_string());
    }
    let mut tera = tera::Tera::default();
    tera.add_raw_template("__default__", template)?;
    Ok(tera.render("__default__", ctx)?)
}

fn to_snake_case(s: &str) -> String {
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

fn to_pascal_case(s: &str) -> String {
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
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my-project"), "MyProject");
        assert_eq!(to_pascal_case("my_project"), "MyProject");
        assert_eq!(to_pascal_case("single"), "Single");
    }
}
