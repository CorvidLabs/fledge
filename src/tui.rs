use anyhow::{Context, Result, bail};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use std::io;
use std::path::PathBuf;
use std::process::Command;

use crate::config::Config;
use crate::templates::Template;

// Ensures raw mode + alternate screen are cleaned up on drop, even on panic or early error return.
struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen);
    }
}

const CYAN: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const GREEN: Color = Color::Green;
const WHITE: Color = Color::White;
const YELLOW: Color = Color::Yellow;
const RED: Color = Color::Red;

// ─── Action Definitions ─────────────────────────────────────────────────────

#[derive(Clone)]
struct ActionDef {
    name: &'static str,
    description: &'static str,
    kind: ActionKind,
}

#[derive(Clone)]
enum ActionKind {
    Direct(Vec<&'static str>),
    WithInput {
        fields: Vec<FieldDef>,
        action_id: ActionId,
    },
    TemplateBrowser,
}

#[derive(Clone, Copy)]
enum ActionId {
    WorkStart,
    WorkPr,
    SpecNew,
    RunTask,
    RunFlow,
    SearchTemplates,
    CreateTemplate,
    PublishTemplate,
    ConfigGet,
    ConfigSet,
    AskQuestion,
    IssueView,
    PrView,
    PluginInstall,
    PluginRemove,
    PluginSearch,
    PluginRun,
}

#[derive(Clone)]
struct FieldDef {
    label: &'static str,
    default: &'static str,
    required: bool,
}

struct CategoryDef {
    name: &'static str,
    icon: &'static str,
    description: &'static str,
    actions: Vec<ActionDef>,
}

fn build_categories() -> Vec<CategoryDef> {
    vec![
        CategoryDef {
            name: "Work",
            icon: "⎇",
            description: "Branch workflow",
            actions: vec![
                ActionDef {
                    name: "Start Branch",
                    description: "Create a new work branch",
                    kind: ActionKind::WithInput {
                        fields: vec![
                            FieldDef {
                                label: "Branch name",
                                default: "",
                                required: true,
                            },
                            FieldDef {
                                label: "Type (feat/fix/chore/docs/hotfix/refactor)",
                                default: "feat",
                                required: false,
                            },
                            FieldDef {
                                label: "Issue number",
                                default: "",
                                required: false,
                            },
                            FieldDef {
                                label: "Base branch",
                                default: "main",
                                required: false,
                            },
                            FieldDef {
                                label: "Prefix (overrides format, e.g. user/leif)",
                                default: "",
                                required: false,
                            },
                        ],
                        action_id: ActionId::WorkStart,
                    },
                },
                ActionDef {
                    name: "Create PR",
                    description: "Open a pull request from current branch",
                    kind: ActionKind::WithInput {
                        fields: vec![
                            FieldDef {
                                label: "Title (auto if empty)",
                                default: "",
                                required: false,
                            },
                            FieldDef {
                                label: "Body",
                                default: "",
                                required: false,
                            },
                        ],
                        action_id: ActionId::WorkPr,
                    },
                },
                ActionDef {
                    name: "Status",
                    description: "Show current branch and PR status",
                    kind: ActionKind::Direct(vec!["work", "status"]),
                },
            ],
        },
        CategoryDef {
            name: "GitHub",
            icon: "⊙",
            description: "Issues, PRs, CI checks",
            actions: vec![
                ActionDef {
                    name: "List Issues",
                    description: "Show open GitHub issues",
                    kind: ActionKind::Direct(vec!["issues"]),
                },
                ActionDef {
                    name: "View Issue",
                    description: "View a specific issue by number",
                    kind: ActionKind::WithInput {
                        fields: vec![FieldDef {
                            label: "Issue number",
                            default: "",
                            required: true,
                        }],
                        action_id: ActionId::IssueView,
                    },
                },
                ActionDef {
                    name: "List PRs",
                    description: "Show open pull requests",
                    kind: ActionKind::Direct(vec!["prs"]),
                },
                ActionDef {
                    name: "View PR",
                    description: "View a specific PR by number",
                    kind: ActionKind::WithInput {
                        fields: vec![FieldDef {
                            label: "PR number",
                            default: "",
                            required: true,
                        }],
                        action_id: ActionId::PrView,
                    },
                },
                ActionDef {
                    name: "CI Checks",
                    description: "View CI/CD status for current branch",
                    kind: ActionKind::Direct(vec!["checks"]),
                },
            ],
        },
        CategoryDef {
            name: "Run",
            icon: "▶",
            description: "Tasks and flows",
            actions: vec![
                ActionDef {
                    name: "List Tasks",
                    description: "Show available tasks from fledge.toml",
                    kind: ActionKind::Direct(vec!["run", "--list"]),
                },
                ActionDef {
                    name: "Run Task",
                    description: "Execute a named task",
                    kind: ActionKind::WithInput {
                        fields: vec![FieldDef {
                            label: "Task name",
                            default: "",
                            required: true,
                        }],
                        action_id: ActionId::RunTask,
                    },
                },
                ActionDef {
                    name: "List Flows",
                    description: "Show available workflow pipelines",
                    kind: ActionKind::Direct(vec!["flow", "--list"]),
                },
                ActionDef {
                    name: "Run Flow",
                    description: "Execute a workflow pipeline",
                    kind: ActionKind::WithInput {
                        fields: vec![FieldDef {
                            label: "Flow name",
                            default: "",
                            required: true,
                        }],
                        action_id: ActionId::RunFlow,
                    },
                },
                ActionDef {
                    name: "Init Tasks",
                    description: "Create a starter fledge.toml",
                    kind: ActionKind::Direct(vec!["run", "--init"]),
                },
            ],
        },
        CategoryDef {
            name: "Specs",
            icon: "📋",
            description: "Spec-sync management",
            actions: vec![
                ActionDef {
                    name: "Check",
                    description: "Validate specs against source code",
                    kind: ActionKind::Direct(vec!["spec", "check"]),
                },
                ActionDef {
                    name: "Init",
                    description: "Initialize spec-sync configuration",
                    kind: ActionKind::Direct(vec!["spec", "init"]),
                },
                ActionDef {
                    name: "New Module",
                    description: "Scaffold a new spec module",
                    kind: ActionKind::WithInput {
                        fields: vec![FieldDef {
                            label: "Module name",
                            default: "",
                            required: true,
                        }],
                        action_id: ActionId::SpecNew,
                    },
                },
            ],
        },
        CategoryDef {
            name: "Metrics",
            icon: "📊",
            description: "Code metrics and deps",
            actions: vec![
                ActionDef {
                    name: "Overview",
                    description: "Lines of code by language",
                    kind: ActionKind::Direct(vec!["metrics"]),
                },
                ActionDef {
                    name: "File Churn",
                    description: "Most-changed files from git history",
                    kind: ActionKind::Direct(vec!["metrics", "--churn"]),
                },
                ActionDef {
                    name: "Test Ratio",
                    description: "Test file detection and coverage ratio",
                    kind: ActionKind::Direct(vec!["metrics", "--tests"]),
                },
                ActionDef {
                    name: "Outdated Deps",
                    description: "Check for outdated dependencies",
                    kind: ActionKind::Direct(vec!["deps", "--outdated"]),
                },
                ActionDef {
                    name: "Security Audit",
                    description: "Run security audit on dependencies",
                    kind: ActionKind::Direct(vec!["deps", "--audit"]),
                },
                ActionDef {
                    name: "Licenses",
                    description: "Show dependency licenses",
                    kind: ActionKind::Direct(vec!["deps", "--licenses"]),
                },
            ],
        },
        CategoryDef {
            name: "Config",
            icon: "⚙",
            description: "Settings management",
            actions: vec![
                ActionDef {
                    name: "List All",
                    description: "Show all config values",
                    kind: ActionKind::Direct(vec!["config", "list"]),
                },
                ActionDef {
                    name: "Get Value",
                    description: "Read a config key",
                    kind: ActionKind::WithInput {
                        fields: vec![FieldDef {
                            label: "Key (e.g. defaults.author)",
                            default: "",
                            required: true,
                        }],
                        action_id: ActionId::ConfigGet,
                    },
                },
                ActionDef {
                    name: "Set Value",
                    description: "Write a config key",
                    kind: ActionKind::WithInput {
                        fields: vec![
                            FieldDef {
                                label: "Key",
                                default: "",
                                required: true,
                            },
                            FieldDef {
                                label: "Value",
                                default: "",
                                required: true,
                            },
                        ],
                        action_id: ActionId::ConfigSet,
                    },
                },
                ActionDef {
                    name: "Config Path",
                    description: "Show config file location",
                    kind: ActionKind::Direct(vec!["config", "path"]),
                },
            ],
        },
        CategoryDef {
            name: "Templates",
            icon: "📦",
            description: "Browse, search, create",
            actions: vec![
                ActionDef {
                    name: "Browse & Scaffold",
                    description: "Interactive template browser",
                    kind: ActionKind::TemplateBrowser,
                },
                ActionDef {
                    name: "List",
                    description: "Show available templates",
                    kind: ActionKind::Direct(vec!["list"]),
                },
                ActionDef {
                    name: "Search GitHub",
                    description: "Find templates on GitHub",
                    kind: ActionKind::WithInput {
                        fields: vec![FieldDef {
                            label: "Search query",
                            default: "",
                            required: false,
                        }],
                        action_id: ActionId::SearchTemplates,
                    },
                },
                ActionDef {
                    name: "Create Template",
                    description: "Scaffold a new fledge template",
                    kind: ActionKind::WithInput {
                        fields: vec![FieldDef {
                            label: "Template name",
                            default: "",
                            required: true,
                        }],
                        action_id: ActionId::CreateTemplate,
                    },
                },
                ActionDef {
                    name: "Publish",
                    description: "Publish a template to GitHub",
                    kind: ActionKind::WithInput {
                        fields: vec![
                            FieldDef {
                                label: "Template path",
                                default: ".",
                                required: false,
                            },
                            FieldDef {
                                label: "Organization (optional)",
                                default: "",
                                required: false,
                            },
                        ],
                        action_id: ActionId::PublishTemplate,
                    },
                },
                ActionDef {
                    name: "Validate",
                    description: "Validate templates in current directory",
                    kind: ActionKind::Direct(vec!["validate-template"]),
                },
                ActionDef {
                    name: "Update Project",
                    description: "Re-apply source template to project",
                    kind: ActionKind::Direct(vec!["update", "--yes"]),
                },
            ],
        },
        CategoryDef {
            name: "AI",
            icon: "✦",
            description: "Code review and Q&A",
            actions: vec![
                ActionDef {
                    name: "Code Review",
                    description: "AI-powered review of current changes",
                    kind: ActionKind::Direct(vec!["review"]),
                },
                ActionDef {
                    name: "Ask Question",
                    description: "Ask a question about your codebase",
                    kind: ActionKind::WithInput {
                        fields: vec![FieldDef {
                            label: "Question",
                            default: "",
                            required: true,
                        }],
                        action_id: ActionId::AskQuestion,
                    },
                },
            ],
        },
        CategoryDef {
            name: "Doctor",
            icon: "🩺",
            description: "Environment diagnostics",
            actions: vec![ActionDef {
                name: "Run Diagnostics",
                description: "Check project environment health",
                kind: ActionKind::Direct(vec!["doctor"]),
            }],
        },
        CategoryDef {
            name: "Changelog",
            icon: "📝",
            description: "Release history",
            actions: vec![
                ActionDef {
                    name: "Generate",
                    description: "Changelog from git tags and commits",
                    kind: ActionKind::Direct(vec!["changelog"]),
                },
                ActionDef {
                    name: "Unreleased",
                    description: "Changes since latest tag",
                    kind: ActionKind::Direct(vec!["changelog", "--unreleased"]),
                },
            ],
        },
        CategoryDef {
            name: "Plugins",
            icon: "🔌",
            description: "Community extensions",
            actions: vec![
                ActionDef {
                    name: "List Installed",
                    description: "Show installed plugins",
                    kind: ActionKind::Direct(vec!["plugin", "list"]),
                },
                ActionDef {
                    name: "Search",
                    description: "Find plugins on GitHub",
                    kind: ActionKind::WithInput {
                        fields: vec![FieldDef {
                            label: "Search query",
                            default: "",
                            required: false,
                        }],
                        action_id: ActionId::PluginSearch,
                    },
                },
                ActionDef {
                    name: "Install",
                    description: "Install a plugin from GitHub (owner/repo)",
                    kind: ActionKind::WithInput {
                        fields: vec![FieldDef {
                            label: "Source (owner/repo)",
                            default: "",
                            required: true,
                        }],
                        action_id: ActionId::PluginInstall,
                    },
                },
                ActionDef {
                    name: "Remove",
                    description: "Remove an installed plugin",
                    kind: ActionKind::WithInput {
                        fields: vec![FieldDef {
                            label: "Plugin name",
                            default: "",
                            required: true,
                        }],
                        action_id: ActionId::PluginRemove,
                    },
                },
                ActionDef {
                    name: "Run Command",
                    description: "Execute a plugin command",
                    kind: ActionKind::WithInput {
                        fields: vec![FieldDef {
                            label: "Command name",
                            default: "",
                            required: true,
                        }],
                        action_id: ActionId::PluginRun,
                    },
                },
            ],
        },
    ]
}

fn field(fields: &[String], idx: usize) -> &str {
    fields.get(idx).map(|s| s.as_str()).unwrap_or("")
}

fn build_command(action_id: ActionId, fields: &[String]) -> Vec<String> {
    match action_id {
        ActionId::WorkStart => {
            let name = field(fields, 0);
            let mut args = vec!["work".into(), "start".into(), name.to_string()];
            let typ = field(fields, 1);
            let typ = if typ.is_empty() { "feat" } else { typ };
            if typ != "feat" {
                args.extend(["--type".into(), typ.to_string()]);
            }
            let issue = field(fields, 2);
            if !issue.is_empty() {
                args.extend(["--issue".into(), issue.to_string()]);
            }
            let base = field(fields, 3);
            let base = if base.is_empty() { "main" } else { base };
            if base != "main" {
                args.extend(["--base".into(), base.to_string()]);
            }
            let prefix = field(fields, 4);
            if !prefix.is_empty() {
                args.extend(["--prefix".into(), prefix.to_string()]);
            }
            args
        }
        ActionId::WorkPr => {
            let mut args = vec!["work".into(), "pr".into()];
            let title = field(fields, 0);
            if !title.is_empty() {
                args.extend(["--title".into(), title.to_string()]);
            }
            let body = field(fields, 1);
            if !body.is_empty() {
                args.extend(["--body".into(), body.to_string()]);
            }
            args
        }
        ActionId::SpecNew => vec!["spec".into(), "new".into(), field(fields, 0).to_string()],
        ActionId::RunTask => vec!["run".into(), field(fields, 0).to_string()],
        ActionId::RunFlow => vec!["flow".into(), field(fields, 0).to_string()],
        ActionId::SearchTemplates => {
            let mut args = vec!["search".into()];
            let q = field(fields, 0);
            if !q.is_empty() {
                args.push(q.to_string());
            }
            args
        }
        ActionId::CreateTemplate => {
            vec!["create-template".into(), field(fields, 0).to_string()]
        }
        ActionId::PublishTemplate => {
            let mut args = vec!["publish".into()];
            let path = field(fields, 0);
            let path = if path.is_empty() { "." } else { path };
            if path != "." {
                args.push(path.to_string());
            }
            let org = field(fields, 1);
            if !org.is_empty() {
                args.extend(["--org".into(), org.to_string()]);
            }
            args
        }
        ActionId::ConfigGet => {
            vec!["config".into(), "get".into(), field(fields, 0).to_string()]
        }
        ActionId::ConfigSet => {
            vec![
                "config".into(),
                "set".into(),
                field(fields, 0).to_string(),
                field(fields, 1).to_string(),
            ]
        }
        ActionId::AskQuestion => {
            let mut args = vec!["ask".into()];
            args.extend(field(fields, 0).split_whitespace().map(String::from));
            args
        }
        ActionId::IssueView => {
            vec!["issues".into(), "view".into(), field(fields, 0).to_string()]
        }
        ActionId::PrView => {
            vec!["prs".into(), "view".into(), field(fields, 0).to_string()]
        }
        ActionId::PluginInstall => {
            vec![
                "plugin".into(),
                "install".into(),
                field(fields, 0).to_string(),
            ]
        }
        ActionId::PluginRemove => {
            vec![
                "plugin".into(),
                "remove".into(),
                field(fields, 0).to_string(),
            ]
        }
        ActionId::PluginSearch => {
            let mut args = vec!["plugin".into(), "search".into()];
            let q = field(fields, 0);
            if !q.is_empty() {
                args.push(q.to_string());
            }
            args
        }
        ActionId::PluginRun => {
            vec!["plugin".into(), "run".into(), field(fields, 0).to_string()]
        }
    }
}

// ─── Dashboard State ────────────────────────────────────────────────────────

enum DashFocus {
    Categories,
    Actions,
}

enum DashScreen {
    Browse,
    Input,
    Output,
}

struct InputField {
    label: String,
    default: String,
    value: String,
    required: bool,
}

struct DashboardApp {
    categories: Vec<CategoryDef>,
    focus: DashFocus,
    screen: DashScreen,
    cat_state: ListState,
    act_state: ListState,
    selected_cat: usize,
    selected_action: usize,
    input_fields: Vec<InputField>,
    active_field: usize,
    current_action_id: Option<ActionId>,
    output_lines: Vec<String>,
    output_scroll: usize,
    output_visible_height: usize,
    error_message: Option<String>,
    should_quit: bool,
}

impl DashboardApp {
    fn new() -> Self {
        let categories = build_categories();
        let mut cat_state = ListState::default();
        cat_state.select(Some(0));
        let mut act_state = ListState::default();
        act_state.select(Some(0));
        Self {
            categories,
            focus: DashFocus::Categories,
            screen: DashScreen::Browse,
            cat_state,
            act_state,
            selected_cat: 0,
            selected_action: 0,
            input_fields: Vec::new(),
            active_field: 0,
            current_action_id: None,
            output_lines: Vec::new(),
            output_scroll: 0,
            output_visible_height: 0,
            error_message: None,
            should_quit: false,
        }
    }

