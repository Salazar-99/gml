use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Creates and configures a new progress spinner with consistent styling
/// 
/// Returns a ProgressBar configured with:
/// - Custom spinner characters
/// - Green spinner color
/// - Message template
/// - 100ms tick interval
pub fn create_spinner() -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner
}
