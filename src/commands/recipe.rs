use crate::cli::RecipeCommand;
use crate::config::ProjectRegistry;
use crate::error::Result;
use crate::recipe::{runner, Recipe, RecipeStep};
use crate::ui::prompts;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Input, MultiSelect};

pub async fn run(cmd: RecipeCommand) -> Result<()> {
    match cmd {
        RecipeCommand::Create { name } => create(name).await,
        RecipeCommand::List => list().await,
        RecipeCommand::Run { name } => run_recipe(name).await,
    }
}

async fn create(name: String) -> Result<()> {
    let path = create_interactive(Some(name))?;
    println!("{} saved recipe to {}", "✓".green().bold(), path.display());
    Ok(())
}

/// Drives the recipe creation flow with interactive prompts. If `name` is
/// `None`, prompts for it. Returns the saved file path. Callable from the
/// CLI and TUI (the TUI suspends raw mode before calling).
pub fn create_interactive(name: Option<String>) -> Result<std::path::PathBuf> {
    let projects = ProjectRegistry::list()?;
    if projects.is_empty() {
        anyhow::bail!("no projects registered — run `aws-utils add` first");
    }
    let project_names: Vec<String> = projects.iter().map(|p| p.name.clone()).collect();

    let name = match name {
        Some(n) if !n.is_empty() => n,
        _ => Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("Recipe name")
            .interact_text()?,
    };
    if name.trim().is_empty() {
        anyhow::bail!("recipe name cannot be empty");
    }
    if Recipe::file_path(&name)?.exists() {
        anyhow::bail!("recipe '{}' already exists", name);
    }

    let description: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Recipe description")
        .allow_empty(true)
        .interact_text()?;

    let selected = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick the projects to include (in release order)")
        .items(&project_names)
        .interact()?;
    if selected.is_empty() {
        anyhow::bail!("a recipe needs at least one step");
    }

    let steps = selected
        .into_iter()
        .map(|i| RecipeStep {
            project: project_names[i].clone(),
        })
        .collect();

    let recipe = Recipe {
        name,
        description,
        steps,
    };
    recipe.save()
}

async fn list() -> Result<()> {
    let recipes = Recipe::list()?;
    if recipes.is_empty() {
        println!("no recipes (create one with `aws-utils recipe create <name>`)");
        return Ok(());
    }
    for r in &recipes {
        println!(
            "{}  {} step{}{}",
            r.name.bold(),
            r.steps.len(),
            if r.steps.len() == 1 { "" } else { "s" },
            if r.description.is_empty() {
                String::new()
            } else {
                format!(" — {}", r.description)
            },
        );
        for (i, step) in r.steps.iter().enumerate() {
            println!("  {}. {}", i + 1, step.project);
        }
    }
    Ok(())
}

async fn run_recipe(name: String) -> Result<()> {
    let recipe = Recipe::load(&name)?;
    println!(
        "{} {} step{}",
        "loaded recipe".dimmed(),
        recipe.steps.len(),
        if recipe.steps.len() == 1 { "" } else { "s" },
    );
    if !prompts::confirm(&format!("Run recipe `{}` now?", name), false)? {
        println!("{}", "aborted".dimmed());
        return Ok(());
    }
    runner::run(&recipe).await
}
