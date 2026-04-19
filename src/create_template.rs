use anyhow::{Context, Result};
use console::style;
use dialoguer::{Confirm, Input, theme::ColorfulTheme};
use std::path::{Path, PathBuf};

pub struct CreateTemplateOptions {
    pub name: String,
    pub output: PathBuf,
}

struct TemplateAnswers {
    name: String,
    description: String,
    render_globs: Vec<String>,
    include_hooks: bool,
    include_prompts: bool,
}

pub fn run(options: CreateTemplateOptions) -> Result<()> {
    let target = options.output.join(&options.name);

    if target.exists() {
        anyhow::bail!("Directory '{}' already exists", target.display());
    }

    let answers = gather_answers(&options.name)?;
    scaffold(&target, &answers)?;

    println!(
        "\n{} Created template at {}",
        style("✓").green().bold(),
        style(target.display()).cyan()
    );
    println!(
        "\n  {} Edit files in {}/",
        style("1.").dim(),
        style(&answers.name).green()
    );
    println!(
        "  {} Add .tera extension to files that need variable substitution",
        style("2.").dim()
    );
    println!(
        "  {} Test locally with: {}",
        style("3.").dim(),
        style(format!("fledge init my-project -t ./{}", answers.name)).cyan()
    );

    Ok(())
}

fn gather_answers(default_name: &str) -> Result<TemplateAnswers> {
    let theme = ColorfulTheme::default();

    let name: String = Input::with_theme(&theme)
        .with_prompt("Template name")
        .default(default_name.to_string())
        .interact_text()?;

    let description: String = Input::with_theme(&theme)
        .with_prompt("Description")
        .default(format!("A {} project template", name))
        .interact_text()?;

    let render_input: String = Input::with_theme(&theme)
        .with_prompt("File patterns to render through Tera (comma-separated)")
        .default("**/*.md, **/*.toml, **/*.json, **/*.yml".to_string())
        .interact_text()?;

    let render_globs: Vec<String> = render_input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let include_hooks = Confirm::with_theme(&theme)
        .with_prompt("Include post-create hooks?")
        .default(false)
        .interact()?;

    let include_prompts = Confirm::with_theme(&theme)
        .with_prompt("Include custom prompts?")
        .default(true)
        .interact()?;

    Ok(TemplateAnswers {
        name,
        description,
        render_globs,
        include_hooks,
        include_prompts,
    })
}

fn scaffold(target: &Path, answers: &TemplateAnswers) -> Result<()> {
    std::fs::create_dir_all(target).with_context(|| format!("creating {}", target.display()))?;

    write_manifest(target, answers)?;
    write_example_files(target)?;
    write_readme(target, &answers.name)?;

    Ok(())
}

fn write_manifest(target: &Path, answers: &TemplateAnswers) -> Result<()> {
    let mut manifest = String::new();

    manifest.push_str("[template]\n");
    manifest.push_str(&format!("name = {:?}\n", answers.name));
    manifest.push_str(&format!("description = {:?}\n", answers.description));
    manifest.push_str("# min_fledge_version = \"0.2.0\"\n");

    if answers.include_prompts {
        manifest.push_str("\n[prompts.description]\n");
        manifest.push_str("message = \"Project description\"\n");
        manifest.push_str(&format!("default = \"A new {} project\"\n", answers.name));
        manifest.push_str("\n# Add more prompts:\n");
        manifest.push_str("# [prompts.database]\n");
        manifest.push_str("# message = \"Database engine\"\n");
        manifest.push_str("# default = \"sqlite\"\n");
    }

    manifest.push_str("\n[files]\n");

    let render_arr: Vec<String> = answers
        .render_globs
        .iter()
        .map(|g| format!("{:?}", g))
        .collect();
    manifest.push_str(&format!("render = [{}]\n", render_arr.join(", ")));
    manifest.push_str("copy = [\"**/*.png\", \"**/*.ico\", \"**/*.woff2\"]\n");
    manifest.push_str("ignore = [\"template.toml\"]\n");

    if answers.include_hooks {
        manifest.push_str("\n[hooks]\n");
        manifest.push_str("post_create = [\n");
        manifest.push_str("    # \"npm install\",\n");
        manifest.push_str("    # \"git init\",\n");
        manifest.push_str("]\n");
    }

    std::fs::write(target.join("template.toml"), manifest).context("writing template.toml")?;

    Ok(())
}

