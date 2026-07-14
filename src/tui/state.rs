use crate::aws::codepipeline::{PipelineClient, StageRevision};
use crate::aws::load_sdk_config;
use crate::config::project::AwsAction;
use crate::config::{Account, ProjectConfig, RegistryEntry};
use crate::github::{
    compare::{compare_commits, Commit},
    GithubClient,
};
use crate::recipe::Recipe;
use ratatui::widgets::ListState;
use std::path::Path;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Projects,
    Recipes,
    Accounts,
}

impl Tab {
    pub fn titles() -> [&'static str; 3] {
        ["Projects", "Recipes", "Accounts"]
    }

    pub fn index(self) -> usize {
        match self {
            Tab::Projects => 0,
            Tab::Recipes => 1,
            Tab::Accounts => 2,
        }
    }
}

#[derive(Debug, Clone)]
pub enum StageFetchState {
    Idle,
    Loading,
    Ready(Box<StageReady>),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct StageReady {
    pub from: StageRevision,
    pub to: StageRevision,
    /// Commits between the two stage revisions, newest first. Rendered as a
    /// bullet list in the changelog pane. Empty when the stages are at the
    /// same revision; a fetch failure is surfaced in `commits_error`.
    pub commits: Vec<Commit>,
    pub commits_error: Option<String>,
}

#[derive(Debug)]
pub struct StageFetchResult {
    pub project_name: String,
    pub state: StageFetchState,
}

#[derive(Debug)]
pub struct ProjectView {
    pub entry: RegistryEntry,
    pub config: Option<ProjectConfig>,
    pub config_error: Option<String>,
    pub stage_state: StageFetchState,
    /// Scroll offset (lines) into the rendered changelog detail pane. Reset
    /// to 0 every time a new fetch completes.
    pub changelog_scroll: u16,
}

impl ProjectView {
    fn new(entry: RegistryEntry) -> Self {
        let project_path = entry.project_config_path();
        let (config, config_error) = load_project_config(&project_path);
        Self {
            entry,
            config,
            config_error,
            stage_state: StageFetchState::Idle,
            changelog_scroll: 0,
        }
    }
}

fn load_project_config(path: &Path) -> (Option<ProjectConfig>, Option<String>) {
    if !path.exists() {
        return (None, Some(format!("{} not found", path.display())));
    }
    match ProjectConfig::load(path) {
        Ok(c) => (Some(c), None),
        Err(e) => (None, Some(format!("{e}"))),
    }
}

pub struct AppState {
    pub tab: Tab,
    pub projects: Vec<ProjectView>,
    pub projects_list: ListState,
    pub recipes: Vec<Recipe>,
    pub recipes_list: ListState,
    pub accounts: Vec<Account>,
    pub accounts_list: ListState,
    /// One-line status message shown in the help bar after actions
    /// (recipe create, account assume). Cleared on the next action.
    pub status: Option<String>,
}

impl AppState {
    pub fn new(registry: Vec<RegistryEntry>, recipes: Vec<Recipe>, accounts: Vec<Account>) -> Self {
        let projects: Vec<ProjectView> = registry.into_iter().map(ProjectView::new).collect();
        let mut projects_list = ListState::default();
        if !projects.is_empty() {
            projects_list.select(Some(0));
        }
        let mut recipes_list = ListState::default();
        if !recipes.is_empty() {
            recipes_list.select(Some(0));
        }
        let mut accounts_list = ListState::default();
        if !accounts.is_empty() {
            accounts_list.select(Some(0));
        }
        Self {
            tab: Tab::Projects,
            projects,
            projects_list,
            recipes,
            recipes_list,
            accounts,
            accounts_list,
            status: None,
        }
    }

    /// Currently-assumed account name, taken from `AWS_ACCOUNT_NAME` which
    /// the assume-role script exports.
    pub fn current_account(&self) -> Option<String> {
        std::env::var("AWS_ACCOUNT_NAME")
            .ok()
            .filter(|s| !s.is_empty())
    }

    pub fn selected_account_name(&self) -> Option<&str> {
        self.accounts_list
            .selected()
            .and_then(|i| self.accounts.get(i))
            .map(|a| a.name.as_str())
    }

    pub fn next_tab(&mut self) {
        self.tab = match self.tab {
            Tab::Projects => Tab::Recipes,
            Tab::Recipes => Tab::Accounts,
            Tab::Accounts => Tab::Projects,
        };
    }

    pub fn prev_tab(&mut self) {
        self.tab = match self.tab {
            Tab::Projects => Tab::Accounts,
            Tab::Recipes => Tab::Projects,
            Tab::Accounts => Tab::Recipes,
        };
    }

    pub fn move_down(&mut self) {
        let (list, len) = self.active_list();
        let Some(len) = len else { return };
        if len == 0 {
            return;
        }
        let next = list.selected().map(|i| (i + 1) % len).unwrap_or(0);
        list.select(Some(next));
    }

