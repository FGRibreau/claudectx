use std::fs;
use std::path::PathBuf;

use crate::config::{claude_config_path, home_dir};

/// Fields that are account-specific and should NOT be carried over when switching profiles.
/// These belong to the target profile's identity and must be preserved.
const ACCOUNT_SPECIFIC_FIELDS: &[&str] = &[
    "oauthAccount",
    "userID",
    "groveConfigCache",
    "cachedChromeExtensionInstalled",
    "subscriptionNoticeCount",
    "s1mAccessCache",
    "recommendedSubscription",
    "hasAvailableSubscription",
];

/// Merge portable (non-account-specific) settings from `current` into `target`.
/// Account-specific fields in `target` are preserved; all other fields from `current` overwrite `target`.
fn merge_portable_settings(current: &serde_json::Value, target: &mut serde_json::Value) {
    let (Some(current_obj), Some(target_obj)) = (current.as_object(), target.as_object_mut())
    else {
        return;
    };

    for (key, value) in current_obj {
        if !ACCOUNT_SPECIFIC_FIELDS.contains(&key.as_str()) {
            target_obj.insert(key.clone(), value.clone());
        }
    }
}

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

/// Save current ~/.claude.json as a profile.
/// If ~/.claude.json is not already a symlink, replaces it with a symlink to the saved profile.
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

    fs::write(&dest, &content).expect("Failed to save profile");

    // If source is not already a symlink, replace it with one pointing to the saved profile
    if !source.is_symlink() {
        fs::remove_file(&source).expect("Failed to remove original config");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&dest, &source).expect("Failed to create symlink");

        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&dest, &source).expect("Failed to create symlink");
    }
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

/// Switch to a profile by making ~/.claude.json a symlink to the profile.
/// Before switching, merges portable (non-account-specific) settings from the
/// current config into the target profile so preferences carry over.
pub fn switch_to_profile(name: &str) {
    let profile_path = get_profile_path(name);
    if !profile_path.exists() {
        panic!("Profile '{}' not found", slugify(name));
    }

    let config_path = claude_config_path();

    // Merge portable settings from current config into target profile
    if config_path.exists() || config_path.is_symlink() {
        let current_content = fs::read_to_string(&config_path).ok();
        let current_config: Option<serde_json::Value> =
            current_content.and_then(|c| serde_json::from_str(&c).ok());

        if let Some(current) = current_config {
            let target_content =
                fs::read_to_string(&profile_path).expect("Failed to read target profile");
            let mut target: serde_json::Value =
                serde_json::from_str(&target_content).expect("Failed to parse target profile");

            merge_portable_settings(&current, &mut target);

            let merged =
                serde_json::to_string_pretty(&target).expect("Failed to serialize merged config");
            fs::write(&profile_path, merged).expect("Failed to write merged profile");
        }

        // Remove existing file/symlink
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

    #[test]
    fn test_merge_portable_settings_overwrites_portable_fields() {
        let current = serde_json::json!({
            "hasCompletedOnboarding": true,
            "primaryApiKey": "sk-current",
            "oauthAccount": {"accountUuid": "current-uuid"}
        });
        let mut target = serde_json::json!({
            "hasCompletedOnboarding": false,
            "primaryApiKey": "sk-target",
            "oauthAccount": {"accountUuid": "target-uuid"}
        });

        merge_portable_settings(&current, &mut target);

        // Portable fields overwritten by current
        assert_eq!(target["hasCompletedOnboarding"], true);
        assert_eq!(target["primaryApiKey"], "sk-current");
        // Account-specific field preserved from target
        assert_eq!(target["oauthAccount"]["accountUuid"], "target-uuid");
    }

    #[test]
    fn test_merge_portable_settings_preserves_all_account_fields() {
        let current = serde_json::json!({
            "oauthAccount": "current",
            "userID": "current",
            "groveConfigCache": "current",
            "cachedChromeExtensionInstalled": "current",
            "subscriptionNoticeCount": "current",
            "s1mAccessCache": "current",
            "recommendedSubscription": "current",
            "hasAvailableSubscription": "current",
            "portable": "from-current"
        });
        let mut target = serde_json::json!({
            "oauthAccount": "target",
            "userID": "target",
            "groveConfigCache": "target",
            "cachedChromeExtensionInstalled": "target",
            "subscriptionNoticeCount": "target",
            "s1mAccessCache": "target",
            "recommendedSubscription": "target",
            "hasAvailableSubscription": "target",
            "portable": "from-target"
        });

        merge_portable_settings(&current, &mut target);

        for field in ACCOUNT_SPECIFIC_FIELDS {
            assert_eq!(
                target[field], "target",
                "Field '{}' should be preserved from target",
                field
            );
        }
        assert_eq!(target["portable"], "from-current");
    }

    #[test]
    fn test_merge_portable_settings_adds_new_fields() {
        let current = serde_json::json!({
            "newField": "added",
            "oauthAccount": "current"
        });
        let mut target = serde_json::json!({
            "oauthAccount": "target"
        });

        merge_portable_settings(&current, &mut target);

        assert_eq!(target["newField"], "added");
        assert_eq!(target["oauthAccount"], "target");
    }

    #[test]
    fn test_merge_portable_settings_non_objects_are_noop() {
        let current = serde_json::json!("not an object");
        let mut target = serde_json::json!({"key": "value"});

        merge_portable_settings(&current, &mut target);

        assert_eq!(target["key"], "value");
    }
}
