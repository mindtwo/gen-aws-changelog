use crate::tui::state::{AppState, ProjectView, StageFetchState, Tab};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap};
use ratatui::Frame;

pub fn draw(f: &mut Frame, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Min(0),    // body
            Constraint::Length(1), // help
        ])
        .split(f.area());

    draw_tabs(f, chunks[0], state);
    match state.tab {
        Tab::Projects => draw_projects(f, chunks[1], state),
        Tab::Recipes => draw_recipes(f, chunks[1], state),
        Tab::Accounts => draw_accounts(f, chunks[1], state),
    }
    draw_help(f, chunks[2], state);
}

fn draw_tabs(f: &mut Frame, area: Rect, state: &AppState) {
    let titles = Tab::titles().map(Line::from).to_vec();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("aws-utils"))
        .select(state.tab.index())
        .style(Style::default())
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, area);
}

fn draw_help(f: &mut Frame, area: Rect, state: &AppState) {
    // If there's a status message (e.g. "assumed into prod-app-teach"),
    // show that instead of the static help. It's the most actionable
    // piece of info right after an action.
    if let Some(status) = &state.status {
        let is_error = status_is_error(status);
        let color = if is_error { Color::Red } else { Color::Green };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(status.clone(), Style::default().fg(color)),
                Span::raw("  "),
                Span::styled("(c", Style::default().fg(Color::Yellow)),
                Span::raw(": clear)"),
            ])),
            area,
        );
        return;
    }
    let mut spans = vec![
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(": switch  "),
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(": nav  "),
    ];
    match state.tab {
        Tab::Projects => {
            spans.push(Span::styled("r", Style::default().fg(Color::Yellow)));
            spans.push(Span::raw(": refresh AWS  "));
        }
        Tab::Recipes => {
            spans.push(Span::styled("n", Style::default().fg(Color::Yellow)));
            spans.push(Span::raw(": new recipe  "));
        }
        Tab::Accounts => {
            spans.push(Span::styled("l/Enter", Style::default().fg(Color::Yellow)));
            spans.push(Span::raw(": assume role  "));
        }
    }
    spans.push(Span::styled("q", Style::default().fg(Color::Yellow)));
    spans.push(Span::raw(": quit"));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_projects(f: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    let items: Vec<ListItem> = state
        .projects
        .iter()
        .map(|p| ListItem::new(p.entry.name.clone()))
        .collect();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Projects ({})", state.projects.len())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");
    f.render_stateful_widget(list, chunks[0], &mut state.projects_list);

    let detail = match state.selected_project() {
        Some(pv) => render_project_detail(pv),
        None => vec![Line::from("No projects registered.")],
    };
    let para = Paragraph::new(detail)
        .block(Block::default().borders(Borders::ALL).title("Detail"))
        .wrap(Wrap { trim: false });
    f.render_widget(para, chunks[1]);
}

