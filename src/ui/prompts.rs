use crate::error::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};

pub fn select<S: ToString>(prompt: &str, items: &[S]) -> Result<usize> {
    let idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact()?;
    Ok(idx)
}

pub fn confirm(prompt: &str, default_yes: bool) -> Result<bool> {
    let yes = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default_yes)
        .interact()?;
    Ok(yes)
}