    fn current_actions(&self) -> &[ActionDef] {
        &self.categories[self.selected_cat].actions
    }

    fn execute_action(&mut self, action: &ActionDef) {
        match &action.kind {
            ActionKind::Direct(args) => {
                let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
                self.run_command(&args);
            }
            ActionKind::WithInput { fields, action_id } => {
                self.input_fields = fields
                    .iter()
                    .map(|f| InputField {
                        label: f.label.to_string(),
                        default: f.default.to_string(),
                        value: String::new(),
                        required: f.required,
                    })
                    .collect();
                self.active_field = 0;
                self.current_action_id = Some(*action_id);
                self.screen = DashScreen::Input;
            }
            ActionKind::TemplateBrowser => {}
        }
    }

    fn submit_input(&mut self) {
        for field in &self.input_fields {
            if field.required && field.value.is_empty() && field.default.is_empty() {
                self.error_message = Some(format!("{} is required", field.label));
                return;
            }
        }

        let field_values: Vec<String> = self
            .input_fields
            .iter()
            .map(|f| {
                if f.value.is_empty() {
                    f.default.clone()
                } else {
                    f.value.clone()
                }
            })
            .collect();

        if let Some(action_id) = self.current_action_id {
            let args = build_command(action_id, &field_values);
            self.run_command(&args);
        }
    }