fn render_project_detail(pv: &ProjectView) -> Vec<Line<'_>> {
    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("name:  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(pv.entry.name.clone()),
        ]),
        Line::from(vec![
            Span::styled("repo:  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(pv.entry.repo.clone()),
        ]),
        Line::from(vec![
            Span::styled("path:  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(pv.entry.path.display().to_string()),
        ]),
        Line::from(""),
    ];

    if let Some(err) = &pv.config_error {
        lines.push(Line::from(Span::styled(
            format!("config error: {err}"),
            Style::default().fg(Color::Red),
        )));
        return lines;
    }
    let Some(cfg) = &pv.config else {
        return lines;
    };

    lines.push(Line::from(vec![
        Span::styled("pipeline: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(cfg.pipeline.clone()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("stages:   ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!("{} → {}", cfg.from_stage, cfg.to_stage)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("region:   ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(cfg.region.clone().unwrap_or_default()),
    ]));
    if !cfg.jira.prefixes.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("jira:     ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(cfg.jira.prefixes.join(", ")),
        ]));
    }
    let aws = &cfg.aws;
    if aws.default.is_some() || aws.release.is_some() || aws.s3.is_some() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "aws accounts:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        if let Some(v) = &aws.default {
            lines.push(Line::from(format!("  default = {v}")));
        }
        if let Some(v) = &aws.release {
            lines.push(Line::from(format!("  release = {v}")));
        }
        if let Some(v) = &aws.s3 {
            lines.push(Line::from(format!("  s3      = {v}")));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Stage state",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    match &pv.stage_state {
        StageFetchState::Idle => {
            lines.push(Line::from(Span::styled(
                "press `r` to fetch from AWS",
                Style::default().fg(Color::DarkGray),
            )));
        }
        StageFetchState::Loading => {
            lines.push(Line::from(Span::styled(
                "loading…",
                Style::default().fg(Color::Yellow),
            )));
        }
        StageFetchState::Ready(ready) => {
            let from = &ready.from;
            let to = &ready.to;
            lines.push(Line::from(format!(
                "  {} = {} {}",
                from.stage,
                short(&from.revision_id),
                from.revision_summary.clone().unwrap_or_default()
            )));
            lines.push(Line::from(format!(
                "  {} = {} {}",
                to.stage,
                short(&to.revision_id),
                to.revision_summary.clone().unwrap_or_default()
            )));
            if from.revision_id == to.revision_id {
                lines.push(Line::from(Span::styled(
                    "  → stages at same revision",
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
        StageFetchState::Failed(err) => {
            lines.push(Line::from(Span::styled(
                format!("error: {err}"),
                Style::default().fg(Color::Red),
            )));
        }
    }
    lines
}

fn draw_recipes(f: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    let items: Vec<ListItem> = state
        .recipes
        .iter()
        .map(|r| ListItem::new(r.name.clone()))
        .collect();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Recipes ({})", state.recipes.len())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");
    f.render_stateful_widget(list, chunks[0], &mut state.recipes_list);

    let detail: Vec<Line> = match state.selected_recipe() {
        Some(r) => {
            let mut out = vec![
                Line::from(vec![
                    Span::styled(
                        "name:        ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(r.name.clone()),
                ]),
                Line::from(vec![
                    Span::styled(
                        "description: ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(r.description.clone()),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Steps",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
            ];
            for (i, s) in r.steps.iter().enumerate() {
                out.push(Line::from(format!("  {}. {}", i + 1, s.project)));
            }
            out
        }
        None => vec![Line::from(
            "No recipes. Create one with `aws-utils recipe create <name>`.",
        )],
    };
    let para = Paragraph::new(detail)
        .block(Block::default().borders(Borders::ALL).title("Detail"))
        .wrap(Wrap { trim: false });
    f.render_widget(para, chunks[1]);
}

fn draw_accounts(f: &mut Frame, area: Rect, state: &mut AppState) {
    // Banner showing the currently-assumed account at the top, then the
    // selectable list below.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let banner_text = match state.current_account() {
        Some(name) => Line::from(vec![
            Span::raw("current: "),
            Span::styled(
                name,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        None => Line::from(Span::styled(
            "no active session (press l/Enter to assume a role)",
            Style::default().fg(Color::DarkGray),
        )),
    };
    let banner = Paragraph::new(banner_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Active session"),
    );
    f.render_widget(banner, chunks[0]);

    let current = state.current_account();
    let items: Vec<ListItem> = state
        .accounts
        .iter()
        .map(|a| {
            let mut spans = vec![Span::raw(a.name.clone())];
            if current.as_deref() == Some(a.name.as_str()) {
                spans.push(Span::styled(
                    "  (active)",
                    Style::default().fg(Color::Green),
                ));
            }
            if !a.description.is_empty() {
                spans.push(Span::styled(
                    format!("  — {}", a.description),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Accounts ({})", state.accounts.len())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");
    f.render_stateful_widget(list, chunks[1], &mut state.accounts_list);
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}

fn status_is_error(status: &str) -> bool {
    let s = status.to_ascii_lowercase();
    s.contains("fail") || s.contains("error") || s.contains("exit")
}