fn write_example_files(target: &Path) -> Result<()> {
    std::fs::create_dir_all(target.join("src"))?;

    std::fs::write(
        target.join("README.md.tera"),
        r#"# {{ project_name }}

{{ description }}

## Getting Started

TODO: Add setup instructions here.

## License

{{ license }}
"#,
    )?;

    std::fs::write(
        target.join(".gitignore"),
        r#"# Build artifacts
/target/
/dist/
/build/
node_modules/

# IDE
.idea/
.vscode/
*.swp

# OS
.DS_Store
Thumbs.db
"#,
    )?;

    Ok(())
}

fn write_readme(target: &Path, name: &str) -> Result<()> {
    std::fs::write(
        target.join("README.md"),
        format!(
            r#"# {name} — fledge template

A project template for [fledge](https://github.com/CorvidLabs/fledge).

## Usage

Test this template locally:

```bash
fledge init my-project -t ./{name}
```

## Template structure

- `template.toml` — Template manifest (name, prompts, file rules, hooks)
- Files with `.tera` extension are rendered through Tera and the extension is stripped
- Files matching `render` globs in template.toml are also rendered through Tera
- Files matching `ignore` globs are not included in generated projects

## Template variables

These variables are available in all rendered files:

| Variable | Description |
|----------|-------------|
| `{{{{ project_name }}}}` | Project name as provided by the user |
| `{{{{ project_name_snake }}}}` | Snake_case version |
| `{{{{ project_name_pascal }}}}` | PascalCase version |
| `{{{{ author }}}}` | Author name |
| `{{{{ github_org }}}}` | GitHub organization |
| `{{{{ license }}}}` | License identifier |
| `{{{{ year }}}}` | Current year |
| `{{{{ date }}}}` | Current date (YYYY-MM-DD) |
| `{{{{ description }}}}` | Project description (if prompt defined) |

Custom prompts defined in `template.toml` also become available as variables.
"#,
        ),
    )
    .context("writing README.md")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn scaffold_creates_expected_files() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("my-template");

        let answers = TemplateAnswers {
            name: "my-template".to_string(),
            description: "A test template".to_string(),
            render_globs: vec!["**/*.md".to_string(), "**/*.toml".to_string()],
            include_hooks: false,
            include_prompts: true,
        };

        scaffold(&target, &answers).unwrap();

        assert!(target.join("template.toml").exists());
        assert!(target.join("README.md").exists());
        assert!(target.join("README.md.tera").exists());
        assert!(target.join(".gitignore").exists());
    }

    #[test]
    fn scaffold_manifest_is_valid_toml() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("test-tpl");

        let answers = TemplateAnswers {
            name: "test-tpl".to_string(),
            description: "Test".to_string(),
            render_globs: vec!["**/*.rs".to_string()],
            include_hooks: true,
            include_prompts: true,
        };

        scaffold(&target, &answers).unwrap();

        let content = std::fs::read_to_string(target.join("template.toml")).unwrap();
        let manifest: Result<crate::templates::TemplateManifest, _> = toml::from_str(&content);
        assert!(
            manifest.is_ok(),
            "Generated template.toml should be valid: {:?}",
            manifest.err()
        );
    }

    #[test]
    fn scaffold_manifest_without_hooks_or_prompts() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("bare-tpl");

        let answers = TemplateAnswers {
            name: "bare-tpl".to_string(),
            description: "Bare template".to_string(),
            render_globs: vec!["**/*.txt".to_string()],
            include_hooks: false,
            include_prompts: false,
        };

        scaffold(&target, &answers).unwrap();

        let content = std::fs::read_to_string(target.join("template.toml")).unwrap();
        assert!(!content.contains("[hooks]"));
        assert!(!content.contains("[prompts"));

        let manifest: Result<crate::templates::TemplateManifest, _> = toml::from_str(&content);
        assert!(manifest.is_ok());
    }

    #[test]
    fn scaffold_fails_if_target_exists() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("existing");
        std::fs::create_dir(&target).unwrap();

        let options = CreateTemplateOptions {
            name: "existing".to_string(),
            output: tmp.path().to_path_buf(),
        };

        let result = run(options);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn manifest_render_globs_are_correct() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("glob-tpl");

        let answers = TemplateAnswers {
            name: "glob-tpl".to_string(),
            description: "Test".to_string(),
            render_globs: vec![
                "**/*.rs".to_string(),
                "**/*.toml".to_string(),
                "**/*.md".to_string(),
            ],
            include_hooks: false,
            include_prompts: false,
        };

        scaffold(&target, &answers).unwrap();

        let content = std::fs::read_to_string(target.join("template.toml")).unwrap();
        let manifest: crate::templates::TemplateManifest = toml::from_str(&content).unwrap();
        assert_eq!(
            manifest.files.render,
            vec!["**/*.rs", "**/*.toml", "**/*.md"]
        );
    }
}