    fn run_command(&mut self, args: &[String]) {
        let exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                self.output_lines = vec![format!("Error finding executable: {}", e)];
                self.output_scroll = 0;
                self.screen = DashScreen::Output;
                return;
            }
        };

        let result = Command::new(&exe).args(args).env("NO_COLOR", "1").output();

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let mut lines: Vec<String> = Vec::new();

                let cmd_display = format!("fledge {}", args.join(" "));
                lines.push(format!("$ {}", cmd_display));
                lines.push(String::new());

                if !stdout.is_empty() {
                    for line in stdout.lines() {
                        lines.push(strip_ansi(line));
                    }
                }

                if !stderr.is_empty() {
                    if !stdout.is_empty() {
                        lines.push(String::new());
                    }
                    for line in stderr.lines() {
                        lines.push(strip_ansi(line));
                    }
                }

                if stdout.is_empty() && stderr.is_empty() {
                    lines.push("(no output)".into());
                }

                if !output.status.success() {
                    lines.push(String::new());
                    lines.push(format!("Exit code: {}", output.status.code().unwrap_or(-1)));
                }

                self.output_lines = lines;
            }
            Err(e) => {
                self.output_lines = vec![format!("Failed to run command: {}", e)];
            }
        }

        self.output_scroll = 0;
        self.screen = DashScreen::Output;
    }
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&nc) = chars.peek() {
                    chars.next();
                    if nc.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

// ─── Dashboard Entry Point ──────────────────────────────────────────────────

pub fn run(output_dir: PathBuf, no_git: bool) -> Result<()> {
    let mut app = DashboardApp::new();

    enable_raw_mode()?;
    crossterm::execute!(io::stdout(), EnterAlternateScreen)?;
    let _guard = RawModeGuard;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    dashboard_loop(&mut terminal, &mut app, output_dir, no_git)
}

fn dashboard_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut DashboardApp,
    output_dir: PathBuf,
    no_git: bool,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw_dashboard(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(());
            }

            app.error_message = None;

            match app.screen {
                DashScreen::Browse => {
                    if handle_browse(app, key.code) {
                        let action = app.current_actions()[app.selected_action].clone();
                        if matches!(action.kind, ActionKind::TemplateBrowser) {
                            disable_raw_mode()?;
                            crossterm::execute!(io::stdout(), LeaveAlternateScreen)?;

                            let tui_result = run_template_browser(output_dir.clone(), no_git);

                            enable_raw_mode()?;
                            crossterm::execute!(io::stdout(), EnterAlternateScreen)?;
                            *terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

                            if let Err(e) = tui_result {
                                app.error_message = Some(format!("{}", e));
                            }
                        } else {
                            app.execute_action(&action);
                        }
                    }
                }
                DashScreen::Input => handle_input(app, key.code),
                DashScreen::Output => handle_output(app, key.code),
            }

            if app.should_quit {
                return Ok(());
            }
        }
    }
}

