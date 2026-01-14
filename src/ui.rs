use dialoguer::Select;

use crate::config::get_oauth_account;
use crate::profiles::get_profile_path;

/// Interactively select a profile from the list
/// Returns the selected profile name, or None if cancelled
pub fn select_profile(profiles: &[String], current_email: &str) -> Option<String> {
    if profiles.is_empty() {
        println!("No profiles found. Use 'claudectx save <name>' to create one.");
        return None;
    }

    // Build display items with profile info
    let items: Vec<String> = profiles
        .iter()
        .map(|name| {
            let path = get_profile_path(name);
            let config: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(&path).expect("Failed to read profile"),
            )
            .expect("Failed to parse profile");

            let account = get_oauth_account(&config);
            let marker = if account.email_address == current_email {
                " *"
            } else {
                ""
            };
            format!(
                "{} - {} @ {}{}",
                name, account.display_name, account.organization_name, marker
            )
        })
        .collect();

    // Find current selection index (default to first if not found)
    let default_index = profiles
        .iter()
        .position(|name| {
            let path = get_profile_path(name);
            let config: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(&path).unwrap_or_default(),
            )
            .unwrap_or_default();

            config
                .get("oauthAccount")
                .and_then(|a| a.get("emailAddress"))
                .and_then(|e| e.as_str())
                .map(|e| e == current_email)
                .unwrap_or(false)
        })
        .unwrap_or(0);

    let selection = Select::new()
        .with_prompt("Select Claude profile")
        .default(default_index)
        .items(&items)
        .interact_opt()
        .expect("Failed to display selection UI");

    selection.map(|idx| profiles[idx].clone())
}