    pub fn move_up(&mut self) {
        let (list, len) = self.active_list();
        let Some(len) = len else { return };
        if len == 0 {
            return;
        }
        let next = list
            .selected()
            .map(|i| if i == 0 { len - 1 } else { i - 1 })
            .unwrap_or(0);
        list.select(Some(next));
    }

    fn active_list(&mut self) -> (&mut ListState, Option<usize>) {
        match self.tab {
            Tab::Projects => (&mut self.projects_list, Some(self.projects.len())),
            Tab::Recipes => (&mut self.recipes_list, Some(self.recipes.len())),
            Tab::Accounts => (&mut self.accounts_list, Some(self.accounts.len())),
        }
    }

    pub fn selected_project(&self) -> Option<&ProjectView> {
        self.projects_list
            .selected()
            .and_then(|i| self.projects.get(i))
    }

    pub fn selected_recipe(&self) -> Option<&Recipe> {
        self.recipes_list
            .selected()
            .and_then(|i| self.recipes.get(i))
    }

    /// Kick off a background fetch of CodePipeline state for the selected
    /// project, then fetch the commits between the two stage revisions.
    /// Sends a [`StageFetchResult`] when done.
    pub fn start_stage_fetch(&mut self, tx: UnboundedSender<StageFetchResult>) {
        let Some(idx) = self.projects_list.selected() else {
            return;
        };
        let (name, repo, pipeline, region, from_stage, to_stage, account) = {
            let pv = &self.projects[idx];
            let Some(cfg) = &pv.config else {
                self.projects[idx].stage_state =
                    StageFetchState::Failed("project config missing".to_string());
                return;
            };
            (
                pv.entry.name.clone(),
                pv.entry.repo.clone(),
                cfg.pipeline.clone(),
                cfg.region.clone().unwrap_or_else(|| "eu-central-1".into()),
                cfg.from_stage.clone(),
                cfg.to_stage.clone(),
                cfg.aws.account_for(AwsAction::Release).map(str::to_owned),
            )
        };

        self.projects[idx].stage_state = StageFetchState::Loading;
        self.projects[idx].changelog_scroll = 0;

        tokio::spawn(async move {
            let result = match fetch_stage_state(&pipeline, &region, &from_stage, &to_stage, &account).await {
                Ok((from, to)) => {
                    let (commits, commits_error) = fetch_commits_for_tui(&repo, &from, &to).await;
                    StageFetchState::Ready(Box::new(StageReady {
                        from,
                        to,
                        commits,
                        commits_error,
                    }))
                }
                Err(e) => StageFetchState::Failed(format!("{e}")),
            };
            let _ = tx.send(StageFetchResult {
                project_name: name,
                state: result,
            });
        });
    }

    pub fn apply_stage_fetch(&mut self, result: StageFetchResult) {
        if let Some(pv) = self
            .projects
            .iter_mut()
            .find(|p| p.entry.name == result.project_name)
        {
            pv.stage_state = result.state;
            pv.changelog_scroll = 0;
        }
    }

    pub fn scroll_changelog(&mut self, delta: i32) {
        let Some(idx) = self.projects_list.selected() else {
            return;
        };
        let pv = &mut self.projects[idx];
        let next = (pv.changelog_scroll as i32 + delta).max(0);
        pv.changelog_scroll = next as u16;
    }
}

/// Fetch the commits between the `from` and `to` stage revisions for the
/// changelog pane. Returns the commits (newest first) and, on failure, an
/// error message to surface in the pane.
async fn fetch_commits_for_tui(
    repo: &str,
    from: &StageRevision,
    to: &StageRevision,
) -> (Vec<Commit>, Option<String>) {
    let gh = match GithubClient::new(repo) {
        Ok(c) => c,
        Err(e) => return (Vec::new(), Some(format!("github: {e}"))),
    };
    match compare_commits(&gh, &to.revision_id, &from.revision_id).await {
        Ok(commits) => (commits, None),
        Err(e) => (Vec::new(), Some(format!("github compare: {e}"))),
    }
}

async fn fetch_stage_state(
    pipeline: &str,
    region: &str,
    from_stage: &str,
    to_stage: &str,
    account: &Option<String>,
) -> crate::error::Result<(StageRevision, StageRevision)> {
    // We deliberately do NOT auto-assume here — the TUI is read-only and
    // can't host an MFA prompt. If creds are missing, the SDK returns an
    // error which we surface in the Failed state.
    let _ = account; // suppress unused warning; the user can see which
                     // account is configured via the project detail panel.
    let sdk = load_sdk_config(region).await;
    let pc = PipelineClient::new(&sdk, pipeline);
    let from = pc.stage_revision(from_stage).await?;
    let to = pc.stage_revision(to_stage).await?;
    Ok((from, to))
}
