use crate::cli::ReleaseArgs;
use crate::commands::release;
use crate::error::Result;
use crate::recipe::Recipe;
use crate::ui::prompts;
use colored::Colorize;

pub async fn run(recipe: &Recipe) -> Result<()> {
    println!(
        "{} {} ({} step{})",
        "running recipe".bold(),
        recipe.name,
        recipe.steps.len(),
        if recipe.steps.len() == 1 { "" } else { "s" },
    );
    if !recipe.description.is_empty() {
        println!("{}\n", recipe.description.dimmed());
    }

    for (i, step) in recipe.steps.iter().enumerate() {
        println!(
            "\n{} step {}/{} — project {}",
            "▶".cyan().bold(),
            i + 1,
            recipe.steps.len(),
            step.project.bold(),
        );
        release::run(ReleaseArgs {
            project: Some(step.project.clone()),
            no_tag: false,
            no_changelog: false,
            summary: Some(format!("Released via recipe `{}`", recipe.name)),
        })
        .await?;

        if i < recipe.steps.len() - 1 && !prompts::confirm("Continue with next step?", true)? {
            println!("{}", "recipe aborted".yellow().bold());
            return Ok(());
        }
    }

    println!("\n{} recipe `{}` complete", "✓".green().bold(), recipe.name);
    Ok(())
}
