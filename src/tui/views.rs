use crate::tui::state::{AppState, ProjectView, StageFetchState, Tab};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap};
use ratatui::Frame;

pub fn draw(f: &mut Frame, state: &mut AppState) {
    // Status area grows to fit the message: 1 line for plain help, up to
    // 10 lines (bordered, wrapped) for multi-line errors so the full
    // assume-role / git diagnostic is readable inline.
    let help_height = status_height(state, f.area().width);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),           // tabs
            Constraint::Min(0),              // body
            Constraint::Length(help_height), // help / status
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

fn status_height(state: &AppState, width: u16) -> u16 {
    let Some(status) = &state.status else {
        return 1;
    };
    let inner = width.saturating_sub(2).max(1) as usize; // borders
    let mut lines = 0u16;
    for line in status.lines() {
        let len = line.chars().count().max(1);
        let wrapped = len.div_ceil(inner);
        lines = lines.saturating_add(wrapped as u16);
    }
    // +2 for borders, +1 for the "(c: clear)" hint row.
    lines.saturating_add(3).clamp(3, 10)
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
    // render it inside a bordered, wrapped panel so the full text is
    // visible — assume-role errors run multiple lines.
    if let Some(status) = &state.status {
        let is_error = status_is_error(status);
        let color = if is_error { Color::Red } else { Color::Green };
        let title = if is_error { " Error " } else { " Status " };
        let mut lines: Vec<Line> = status
            .lines()
            .map(|l| Line::from(Span::styled(l.to_string(), Style::default().fg(color))))
            .collect();
        lines.push(Line::from(vec![
            Span::raw(""),
            Span::styled("c", Style::default().fg(Color::Yellow)),
            Span::raw(": clear  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(": quit"),
        ]));
        f.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(title))
                .wrap(Wrap { trim: false }),
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
            spans.push(Span::raw(": refresh  "));
            spans.push(Span::styled("PgUp/PgDn", Style::default().fg(Color::Yellow)));
            spans.push(Span::raw(": scroll  "));
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

    let Some(pv) = state.selected_project() else {
        let para = Paragraph::new(vec![Line::from("No projects registered.")])
            .block(Block::default().borders(Borders::ALL).title("Detail"));
        f.render_widget(para, chunks[1]);
        return;
    };

    // Split right pane: metadata on top, rendered changelog below.
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(meta_height(pv)), Constraint::Min(0)])
        .split(chunks[1]);

    let meta = Paragraph::new(render_project_detail(pv))
        .block(Block::default().borders(Borders::ALL).title("Detail"))
        .wrap(Wrap { trim: false });
    f.render_widget(meta, right[0]);

    let (changelog_lines, title) = render_changelog_pane(pv);
    let changelog = Paragraph::new(changelog_lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false })
        .scroll((pv.changelog_scroll, 0));
    f.render_widget(changelog, right[1]);
}

fn meta_height(pv: &ProjectView) -> u16 {
    // Conservative: enough to fit name/repo/path/pipeline/stages/region/jira
    // + a few aws account lines + the "Stage state" label + 2-line status.
    // Anything beyond that flows into the changelog pane, which scrolls.
    let mut lines: u16 = 6; // header block (name, repo, path, blank)
    if let Some(cfg) = &pv.config {
        lines += 3; // pipeline + stages + region
        if !cfg.jira.prefixes.is_empty() {
            lines += 1;
        }
        if !cfg.jira.statuses.is_empty() {
            lines += 1;
        }
        let aws = &cfg.aws;
        if aws.default.is_some() || aws.release.is_some() || aws.s3.is_some() {
            lines += 2; // blank + header
            if aws.default.is_some() {
                lines += 1;
            }
            if aws.release.is_some() {
                lines += 1;
            }
            if aws.s3.is_some() {
                lines += 1;
            }
        }
        lines += 4; // blank + "Stage state" + 2 stage lines
    } else if pv.config_error.is_some() {
        lines += 1;
    }
    lines.saturating_add(2).clamp(8, 22) // + borders
}

fn render_changelog_pane(pv: &ProjectView) -> (Vec<Line<'_>>, String) {
    match &pv.stage_state {
        StageFetchState::Idle => (
            vec![Line::from(Span::styled(
                "press `r` to fetch commits",
                Style::default().fg(Color::DarkGray),
            ))],
            "Changelog".to_string(),
        ),
        StageFetchState::Loading => (
            vec![Line::from(Span::styled(
                "fetching commits…",
                Style::default().fg(Color::Yellow),
            ))],
            "Changelog".to_string(),
        ),
        StageFetchState::Failed(err) => (
            vec![Line::from(Span::styled(
                format!("stage fetch failed: {err}"),
                Style::default().fg(Color::Red),
            ))],
            "Changelog".to_string(),
        ),
        StageFetchState::Ready(ready) => {
            let mut lines: Vec<Line> = Vec::new();
            if let Some(err) = &ready.commits_error {
                lines.push(Line::from(Span::styled(
                    format!("commit fetch error: {err}"),
                    Style::default().fg(Color::Red),
                )));
            } else if ready.commits.is_empty() {
                lines.push(Line::from(Span::styled(
                    "no commits between stages",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                for commit in &ready.commits {
                    lines.push(Line::from(vec![
                        Span::raw("- "),
                        Span::styled(
                            commit.short_sha(),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::raw(" "),
                        Span::raw(commit.first_line().to_string()),
                    ]));
                }
            }
            (
                lines,
                format!("Commits ({}) (PgUp/PgDn to scroll)", ready.commits.len()),
            )
        }
    }
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
    if !cfg.jira.statuses.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                "statuses: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(cfg.jira.statuses.join(", ")),
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
