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

use crate::config::Config;
use crate::templates::Template;

const CYAN: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const GREEN: Color = Color::Green;
const WHITE: Color = Color::White;
const YELLOW: Color = Color::Yellow;

#[derive(PartialEq)]
enum Screen {
    SelectTemplate,
    InputVariables,
    Confirm,
    Done,
}

struct VariableField {
    key: String,
    label: String,
    value: String,
    default: String,
}

pub struct TuiApp {
    config: Config,
    templates: Vec<Template>,
    screen: Screen,
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

impl TuiApp {
    pub fn new(
        config: Config,
        templates: Vec<Template>,
        output_dir: PathBuf,
        no_git: bool,
    ) -> Self {
        let mut list_state = ListState::default();
        if !templates.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            config,
            templates,
            screen: Screen::SelectTemplate,
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

pub fn run(output_dir: PathBuf, no_git: bool) -> Result<()> {
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

    let mut app = TuiApp::new(config, templates, output_dir, no_git);

    enable_raw_mode()?;
    crossterm::execute!(io::stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    crossterm::execute!(io::stdout(), LeaveAlternateScreen)?;

    result
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut TuiApp) -> Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;

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
                Screen::SelectTemplate => handle_select_template(app, key.code),
                Screen::InputVariables => handle_input_variables(app, key.code),
                Screen::Confirm => handle_confirm(app, key.code),
                Screen::Done => {
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

fn handle_select_template(app: &mut TuiApp, key: KeyCode) {
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
                app.screen = Screen::InputVariables;
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

fn handle_input_variables(app: &mut TuiApp, key: KeyCode) {
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
            app.screen = Screen::Confirm;
        }
        KeyCode::Esc => {
            app.screen = Screen::SelectTemplate;
        }
        _ => {}
    }
}

fn handle_confirm(app: &mut TuiApp, key: KeyCode) {
    match key {
        KeyCode::Enter | KeyCode::Char('y') => match app.scaffold() {
            Ok(()) => {
                app.screen = Screen::Done;
            }
            Err(e) => {
                app.error_message = Some(format!("Error: {}", e));
            }
        },
        KeyCode::Esc | KeyCode::Char('n') => {
            app.screen = Screen::InputVariables;
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        _ => {}
    }
}

fn draw(f: &mut Frame, app: &mut TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    draw_header(f, chunks[0]);

    match app.screen {
        Screen::SelectTemplate => draw_template_list(f, app, chunks[1]),
        Screen::InputVariables => draw_variable_form(f, app, chunks[1]),
        Screen::Confirm => draw_confirm(f, app, chunks[1]),
        Screen::Done => draw_done(f, app, chunks[1]),
    }

    draw_footer(f, app, chunks[2]);

    if let Some(ref msg) = app.error_message {
        draw_error_popup(f, msg);
    }
}

fn draw_header(f: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            " fledge ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled("— get your projects ready to fly", Style::default().fg(DIM)),
    ]);
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(DIM));
    let paragraph = Paragraph::new(title).block(block);
    f.render_widget(paragraph, area);
}

fn draw_footer(f: &mut Frame, app: &TuiApp, area: Rect) {
    let hints = match app.screen {
        Screen::SelectTemplate => "↑↓ navigate  ⏎ select  q quit",
        Screen::InputVariables => "↑↓/Tab navigate  type to edit  ⏎ continue  Esc back",
        Screen::Confirm => "⏎/y scaffold  Esc back  q quit",
        Screen::Done => "⏎/q exit",
    };

    let footer = Paragraph::new(Line::from(Span::styled(hints, Style::default().fg(DIM)))).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(DIM)),
    );
    f.render_widget(footer, area);
}

fn draw_template_list(f: &mut Frame, app: &mut TuiApp, area: Rect) {
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

fn draw_variable_form(f: &mut Frame, app: &TuiApp, area: Rect) {
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

fn draw_confirm(f: &mut Frame, app: &TuiApp, area: Rect) {
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

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn draw_done(f: &mut Frame, app: &TuiApp, area: Rect) {
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

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn draw_error_popup(f: &mut Frame, msg: &str) {
    let area = f.area();
    let popup_width = (msg.len() as u16 + 6).min(area.width.saturating_sub(4));
    let popup_height = 5;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Error ")
        .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let text = Paragraph::new(Line::from(Span::styled(
        msg,
        Style::default().fg(Color::Red),
    )))
    .block(block)
    .wrap(Wrap { trim: true });

    f.render_widget(text, popup_area);
}