fn handle_browse(app: &mut DashboardApp, key: KeyCode) -> bool {
    match app.focus {
        DashFocus::Categories => match key {
            KeyCode::Up | KeyCode::Char('k') => {
                let total = app.categories.len();
                let cur = app.selected_cat;
                app.selected_cat = if cur == 0 { total - 1 } else { cur - 1 };
                app.cat_state.select(Some(app.selected_cat));
                app.selected_action = 0;
                app.act_state.select(Some(0));
                false
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let total = app.categories.len();
                let cur = app.selected_cat;
                app.selected_cat = if cur >= total - 1 { 0 } else { cur + 1 };
                app.cat_state.select(Some(app.selected_cat));
                app.selected_action = 0;
                app.act_state.select(Some(0));
                false
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                app.focus = DashFocus::Actions;
                false
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                app.should_quit = true;
                false
            }
            _ => false,
        },
        DashFocus::Actions => match key {
            KeyCode::Up | KeyCode::Char('k') => {
                let total = app.current_actions().len();
                let cur = app.selected_action;
                app.selected_action = if cur == 0 { total - 1 } else { cur - 1 };
                app.act_state.select(Some(app.selected_action));
                false
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let total = app.current_actions().len();
                let cur = app.selected_action;
                app.selected_action = if cur >= total - 1 { 0 } else { cur + 1 };
                app.act_state.select(Some(app.selected_action));
                false
            }
            KeyCode::Enter => true,
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Esc | KeyCode::BackTab => {
                app.focus = DashFocus::Categories;
                false
            }
            KeyCode::Char('q') => {
                app.should_quit = true;
                false
            }
            _ => false,
        },
    }
}

fn handle_input(app: &mut DashboardApp, key: KeyCode) {
    match key {
        KeyCode::Up => {
            if app.active_field > 0 {
                app.active_field -= 1;
            }
        }
        KeyCode::Down => {
            if app.active_field < app.input_fields.len() - 1 {
                app.active_field += 1;
            }
        }
        KeyCode::Tab => {
            app.active_field = (app.active_field + 1) % app.input_fields.len();
        }
        KeyCode::BackTab => {
            app.active_field = if app.active_field == 0 {
                app.input_fields.len() - 1
            } else {
                app.active_field - 1
            };
        }
        KeyCode::Char(c) => {
            app.input_fields[app.active_field].value.push(c);
        }
        KeyCode::Backspace => {
            app.input_fields[app.active_field].value.pop();
        }
        KeyCode::Enter => {
            app.submit_input();
        }
        KeyCode::Esc => {
            app.screen = DashScreen::Browse;
            app.focus = DashFocus::Actions;
        }
        _ => {}
    }
}

fn handle_output(app: &mut DashboardApp, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            app.output_scroll = app.output_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max_scroll = app
                .output_lines
                .len()
                .saturating_sub(app.output_visible_height);
            if app.output_scroll < max_scroll {
                app.output_scroll += 1;
            }
        }
        KeyCode::PageUp => {
            app.output_scroll = app.output_scroll.saturating_sub(20);
        }
        KeyCode::PageDown => {
            let max_scroll = app
                .output_lines
                .len()
                .saturating_sub(app.output_visible_height);
            app.output_scroll = (app.output_scroll + 20).min(max_scroll);
        }
        KeyCode::Home | KeyCode::Char('g') => {
            app.output_scroll = 0;
        }
        KeyCode::End | KeyCode::Char('G') => {
            app.output_scroll = app
                .output_lines
                .len()
                .saturating_sub(app.output_visible_height);
        }
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace => {
            app.screen = DashScreen::Browse;
            app.focus = DashFocus::Actions;
        }
        _ => {}
    }
}

// ─── Dashboard Drawing ──────────────────────────────────────────────────────

fn draw_dashboard(f: &mut Frame, app: &mut DashboardApp) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    draw_dash_header(f, outer[0]);

    match app.screen {
        DashScreen::Browse => draw_browse_panels(f, app, outer[1]),
        DashScreen::Input => draw_input_form(f, app, outer[1]),
        DashScreen::Output => draw_output_panel(f, app, outer[1]),
    }

    draw_dash_footer(f, app, outer[2]);

    if let Some(ref msg) = app.error_message {
        draw_error_popup(f, msg);
    }
}

fn draw_dash_header(f: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            " fledge ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled("— dev lifecycle dashboard", Style::default().fg(DIM)),
    ]);
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(DIM));
    f.render_widget(Paragraph::new(title).block(block), area);
}

fn draw_dash_footer(f: &mut Frame, app: &DashboardApp, area: Rect) {
    let hints = match app.screen {
        DashScreen::Browse => match app.focus {
            DashFocus::Categories => "↑↓ navigate  ⏎/→ open category  q quit",
            DashFocus::Actions => "↑↓ navigate  ⏎ run  ←/Esc back  q quit",
        },
        DashScreen::Input => "↑↓/Tab fields  type to edit  ⏎ submit  Esc back",
        DashScreen::Output => "↑↓ scroll  PgUp/PgDn  g/G top/bottom  Esc back",
    };
    let footer = Paragraph::new(Line::from(Span::styled(hints, Style::default().fg(DIM)))).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(DIM)),
    );
    f.render_widget(footer, area);
}

fn draw_browse_panels(f: &mut Frame, app: &mut DashboardApp, area: Rect) {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_category_list(f, app, panels[0]);
    draw_action_list(f, app, panels[1]);
}

