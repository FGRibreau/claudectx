use std::fs;
use std::path::PathBuf;

use crate::config::{claude_config_path, home_dir};

/// Extract account UUID from a config JSON value
fn get_account_uuid(config: &serde_json::Value) -> Option<String> {
    config
        .get("oauthAccount")?
        .get("accountUuid")?
        .as_str()
        .map(String::from)
}

/// Get the profiles directory path (~/.claudectx/)
pub fn profiles_dir() -> PathBuf {
    home_dir().join(".claudectx")
}

/// Ensure the profiles directory exists
pub fn ensure_profiles_dir() {
    fs::create_dir_all(profiles_dir()).expect("Failed to create profiles directory");
}

/// Slugify profile name: lowercase, replace spaces/special chars with dashes
/// "My Work Profile" → "my-work-profile"
/// "FG@Company" → "fg-company"
pub fn slugify(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// List all profile names (without .claude.json extension)
pub fn list_profiles() -> Vec<String> {
    let dir = profiles_dir();
    if !dir.exists() {
        return vec![];
    }

    fs::read_dir(dir)
        .expect("Failed to read profiles directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            name.strip_suffix(".claude.json").map(String::from)
        })
        .collect()
}

/// Get the path to a profile file
pub fn get_profile_path(name: &str) -> PathBuf {
    let slug = slugify(name);
    profiles_dir().join(format!("{}.claude.json", slug))
}

/// Save current ~/.claude.json as a profile
pub fn save_profile(name: &str) {
    let source = claude_config_path();
    if !source.exists() {
        panic!(
            "Failed to read Claude config at {:?} - is Claude Code installed?",
            source
        );
    }

    ensure_profiles_dir();
    let dest = get_profile_path(name);

    let content = fs::read_to_string(&source).unwrap_or_else(|_| {
        panic!(
            "Failed to read Claude config at {:?} - is Claude Code installed?",
            source
        )
    });

    fs::write(&dest, content).expect("Failed to save profile");
}

/// Delete a profile
pub fn delete_profile(name: &str) {
    let path = get_profile_path(name);
    fs::remove_file(&path).expect("Failed to delete profile");
}

/// Check if a profile exists
pub fn profile_exists(name: &str) -> bool {
    get_profile_path(name).exists()
}

/// Switch to a profile by making ~/.claude.json a symlink to the profile
pub fn switch_to_profile(name: &str) {
    let profile_path = get_profile_path(name);
    if !profile_path.exists() {
        panic!("Profile '{}' not found", slugify(name));
    }

    let config_path = claude_config_path();

    // Remove existing file/symlink if it exists
    if config_path.exists() || config_path.is_symlink() {
        fs::remove_file(&config_path).expect("Failed to remove existing config");
    }

    // Create symlink
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&profile_path, &config_path).expect("Failed to create symlink");
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(&profile_path, &config_path)
            .expect("Failed to create symlink");
    }
}

/// Get the current profile name by checking:
/// 1. If ~/.claude.json is a symlink to a profile (fast path)
/// 2. If ~/.claude.json content matches a profile by accountUuid (fallback)
pub fn get_current_profile() -> Option<String> {
    let config_path = claude_config_path();

    // Fast path: check if it's a symlink to a profile
    if config_path.is_symlink() {
        let target = fs::read_link(&config_path).ok()?;
        let target_name = target.file_name()?.to_string_lossy().to_string();
        return target_name.strip_suffix(".claude.json").map(String::from);
    }

    // Fallback: compare accountUuid with saved profiles
    if !config_path.exists() {
        return None;
    }

    let current_content = fs::read_to_string(&config_path).ok()?;
    let current_config: serde_json::Value = serde_json::from_str(&current_content).ok()?;
    let current_uuid = get_account_uuid(&current_config)?;

    // Search through profiles for matching accountUuid
    for profile_name in list_profiles() {
        let profile_path = get_profile_path(&profile_name);
        let profile_content = fs::read_to_string(&profile_path).ok();
        let profile_config: Option<serde_json::Value> =
            profile_content.and_then(|c| serde_json::from_str(&c).ok());

        if let Some(profile_uuid) = profile_config.and_then(|c| get_account_uuid(&c)) {
            if profile_uuid == current_uuid {
                return Some(profile_name);
            }
        }
    }

    None
}

/// Get the backup path for claude.json
pub fn claude_config_backup_path() -> PathBuf {
    home_dir().join(".claude.json.bak")
}

/// Backup ~/.claude.json to ~/.claude.json.bak if it exists
/// Returns true if a backup was created, false if no config existed
pub fn backup_claude_config() -> bool {
    let config_path = claude_config_path();
    let backup_path = claude_config_backup_path();

    if config_path.exists() || config_path.is_symlink() {
        // Read actual content (follows symlink)
        let content = fs::read_to_string(&config_path).expect("Failed to read Claude config");
        fs::write(&backup_path, content).expect("Failed to create backup");
        // Remove the original (or symlink)
        fs::remove_file(&config_path).expect("Failed to remove original config");
        true
    } else {
        false
    }
}

/// Restore ~/.claude.json from backup, or remove the current config if no backup exists
/// - If backup exists: restore it and remove backup
/// - If no backup: just remove the current config (if any)
pub fn restore_claude_config(had_backup: bool) {
    let config_path = claude_config_path();
    let backup_path = claude_config_backup_path();

    // Remove current config if it exists
    if config_path.exists() || config_path.is_symlink() {
        fs::remove_file(&config_path).expect("Failed to remove current config");
    }

    if had_backup && backup_path.exists() {
        fs::rename(&backup_path, &config_path).expect("Failed to restore backup");
    }
}

/// Check if claude.json exists (as file or symlink)
pub fn claude_config_exists() -> bool {
    let config_path = claude_config_path();
    config_path.exists() || config_path.is_symlink()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_simple() {
        assert_eq!(slugify("fg"), "fg");
        assert_eq!(slugify("FG"), "fg");
    }

    #[test]
    fn test_slugify_spaces() {
        assert_eq!(slugify("My Work Profile"), "my-work-profile");
        assert_eq!(slugify("  test  "), "test");
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(slugify("FG@Company"), "fg-company");
        assert_eq!(slugify("test!@#$%name"), "test-name");
    }

    #[test]
    fn test_slugify_multiple_dashes() {
        assert_eq!(slugify("test---name"), "test-name");
        assert_eq!(slugify("a - b - c"), "a-b-c");
    }

    #[test]
    fn test_backup_path() {
        // Test that backup path is derived correctly
        let backup_path = super::claude_config_backup_path();
        assert!(backup_path.to_string_lossy().ends_with(".claude.json.bak"));
    }
}
