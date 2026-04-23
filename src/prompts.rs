use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input, Select};

use crate::config::Config;
use crate::templates::Template;

pub fn select_template(templates: &[Template]) -> Result<usize> {
    crate::utils::require_interactive("template")?;

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
    yes: bool,
    author_override: Option<&str>,
    org_override: Option<&str>,
) -> Result<tera::Context> {
    let mut ctx = tera::Context::new();

    // Core variables
    ctx.insert("project_name", project_name);
    crate::utils::validate_project_name(project_name)?;
    ctx.insert(
        "project_name_snake",
        &crate::utils::to_snake_case(project_name),
    );
    ctx.insert(
        "project_name_kebab",
        &crate::utils::to_kebab_case(project_name),
    );
    ctx.insert(
        "project_name_pascal",
        &crate::utils::to_pascal_case(project_name),
    );
    ctx.insert(
        "project_name_camel",
        &crate::utils::to_camel_case(project_name),
    );

    // Date variables
    let now = chrono::Local::now();
    ctx.insert("year", &now.format("%Y").to_string());
    ctx.insert("date", &now.format("%Y-%m-%d").to_string());

    let author = if let Some(a) = author_override {
        a.to_string()
    } else {
        match config.author_or_git() {
            Some(a) => a,
            None if yes || !crate::utils::is_interactive() => project_name.to_string(),
            None => Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Author name")
                .interact_text()?,
        }
    };
    ctx.insert("author", &author);

    let github_org = if let Some(o) = org_override {
        o.to_string()
    } else {
        match config.github_org() {
            Some(org) => org,
            None if yes || !crate::utils::is_interactive() => author.clone(),
            None => {
                let theme = ColorfulTheme::default();
                let input = Input::<String>::with_theme(&theme).with_prompt("GitHub organization");
                let input = if let Some(ref a) = config.author_or_git() {
                    input.default(a.clone())
                } else {
                    input
                };
                input.interact_text()?
            }
        }
    };
    crate::utils::validate_github_org(&github_org)?;
    ctx.insert("github_org", &github_org);

    // License
    ctx.insert("license", &config.license());

    for (key, prompt_def) in &template.manifest.prompts {
        let value: String = if yes || !crate::utils::is_interactive() {
            if let Some(ref default) = prompt_def.default {
                render_default(default, &ctx).unwrap_or_else(|_| default.clone())
            } else {
                String::new()
            }
        } else {
            let theme = ColorfulTheme::default();
            if let Some(ref default) = prompt_def.default {
                let rendered = render_default(default, &ctx).unwrap_or_else(|_| default.clone());
                Input::with_theme(&theme)
                    .with_prompt(&prompt_def.message)
                    .default(rendered)
                    .interact_text()?
            } else {
                Input::with_theme(&theme)
                    .with_prompt(&prompt_def.message)
                    .interact_text()?
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("my-project"), "my_project");
        assert_eq!(to_snake_case("MyProject"), "myproject");
        assert_eq!(to_snake_case("already_snake"), "already_snake");
    }

    #[test]
    fn test_to_snake_case_multiple_hyphens() {
        assert_eq!(to_snake_case("my-cool-project"), "my_cool_project");
    }

    #[test]
    fn test_to_snake_case_empty() {
        assert_eq!(to_snake_case(""), "");
    }

    #[test]
    fn test_to_snake_case_single_char() {
        assert_eq!(to_snake_case("A"), "a");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my-project"), "MyProject");
        assert_eq!(to_pascal_case("my_project"), "MyProject");
        assert_eq!(to_pascal_case("single"), "Single");
    }

    #[test]
    fn test_to_pascal_case_multiple_segments() {
        assert_eq!(to_pascal_case("my-cool-project"), "MyCoolProject");
    }

    #[test]
    fn test_to_pascal_case_mixed_separators() {
        assert_eq!(to_pascal_case("my-cool_project"), "MyCoolProject");
    }

    #[test]
    fn test_to_pascal_case_empty() {
        assert_eq!(to_pascal_case(""), "");
    }

    #[test]
    fn test_to_pascal_case_single_char() {
        assert_eq!(to_pascal_case("a"), "A");
    }

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("my_project"), "my-project");
        assert_eq!(to_kebab_case("my-project"), "my-project");
        assert_eq!(to_kebab_case("MyProject"), "myproject");
    }

    #[test]
    fn test_to_kebab_case_empty() {
        assert_eq!(to_kebab_case(""), "");
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("my-project"), "myProject");
        assert_eq!(to_camel_case("my_project"), "myProject");
        assert_eq!(to_camel_case("single"), "single");
    }

    #[test]
    fn test_to_camel_case_multiple_segments() {
        assert_eq!(to_camel_case("my-cool-project"), "myCoolProject");
    }

    #[test]
    fn test_to_camel_case_empty() {
        assert_eq!(to_camel_case(""), "");
    }

    #[test]
    fn render_default_plain_string() {
        let ctx = tera::Context::new();
        assert_eq!(render_default("hello world", &ctx).unwrap(), "hello world");
    }

    #[test]
    fn render_default_with_variable() {
        let mut ctx = tera::Context::new();
        ctx.insert("project_name", "my-app");
        assert_eq!(
            render_default("A {{ project_name }} project", &ctx).unwrap(),
            "A my-app project"
        );
    }

    #[test]
    fn render_default_no_braces_passthrough() {
        let ctx = tera::Context::new();
        assert_eq!(
            render_default("no variables here", &ctx).unwrap(),
            "no variables here"
        );
    }

    #[test]
    fn render_default_missing_var_errors() {
        let ctx = tera::Context::new();
        let result = render_default("{{ missing }}", &ctx);
        assert!(result.is_err());
    }
}
