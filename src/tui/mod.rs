//! Read-only-ish TUI browser. Three tabs:
//!
//! - **Projects**: list registered projects, show resolved config. Press `r`
//!   to refresh CodePipeline stage state for the selected project (async).
//! - **Recipes**: browse recipes; press `n` to create a new one.
//! - **Accounts**: list pre-configured accounts. Press `l` (or Enter) to
//!   assume-role into the selected account — the TUI suspends raw mode,
//!   prompts for MFA, then resumes. The current account is shown at the top
//!   of the tab.
//!
//! Actions that interact with the user (recipe create, account login) follow
//! the same pattern: suspend the terminal, drive dialoguer/Input outside
//! raw mode, restore the terminal and refresh state.

mod state;
mod views;

use crate::aws::assume;
use crate::commands::recipe as recipe_cmd;
use crate::config::{GlobalConfig, ProjectRegistry};
use crate::error::Result;
use crate::recipe::Recipe;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
};
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
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Suspends the TUI for an interactive prompt, runs `body`, then resumes.
/// Whatever `body` returns is passed back to the caller along with a fresh
/// terminal handle. When `body` errors we pause for a keypress before
/// resuming so the user can actually read anything the failing subprocess
/// printed to stderr.
fn with_suspend<T>(terminal: &mut Term, body: impl FnOnce() -> Result<T>) -> Result<T> {
    restore_terminal(terminal)?;
    let result = body();
    if result.is_err() {
        eprintln!("\nPress Enter to return to the TUI...");
        let mut buf = String::new();
        let _ = std::io::stdin().read_line(&mut buf);
    }
    *terminal = setup_terminal()?;
    terminal.clear()?;
    result
}

#[derive(Debug)]
enum Action {
    None,
    StageFetch,
    AssumeSelected,
    NewRecipe,
    ClearStatus,
    ScrollChangelog(i32),
}

async fn event_loop(
    terminal: &mut Term,
    state: &mut AppState,
    tx: mpsc::UnboundedSender<state::StageFetchResult>,
    rx: &mut mpsc::UnboundedReceiver<state::StageFetchResult>,
) -> Result<()> {
    loop {
        while let Ok(result) = rx.try_recv() {
            state.apply_stage_fetch(result);
        }

        terminal.draw(|f| views::draw(f, state))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if is_quit(&key) {
            return Ok(());
        }
        match handle_key(state, key.code) {
            Action::None => {}
            Action::ClearStatus => state.status = None,
            Action::ScrollChangelog(delta) => state.scroll_changelog(delta),
            Action::StageFetch => state.start_stage_fetch(tx.clone()),
            Action::AssumeSelected => {
                if let Some(account) = state.selected_account_name().map(str::to_owned) {
                    let outcome = with_suspend(terminal, || perform_assume(&account));
                    match outcome {
                        Ok(()) => {
                            state.status = Some(format!("assumed into {account}"));
                        }
                        Err(e) => {
                            state.status = Some(format!("assume failed: {e}"));
                        }
                    }
                }
            }
            Action::NewRecipe => {
                let outcome = with_suspend(terminal, || {
                    recipe_cmd::create_interactive(None).map(|p| p.display().to_string())
                });
                match outcome {
                    Ok(path) => {
                        state.recipes = Recipe::list()?;
                        state.status = Some(format!("saved recipe → {path}"));
                    }
                    Err(e) => {
                        state.status = Some(format!("recipe create failed: {e}"));
                    }
                }
            }
        }
    }
}

fn is_quit(key: &event::KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        || (key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')))
}

fn handle_key(state: &mut AppState, code: KeyCode) -> Action {
    match code {
        KeyCode::Tab => {
            state.next_tab();
            Action::None
        }
        KeyCode::BackTab => {
            state.prev_tab();
            Action::None
        }
        KeyCode::Char('1') => {
            state.tab = Tab::Projects;
            Action::None
        }
        KeyCode::Char('2') => {
            state.tab = Tab::Recipes;
            Action::None
        }
        KeyCode::Char('3') => {
            state.tab = Tab::Accounts;
            Action::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.move_down();
            Action::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.move_up();
            Action::None
        }
        KeyCode::Char('c') if state.status.is_some() => Action::ClearStatus,
        KeyCode::PageDown if matches!(state.tab, Tab::Projects) => Action::ScrollChangelog(5),
        KeyCode::PageUp if matches!(state.tab, Tab::Projects) => Action::ScrollChangelog(-5),
        KeyCode::Char('r') if matches!(state.tab, Tab::Projects) => Action::StageFetch,
        KeyCode::Char('n') if matches!(state.tab, Tab::Recipes) => Action::NewRecipe,
        KeyCode::Char('l') if matches!(state.tab, Tab::Accounts) => Action::AssumeSelected,
        KeyCode::Enter if matches!(state.tab, Tab::Accounts) => Action::AssumeSelected,
        _ => Action::None,
    }
}

fn perform_assume(account: &str) -> Result<()> {
    let mfa = assume::prompt_mfa(account)?;
    let vars = assume::run(account, Some(&mfa))?;
    assume::apply_to_env(&vars);
    // Persist for the shell wrapper so the calling shell can pick up
    // the session after the TUI exits.
    let _ = assume::write_session_file(&vars);
    Ok(())
}
