//! Read-only TUI browser. Three tabs:
//!
//! - **Projects**: list registered projects, show resolved config, optional
//!   AWS stage state refresh (`r`) that fetches CodePipeline state in the
//!   background.
//! - **Recipes**: list recipes and their step order.
//! - **Accounts**: list pre-configured AWS account names.
//!
//! Actions (release, approve, push) live in the CLI — they require MFA
//! and confirmation flows that don't compose well with a TUI event loop.

mod state;
mod views;

use crate::config::{GlobalConfig, ProjectRegistry};
use crate::error::Result;
use crate::recipe::Recipe;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use state::{AppState, Tab};
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

pub async fn run() -> Result<()> {
    let projects = ProjectRegistry::list()?;
    let recipes = Recipe::list()?;
    let global = GlobalConfig::load_or_default()?;
    let mut state = AppState::new(projects, recipes, global.accounts);

    // mpsc channel for async stage-fetch results from background tasks.
    let (tx, mut rx) = mpsc::unbounded_channel::<state::StageFetchResult>();

    let mut terminal = setup_terminal()?;
    let result = event_loop(&mut terminal, &mut state, tx, &mut rx).await;
    restore_terminal(&mut terminal)?;
    result
}

type Term = Terminal<CrosstermBackend<io::Stdout>>;

fn setup_terminal() -> Result<Term> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Term) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

async fn event_loop(
    terminal: &mut Term,
    state: &mut AppState,
    tx: mpsc::UnboundedSender<state::StageFetchResult>,
    rx: &mut mpsc::UnboundedReceiver<state::StageFetchResult>,
) -> Result<()> {
    loop {
        // Drain any pending async results.
        while let Ok(result) = rx.try_recv() {
            state.apply_stage_fetch(result);
        }

        terminal.draw(|f| views::draw(f, state))?;

        // Block briefly for input so we don't peg the CPU but stay
        // responsive to channel messages too.
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if is_quit(&key) {
                        return Ok(());
                    }
                    handle_key(state, key.code, &tx);
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
}

fn is_quit(key: &event::KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        || (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c')))
}

fn handle_key(
    state: &mut AppState,
    code: KeyCode,
    tx: &mpsc::UnboundedSender<state::StageFetchResult>,
) {
    match code {
        KeyCode::Tab => state.next_tab(),
        KeyCode::BackTab => state.prev_tab(),
        KeyCode::Char('1') => state.tab = Tab::Projects,
        KeyCode::Char('2') => state.tab = Tab::Recipes,
        KeyCode::Char('3') => state.tab = Tab::Accounts,
        KeyCode::Down | KeyCode::Char('j') => state.move_down(),
        KeyCode::Up | KeyCode::Char('k') => state.move_up(),
        KeyCode::Char('r') => {
            if matches!(state.tab, Tab::Projects) {
                state.start_stage_fetch(tx.clone());
            }
        }
        _ => {}
    }
}
