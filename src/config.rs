use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// OAuth account structure from ~/.claude.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthAccount {
    pub account_uuid: String,
    pub email_address: String,
    pub organization_uuid: String,
    pub display_name: String,
    pub organization_role: String,
    pub organization_name: String,
    pub has_extra_usage_enabled: bool,
    pub workspace_role: Option<String>,
}

/// Get the home directory, with CLAUDECTX_HOME override for testing.
/// This is needed because dirs::home_dir() doesn't respect USERPROFILE
/// environment variable when set for child processes on Windows.
pub fn home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("CLAUDECTX_HOME") {
        return PathBuf::from(home);
    }
    dirs::home_dir().expect("Failed to find home directory")
}

/// Get the path to ~/.claude.json
pub fn claude_config_path() -> PathBuf {
    home_dir().join(".claude.json")
}

/// Read the Claude config file as a JSON Value (preserves all fields)
pub fn read_claude_config() -> serde_json::Value {
    let path = claude_config_path();
    let content = fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "Failed to read Claude config at {:?} - is Claude Code installed?",
            path
        )
    });
    serde_json::from_str(&content).expect("Failed to parse Claude config JSON")
}

/// Extract the oauthAccount from the config
pub fn get_oauth_account(config: &serde_json::Value) -> OAuthAccount {
    let account_value = config
        .get("oauthAccount")
        .expect("oauthAccount field is missing from claude.json");
    serde_json::from_value(account_value.clone()).expect("Failed to parse oauthAccount")
}