fn draw_category_list(f: &mut Frame, app: &mut DashboardApp, area: Rect) {
    let is_focused = matches!(app.focus, DashFocus::Categories);
    let border_color = if is_focused { CYAN } else { DIM };

    let items: Vec<ListItem> = app
        .categories
        .iter()
        .map(|cat| {
            let line = Line::from(vec![
                Span::styled(format!(" {} ", cat.icon), Style::default().fg(DIM)),
                Span::styled(format!("{:<12}", cat.name), Style::default().fg(GREEN)),
                Span::styled(cat.description, Style::default().fg(DIM)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Categories ")
                .title_style(
                    Style::default()
                        .fg(if is_focused { CYAN } else { DIM })
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut app.cat_state);
}

fn draw_action_list(f: &mut Frame, app: &mut DashboardApp, area: Rect) {
    let is_focused = matches!(app.focus, DashFocus::Actions);
    let border_color = if is_focused { CYAN } else { DIM };
    let cat_name = app.categories[app.selected_cat].name;

    let actions = app.current_actions();
    let items: Vec<ListItem> = actions
        .iter()
        .map(|act| {
            let tag = match &act.kind {
                ActionKind::Direct(_) => Span::styled("  ", Style::default()),
                ActionKind::WithInput { .. } => Span::styled(" ✎ ", Style::default().fg(YELLOW)),
                ActionKind::TemplateBrowser => Span::styled(" ⊞ ", Style::default().fg(YELLOW)),
            };
            let line = Line::from(vec![
                tag,
                Span::styled(format!("{:<20}", act.name), Style::default().fg(GREEN)),
                Span::styled(act.description, Style::default().fg(DIM)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" {} ", cat_name))
                .title_style(
                    Style::default()
                        .fg(if is_focused { CYAN } else { DIM })
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut app.act_state);
}

fn draw_input_form(f: &mut Frame, app: &DashboardApp, area: Rect) {
    let cat_name = app.categories[app.selected_cat].name;
    let act_name = app.current_actions()[app.selected_action].name;

    let block = Block::default()
        .title(format!(" {} — {} ", cat_name, act_name))
        .title_style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CYAN));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let field_height = 2u16;
    let scroll_offset = if app.active_field as u16 * field_height > inner.height.saturating_sub(2) {
        (app.active_field as u16 * field_height).saturating_sub(inner.height.saturating_sub(2))
    } else {
        0
    };

    for (i, field) in app.input_fields.iter().enumerate() {
        let field_y = (i as u16 * field_height).saturating_sub(scroll_offset);
        if field_y + field_height > inner.height || (i as u16 * field_height) < scroll_offset {
            continue;
        }

        let actual_y = inner.y + field_y;
        if actual_y >= inner.y + inner.height {
            break;
        }

        let is_active = i == app.active_field;

        let label_style = if is_active {
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(WHITE)
        };

        let req_marker = if field.required { "*" } else { "" };

        let label_area = Rect::new(inner.x + 1, actual_y, inner.width.saturating_sub(2), 1);
        let label = Paragraph::new(Line::from(vec![
            Span::styled(&field.label, label_style),
            Span::styled(req_marker, Style::default().fg(RED)),
            Span::styled(": ", label_style),
        ]));
        f.render_widget(label, label_area);

        let prefix_len = field.label.len() + req_marker.len() + 2;
        let input_x = inner.x + 1 + prefix_len as u16;
        let input_width = inner.width.saturating_sub(prefix_len as u16 + 2);
        let input_area = Rect::new(input_x, actual_y, input_width, 1);

        let display_val = if field.value.is_empty() && !field.default.is_empty() {
            Span::styled(&field.default, Style::default().fg(DIM))
        } else {
            Span::styled(&field.value, Style::default().fg(WHITE))
        };

        let mut spans = vec![display_val];
        if is_active {
            spans.push(Span::styled("▎", Style::default().fg(CYAN)));
        }

        f.render_widget(Paragraph::new(Line::from(spans)), input_area);
    }
}

fn draw_output_panel(f: &mut Frame, app: &mut DashboardApp, area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;
    app.output_visible_height = visible_height;

    let total = app.output_lines.len();
    let scroll_info = if total > visible_height {
        format!(
            " {}/{} ",
            app.output_scroll + 1,
            total.saturating_sub(visible_height) + 1
        )
    } else {
        String::new()
    };

    let lines: Vec<Line> = app
        .output_lines
        .iter()
        .skip(app.output_scroll)
        .take(visible_height)
        .map(|line| {
            if line.starts_with("$ ") {
                Line::from(Span::styled(
                    line.as_str(),
                    Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
                ))
            } else if line.starts_with("Exit code:") || line.starts_with("error:") {
                Line::from(Span::styled(line.as_str(), Style::default().fg(RED)))
            } else if line.starts_with("(no output)") {
                Line::from(Span::styled(line.as_str(), Style::default().fg(DIM)))
            } else {
                Line::from(Span::styled(line.as_str(), Style::default().fg(WHITE)))
            }
        })
        .collect();

    let block = Block::default()
        .title(" Output ")
        .title_style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
        .title_bottom(Line::from(Span::styled(
            scroll_info,
            Style::default().fg(DIM),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CYAN));

    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_error_popup(f: &mut Frame, msg: &str) {
    let area = f.area();
    let popup_width = (msg.chars().count() as u16 + 6).min(area.width.saturating_sub(4));
    let popup_height = 5;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Error ")
        .title_style(Style::default().fg(RED).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(RED));

    let text = Paragraph::new(Line::from(Span::styled(msg, Style::default().fg(RED))))
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(text, popup_area);
}

// ─── Template Browser (existing functionality) ──────────────────────────────

struct VariableField {
    key: String,
    label: String,
    value: String,
    default: String,
}

#[derive(PartialEq)]
enum TemplateScreen {
    SelectTemplate,
    InputVariables,
    Confirm,
    Done,
}

struct TemplateBrowserApp {
    config: Config,
    templates: Vec<Template>,
    screen: TemplateScreen,
    list_state: ListState,
    selected_template: Option<usize>,
    variables: Vec<VariableField>,
    active_field: usize,
    project_name: String,
    output_dir: PathBuf,
    no_git: bool,
    created_files: Vec<PathBuf>,
    error_message: Option<String>,
    should_quit: bool,
}

impl TemplateBrowserApp {
    fn new(config: Config, templates: Vec<Template>, output_dir: PathBuf, no_git: bool) -> Self {
        let mut list_state = ListState::default();
        if !templates.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            config,
            templates,
            screen: TemplateScreen::SelectTemplate,
            list_state,
            selected_template: None,
            variables: Vec::new(),
            active_field: 0,
            project_name: String::new(),
            output_dir,
            no_git,
            created_files: Vec::new(),
            error_message: None,
            should_quit: false,
        }
    }

    fn build_variable_fields(&mut self, template: &Template) {
        let mut fields = Vec::new();

        fields.push(VariableField {
            key: "__project_name__".into(),
            label: "Project name".into(),
            value: String::new(),
            default: String::new(),
        });

        let author_default = self.config.author_or_git().unwrap_or_default();
        fields.push(VariableField {
            key: "__author__".into(),
            label: "Author".into(),
            value: String::new(),
            default: author_default,
        });

        let org_default = self
            .config
            .github_org()
            .or_else(|| self.config.author_or_git())
            .unwrap_or_default();
        fields.push(VariableField {
            key: "__github_org__".into(),
            label: "GitHub org".into(),
            value: String::new(),
            default: org_default,
        });

        let mut prompt_keys: Vec<_> = template.manifest.prompts.keys().collect();
        prompt_keys.sort();
        for key in prompt_keys {
            let prompt_def = &template.manifest.prompts[key];
            fields.push(VariableField {
                key: key.clone(),
                label: prompt_def.message.clone(),
                value: String::new(),
                default: prompt_def.default.clone().unwrap_or_default(),
            });
        }

        self.variables = fields;
        self.active_field = 0;
    }

    fn effective_value(&self, field: &VariableField) -> String {
        if field.value.is_empty() {
            field.default.clone()
        } else {
            field.value.clone()
        }
    }

    fn build_tera_context(&self) -> tera::Context {
        let mut ctx = tera::Context::new();
        for field in &self.variables {
            let val = self.effective_value(field);
            match field.key.as_str() {
                "__project_name__" => {
                    ctx.insert("project_name", &val);
                    ctx.insert("project_name_snake", &to_snake_case(&val));
                    ctx.insert("project_name_pascal", &to_pascal_case(&val));
                }
                "__author__" => ctx.insert("author", &val),
                "__github_org__" => ctx.insert("github_org", &val),
                key => ctx.insert(key, &val),
            }
        }

        let now = chrono::Local::now();
        ctx.insert("year", &now.format("%Y").to_string());
        ctx.insert("date", &now.format("%Y-%m-%d").to_string());
        ctx.insert("license", &self.config.license());

        ctx
    }

    fn scaffold(&mut self) -> Result<()> {
        let tpl_idx = self.selected_template.unwrap();
        let template = &self.templates[tpl_idx];

        let project_name = self.project_name.clone();
        let target_dir = self.output_dir.join(&project_name);

        if target_dir.exists() {
            bail!("Directory '{}' already exists.", target_dir.display());
        }

        let ctx = self.build_tera_context();

        std::fs::create_dir_all(&target_dir)
            .with_context(|| format!("creating directory {}", target_dir.display()))?;

        self.created_files = crate::templates::render_template(template, &target_dir, &ctx)?;

        if !self.no_git {
            crate::init::init_git_for_tui(&target_dir)?;
        }

        Ok(())
    }
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

fn run_template_browser(output_dir: PathBuf, no_git: bool) -> Result<()> {
    let config = Config::load().context("loading config")?;
    let extra_paths = config.extra_template_paths();
    let token = config.github_token();
    let templates = crate::templates::discover_templates_with_repos(
        &extra_paths,
        config.template_repos(),
        token.as_deref(),
    )?;

    if templates.is_empty() {
        bail!("No templates found.");
    }

    let mut app = TemplateBrowserApp::new(config, templates, output_dir, no_git);

    enable_raw_mode()?;
    crossterm::execute!(io::stdout(), EnterAlternateScreen)?;
    let _guard = RawModeGuard;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    template_browser_loop(&mut terminal, &mut app)
}

fn template_browser_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut TemplateBrowserApp,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw_template_browser(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.should_quit = true;
            }

            if app.should_quit {
                return Ok(());
            }

            app.error_message = None;

            match app.screen {
                TemplateScreen::SelectTemplate => handle_select_template(app, key.code),
                TemplateScreen::InputVariables => handle_input_variables(app, key.code),
                TemplateScreen::Confirm => handle_confirm(app, key.code),
                TemplateScreen::Done => {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char('q') | KeyCode::Esc) {
                        return Ok(());
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_select_template(app: &mut TemplateBrowserApp, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            let i = app.list_state.selected().unwrap_or(0);
            let new = if i == 0 {
                app.templates.len() - 1
            } else {
                i - 1
            };
            app.list_state.select(Some(new));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let i = app.list_state.selected().unwrap_or(0);
            let new = if i >= app.templates.len() - 1 {
                0
            } else {
                i + 1
            };
            app.list_state.select(Some(new));
        }
        KeyCode::Enter => {
            if let Some(idx) = app.list_state.selected() {
                app.selected_template = Some(idx);
                let tpl = &app.templates[idx];
                let tpl_clone = clone_template_for_fields(tpl);
                app.build_variable_fields(&tpl_clone);
                app.screen = TemplateScreen::InputVariables;
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.should_quit = true;
        }
        _ => {}
    }
}

fn clone_template_for_fields(tpl: &Template) -> Template {
    use crate::templates::*;
    use std::collections::HashMap;

    let mut prompts = HashMap::new();
    for (k, v) in &tpl.manifest.prompts {
        prompts.insert(
            k.clone(),
            PromptDef {
                message: v.message.clone(),
                default: v.default.clone(),
            },
        );
    }

    Template {
        name: tpl.name.clone(),
        description: tpl.description.clone(),
        path: tpl.path.clone(),
        manifest: TemplateManifest {
            template: TemplateInfo {
                name: tpl.manifest.template.name.clone(),
                description: tpl.manifest.template.description.clone(),
                min_fledge_version: tpl.manifest.template.min_fledge_version.clone(),
                version: tpl.manifest.template.version.clone(),
                requires: tpl.manifest.template.requires.clone(),
            },
            prompts,
            files: FileRules {
                render: tpl.manifest.files.render.clone(),
                copy: tpl.manifest.files.copy.clone(),
                ignore: tpl.manifest.files.ignore.clone(),
            },
            hooks: tpl.manifest.hooks.clone(),
        },
    }
}

fn handle_input_variables(app: &mut TemplateBrowserApp, key: KeyCode) {
    match key {
        KeyCode::Up => {
            if app.active_field > 0 {
                app.active_field -= 1;
            }
        }
        KeyCode::Down => {
            if app.active_field < app.variables.len() - 1 {
                app.active_field += 1;
            }
        }
        KeyCode::Tab => {
            app.active_field = (app.active_field + 1) % app.variables.len();
        }
        KeyCode::BackTab => {
            app.active_field = if app.active_field == 0 {
                app.variables.len() - 1
            } else {
                app.active_field - 1
            };
        }
        KeyCode::Char(c) => {
            app.variables[app.active_field].value.push(c);
        }
        KeyCode::Backspace => {
            app.variables[app.active_field].value.pop();
        }
        KeyCode::Enter => {
            let project_name = app.effective_value(&app.variables[0]);
            if project_name.is_empty() {
                app.error_message = Some("Project name is required.".into());
                app.active_field = 0;
                return;
            }
            app.project_name = project_name;
            app.screen = TemplateScreen::Confirm;
        }
        KeyCode::Esc => {
            app.screen = TemplateScreen::SelectTemplate;
        }
        _ => {}
    }
}

fn handle_confirm(app: &mut TemplateBrowserApp, key: KeyCode) {
    match key {
        KeyCode::Enter | KeyCode::Char('y') => match app.scaffold() {
            Ok(()) => {
                app.screen = TemplateScreen::Done;
            }
            Err(e) => {
                app.error_message = Some(format!("Error: {}", e));
            }
        },
        KeyCode::Esc | KeyCode::Char('n') => {
            app.screen = TemplateScreen::InputVariables;
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        _ => {}
    }
}

// ─── Template Browser Drawing ───────────────────────────────────────────────

fn draw_template_browser(f: &mut Frame, app: &mut TemplateBrowserApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    draw_tb_header(f, chunks[0]);

    match app.screen {
        TemplateScreen::SelectTemplate => draw_template_list(f, app, chunks[1]),
        TemplateScreen::InputVariables => draw_variable_form(f, app, chunks[1]),
        TemplateScreen::Confirm => draw_tb_confirm(f, app, chunks[1]),
        TemplateScreen::Done => draw_tb_done(f, app, chunks[1]),
    }

    draw_tb_footer(f, app, chunks[2]);

    if let Some(ref msg) = app.error_message {
        draw_error_popup(f, msg);
    }
}

fn draw_tb_header(f: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            " fledge ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled("— template browser", Style::default().fg(DIM)),
    ]);
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(DIM));
    f.render_widget(Paragraph::new(title).block(block), area);
}

fn draw_tb_footer(f: &mut Frame, app: &TemplateBrowserApp, area: Rect) {
    let hints = match app.screen {
        TemplateScreen::SelectTemplate => "↑↓ navigate  ⏎ select  q quit",
        TemplateScreen::InputVariables => "↑↓/Tab navigate  type to edit  ⏎ continue  Esc back",
        TemplateScreen::Confirm => "⏎/y scaffold  Esc back  q quit",
        TemplateScreen::Done => "⏎/q exit",
    };
    let footer = Paragraph::new(Line::from(Span::styled(hints, Style::default().fg(DIM)))).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(DIM)),
    );
    f.render_widget(footer, area);
}

fn draw_template_list(f: &mut Frame, app: &mut TemplateBrowserApp, area: Rect) {
    let items: Vec<ListItem> = app
        .templates
        .iter()
        .map(|t| {
            let line = Line::from(vec![
                Span::styled(format!("{:<16}", t.name), Style::default().fg(GREEN)),
                Span::styled(&t.description, Style::default().fg(DIM)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Select a template ")
                .title_style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM)),
        )
        .highlight_style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_variable_form(f: &mut Frame, app: &TemplateBrowserApp, area: Rect) {
    let tpl_idx = app.selected_template.unwrap();
    let tpl_name = &app.templates[tpl_idx].name;

    let block = Block::default()
        .title(format!(" {} — configure ", tpl_name))
        .title_style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let field_height = 2u16;
    let scroll_offset = if app.active_field as u16 * field_height > inner.height.saturating_sub(2) {
        (app.active_field as u16 * field_height).saturating_sub(inner.height.saturating_sub(2))
    } else {
        0
    };

    for (i, field) in app.variables.iter().enumerate() {
        let field_y = (i as u16 * field_height).saturating_sub(scroll_offset);
        if field_y + field_height > inner.height || (i as u16 * field_height) < scroll_offset {
            continue;
        }

        let actual_y = inner.y + field_y;
        if actual_y >= inner.y + inner.height {
            break;
        }

        let is_active = i == app.active_field;

        let label_style = if is_active {
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(WHITE)
        };

        let label_area = Rect::new(inner.x + 1, actual_y, inner.width.saturating_sub(2), 1);
        let label = Paragraph::new(Line::from(vec![
            Span::styled(&field.label, label_style),
            Span::styled(": ", label_style),
        ]));
        f.render_widget(label, label_area);

        let input_x = inner.x + 1 + field.label.len() as u16 + 2;
        let input_width = inner.width.saturating_sub(field.label.len() as u16 + 4);
        let input_area = Rect::new(input_x, actual_y, input_width, 1);

        let display_val = if field.value.is_empty() && !field.default.is_empty() {
            Span::styled(&field.default, Style::default().fg(DIM))
        } else {
            Span::styled(&field.value, Style::default().fg(WHITE))
        };

        let mut spans = vec![display_val];
        if is_active {
            spans.push(Span::styled("▎", Style::default().fg(CYAN)));
        }

        let input = Paragraph::new(Line::from(spans));
        f.render_widget(input, input_area);
    }
}

fn draw_tb_confirm(f: &mut Frame, app: &TemplateBrowserApp, area: Rect) {
    let tpl_idx = app.selected_template.unwrap();
    let tpl = &app.templates[tpl_idx];

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Template:  ", Style::default().fg(DIM)),
            Span::styled(
                &tpl.name,
                Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Project:   ", Style::default().fg(DIM)),
            Span::styled(
                &app.project_name,
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Location:  ", Style::default().fg(DIM)),
            Span::styled(
                app.output_dir.join(&app.project_name).display().to_string(),
                Style::default().fg(WHITE),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Git init:  ", Style::default().fg(DIM)),
            Span::styled(
                if app.no_git { "no" } else { "yes" },
                Style::default().fg(WHITE),
            ),
        ]),
        Line::from(""),
    ];

    for field in &app.variables {
        if field.key.starts_with("__") {
            continue;
        }
        lines.push(Line::from(vec![
            Span::styled(format!("  {}:  ", field.label), Style::default().fg(DIM)),
            Span::styled(app.effective_value(field), Style::default().fg(WHITE)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Press ", Style::default().fg(DIM)),
        Span::styled(
            "Enter",
            Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to scaffold, ", Style::default().fg(DIM)),
        Span::styled(
            "Esc",
            Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to go back", Style::default().fg(DIM)),
    ]));

    let block = Block::default()
        .title(" Confirm ")
        .title_style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM));

    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_tb_done(f: &mut Frame, app: &TemplateBrowserApp, area: Rect) {
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  ✓ ",
                Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("Created {} ", app.project_name),
                Style::default().fg(WHITE).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("in {}", app.output_dir.join(&app.project_name).display()),
                Style::default().fg(DIM),
            ),
        ]),
        Line::from(""),
    ];

    let max_files = (area.height as usize)
        .saturating_sub(8)
        .min(app.created_files.len());
    for file in app.created_files.iter().take(max_files) {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(file.display().to_string(), Style::default().fg(DIM)),
        ]));
    }
    if app.created_files.len() > max_files {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("... and {} more", app.created_files.len() - max_files),
                Style::default().fg(DIM),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            format!("{} files created", app.created_files.len()),
            Style::default().fg(WHITE),
        ),
    ]));

    if !app.no_git {
        lines.push(Line::from(vec![Span::styled(
            "  git repo initialized with initial commit",
            Style::default().fg(DIM),
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  cd ", Style::default().fg(DIM)),
        Span::styled(&app.project_name, Style::default().fg(CYAN)),
        Span::styled(" && get started!", Style::default().fg(DIM)),
    ]));

    let block = Block::default()
        .title(" Done ")
        .title_style(Style::default().fg(GREEN).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM));

    f.render_widget(Paragraph::new(lines).block(block), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    // strip_ansi tests

    #[test]
    fn test_strip_ansi_plain() {
        assert_eq!(strip_ansi("hello"), "hello");
    }

    #[test]
    fn test_strip_ansi_color_code() {
        assert_eq!(strip_ansi("\x1b[32mgreen\x1b[0m"), "green");
    }

    #[test]
    fn test_strip_ansi_bold() {
        assert_eq!(strip_ansi("\x1b[1mbold\x1b[0m"), "bold");
    }

    #[test]
    fn test_strip_ansi_empty() {
        assert_eq!(strip_ansi(""), "");
    }

    #[test]
    fn test_strip_ansi_mixed() {
        assert_eq!(strip_ansi("ok \x1b[31merror\x1b[0m done"), "ok error done");
    }

    // build_command tests — one per ActionId variant

    #[test]
    fn test_build_command_work_start_defaults() {
        let args = build_command(
            ActionId::WorkStart,
            &[
                "my-feature".into(),
                "".into(),
                "".into(),
                "".into(),
                "".into(),
            ],
        );
        assert_eq!(args, vec!["work", "start", "my-feature"]);
    }

    #[test]
    fn test_build_command_work_start_fix_type() {
        let args = build_command(
            ActionId::WorkStart,
            &[
                "crash".into(),
                "fix".into(),
                "".into(),
                "".into(),
                "".into(),
            ],
        );
        assert!(args.contains(&"--type".to_string()));
        assert!(args.contains(&"fix".to_string()));
    }

    #[test]
    fn test_build_command_work_start_with_issue() {
        let args = build_command(
            ActionId::WorkStart,
            &[
                "bug".into(),
                "fix".into(),
                "42".into(),
                "".into(),
                "".into(),
            ],
        );
        assert!(args.contains(&"--issue".to_string()));
        assert!(args.contains(&"42".to_string()));
    }

    #[test]
    fn test_build_command_work_start_with_prefix() {
        let args = build_command(
            ActionId::WorkStart,
            &[
                "feat".into(),
                "".into(),
                "".into(),
                "".into(),
                "user/leif".into(),
            ],
        );
        assert!(args.contains(&"--prefix".to_string()));
        assert!(args.contains(&"user/leif".to_string()));
    }

    #[test]
    fn test_build_command_work_start_with_base() {
        let args = build_command(
            ActionId::WorkStart,
            &[
                "feat".into(),
                "".into(),
                "".into(),
                "develop".into(),
                "".into(),
            ],
        );
        assert!(args.contains(&"--base".to_string()));
        assert!(args.contains(&"develop".to_string()));
    }

    #[test]
    fn test_build_command_work_pr_empty() {
        let args = build_command(ActionId::WorkPr, &["".into(), "".into()]);
        assert_eq!(args, vec!["work", "pr"]);
    }

    #[test]
    fn test_build_command_work_pr_with_title() {
        let args = build_command(ActionId::WorkPr, &["Add feature".into(), "".into()]);
        assert!(args.contains(&"--title".to_string()));
    }

    #[test]
    fn test_build_command_spec_new() {
        let args = build_command(ActionId::SpecNew, &["auth".into()]);
        assert_eq!(args, vec!["spec", "new", "auth"]);
    }

    #[test]
    fn test_build_command_run_task() {
        let args = build_command(ActionId::RunTask, &["test".into()]);
        assert_eq!(args, vec!["run", "test"]);
    }

    #[test]
    fn test_build_command_run_flow() {
        let args = build_command(ActionId::RunFlow, &["ci".into()]);
        assert_eq!(args, vec!["flow", "ci"]);
    }

    #[test]
    fn test_build_command_search_templates_empty() {
        let args = build_command(ActionId::SearchTemplates, &["".into()]);
        assert_eq!(args, vec!["search"]);
    }

    #[test]
    fn test_build_command_search_templates_query() {
        let args = build_command(ActionId::SearchTemplates, &["rust".into()]);
        assert_eq!(args, vec!["search", "rust"]);
    }

    #[test]
    fn test_build_command_create_template() {
        let args = build_command(ActionId::CreateTemplate, &["my-tpl".into()]);
        assert_eq!(args, vec!["create-template", "my-tpl"]);
    }

    #[test]
    fn test_build_command_publish_template_dot() {
        let args = build_command(ActionId::PublishTemplate, &["".into(), "".into()]);
        assert_eq!(args, vec!["publish"]);
    }

    #[test]
    fn test_build_command_publish_template_path() {
        let args = build_command(
            ActionId::PublishTemplate,
            &["./tmpl".into(), "myorg".into()],
        );
        assert!(args.contains(&"./tmpl".to_string()));
        assert!(args.contains(&"--org".to_string()));
    }

    #[test]
    fn test_build_command_config_get() {
        let args = build_command(ActionId::ConfigGet, &["defaults.author".into()]);
        assert_eq!(args, vec!["config", "get", "defaults.author"]);
    }

    #[test]
    fn test_build_command_config_set() {
        let args = build_command(
            ActionId::ConfigSet,
            &["defaults.author".into(), "leif".into()],
        );
        assert_eq!(args, vec!["config", "set", "defaults.author", "leif"]);
    }

    #[test]
    fn test_build_command_ask_question() {
        let args = build_command(ActionId::AskQuestion, &["how does auth work".into()]);
        assert_eq!(args, vec!["ask", "how", "does", "auth", "work"]);
    }

    #[test]
    fn test_build_command_issue_view() {
        let args = build_command(ActionId::IssueView, &["42".into()]);
        assert_eq!(args, vec!["issues", "view", "42"]);
    }

    #[test]
    fn test_build_command_pr_view() {
        let args = build_command(ActionId::PrView, &["7".into()]);
        assert_eq!(args, vec!["prs", "view", "7"]);
    }

    #[test]
    fn test_build_command_plugin_install() {
        let args = build_command(ActionId::PluginInstall, &["owner/repo".into()]);
        assert_eq!(args, vec!["plugin", "install", "owner/repo"]);
    }

    #[test]
    fn test_build_command_plugin_remove() {
        let args = build_command(ActionId::PluginRemove, &["myplugin".into()]);
        assert_eq!(args, vec!["plugin", "remove", "myplugin"]);
    }

    #[test]
    fn test_build_command_plugin_search_empty() {
        let args = build_command(ActionId::PluginSearch, &["".into()]);
        assert_eq!(args, vec!["plugin", "search"]);
    }

    #[test]
    fn test_build_command_plugin_run() {
        let args = build_command(ActionId::PluginRun, &["deploy".into()]);
        assert_eq!(args, vec!["plugin", "run", "deploy"]);
    }

    #[test]
    fn test_build_command_bounds_empty_fields() {
        // field() helper returns "" for out-of-bounds, preventing panics
        let args = build_command(ActionId::WorkStart, &[]);
        assert_eq!(args, vec!["work", "start", ""]);

        let args = build_command(ActionId::RunTask, &[]);
        assert_eq!(args, vec!["run", ""]);
    }
}
