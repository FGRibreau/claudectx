use std::fs;
use std::path::PathBuf;

use crate::config::{claude_config_path, home_dir};

/// Fields that are account-specific and stored in slim profile files.
/// Everything else in ~/.claude.json is portable (settings, preferences, etc.)
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

/// Extract only the account-specific fields from a config JSON object.
/// Returns a new JSON object containing only the 8 account-specific keys.
fn extract_account_fields(config: &serde_json::Value) -> serde_json::Value {
    let Some(obj) = config.as_object() else {
        return serde_json::json!({});
    };

    let mut result = serde_json::Map::new();
    for &field in ACCOUNT_SPECIFIC_FIELDS {
        if let Some(value) = obj.get(field) {
            result.insert(field.to_string(), value.clone());
        }
    }
    serde_json::Value::Object(result)
}

/// Overwrite account-specific keys in `config` with values from `profile`.
/// Keys present in ACCOUNT_SPECIFIC_FIELDS but absent from `profile` are
/// removed from `config` to prevent data leakage between accounts.
fn patch_account_fields(config: &mut serde_json::Value, profile: &serde_json::Value) {
    let (Some(config_obj), Some(profile_obj)) = (config.as_object_mut(), profile.as_object())
    else {
        return;
    };

    for &field in ACCOUNT_SPECIFIC_FIELDS {
        match profile_obj.get(field) {
            Some(value) => {
                config_obj.insert(field.to_string(), value.clone());
            }
            None => {
                config_obj.remove(field);
            }
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
            // Exclude .bak files from listing
            if name.ends_with(".bak") {
                return None;
            }
            name.strip_suffix(".claude.json").map(String::from)
        })
        .collect()
}

/// Get the path to a profile file
pub fn get_profile_path(name: &str) -> PathBuf {
    let slug = slugify(name);
    profiles_dir().join(format!("{}.claude.json", slug))
}

/// Save current ~/.claude.json as a slim profile (account-specific fields only).
/// ~/.claude.json stays a regular file, untouched.
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

    let config: serde_json::Value =
        serde_json::from_str(&content).expect("Failed to parse Claude config JSON");

    let slim = extract_account_fields(&config);
    let slim_json = serde_json::to_string_pretty(&slim).expect("Failed to serialize slim profile");

    fs::write(&dest, slim_json).expect("Failed to save profile");
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

/// Switch to a profile by patching ~/.claude.json in-place.
/// Only the 8 account-specific fields are touched; all other settings are preserved.
/// The profile file is read-only and never modified.
pub fn switch_to_profile(name: &str) {
    let profile_path = get_profile_path(name);
    if !profile_path.exists() {
        panic!("Profile '{}' not found", slugify(name));
    }

    let config_path = claude_config_path();

    // Read the slim profile
    let profile_content = fs::read_to_string(&profile_path).expect("Failed to read target profile");
    let profile: serde_json::Value =
        serde_json::from_str(&profile_content).expect("Failed to parse target profile");

    // Read current config or start from empty object
    let mut config: serde_json::Value = if config_path.exists() {
        let content = fs::read_to_string(&config_path).unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Patch only account-specific fields
    patch_account_fields(&mut config, &profile);

    // Write back
    let output = serde_json::to_string_pretty(&config).expect("Failed to serialize config");
    fs::write(&config_path, output).expect("Failed to write config");
}

/// Get the current profile name by comparing accountUuid in ~/.claude.json
/// with saved profiles.
pub fn get_current_profile() -> Option<String> {
    let config_path = claude_config_path();

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

    if config_path.exists() {
        let content = fs::read_to_string(&config_path).expect("Failed to read Claude config");
        fs::write(&backup_path, content).expect("Failed to create backup");
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
    if config_path.exists() {
        fs::remove_file(&config_path).expect("Failed to remove current config");
    }

    if had_backup && backup_path.exists() {
        fs::rename(&backup_path, &config_path).expect("Failed to restore backup");
    }
}

/// Check if claude.json exists
pub fn claude_config_exists() -> bool {
    let config_path = claude_config_path();
    config_path.exists()
}

/// One-shot migration from symlink-based to slim-profile architecture.
/// Triggered only when ~/.claude.json is a symlink (old architecture).
/// On subsequent runs, is_symlink() returns false → no-op.
pub fn migrate_if_needed() {
    let config_path = claude_config_path();

    if !config_path.is_symlink() {
        return;
    }

    // 1. Read content through the symlink
    let content =
        fs::read_to_string(&config_path).expect("Failed to read Claude config through symlink");

    // 2. Remove the symlink
    fs::remove_file(&config_path).expect("Failed to remove symlink");

    // 3. Write the content as a regular file
    fs::write(&config_path, &content).expect("Failed to write config as regular file");

    // 4. Slim down each profile in ~/.claudectx/
    let dir = profiles_dir();
    if dir.exists() {
        let entries: Vec<_> = fs::read_dir(&dir)
            .expect("Failed to read profiles directory")
            .filter_map(|e| e.ok())
            .collect();

        for entry in entries {
            let path = entry.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            if !name.ends_with(".claude.json") || name.ends_with(".bak") {
                continue;
            }

            // a. Create backup
            let backup_path = path.with_extension("json.bak");
            fs::copy(&path, &backup_path).expect("Failed to create profile backup");

            // b. Rewrite with only account-specific fields
            let profile_content =
                fs::read_to_string(&path).expect("Failed to read profile for migration");
            let profile_config: serde_json::Value = serde_json::from_str(&profile_content)
                .expect("Failed to parse profile for migration");

            let slim = extract_account_fields(&profile_config);
            let slim_json =
                serde_json::to_string_pretty(&slim).expect("Failed to serialize slim profile");
            fs::write(&path, slim_json).expect("Failed to write slim profile");
        }
    }

    println!("Migrated profiles to slim format (backups in ~/.claudectx/*.bak)");
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
        let backup_path = super::claude_config_backup_path();
        assert!(backup_path.to_string_lossy().ends_with(".claude.json.bak"));
    }

    #[test]
    fn test_extract_account_fields_returns_only_account_keys() {
        let config = serde_json::json!({
            "oauthAccount": {"accountUuid": "uuid-123"},
            "userID": "user-123",
            "groveConfigCache": {"key": "value"},
            "cachedChromeExtensionInstalled": true,
            "subscriptionNoticeCount": 3,
            "s1mAccessCache": {"cache": true},
            "recommendedSubscription": "pro",
            "hasAvailableSubscription": true,
            "hasCompletedOnboarding": true,
            "primaryApiKey": "sk-key",
            "customSetting": "custom"
        });

        let slim = extract_account_fields(&config);
        let obj = slim.as_object().unwrap();

        // Only account-specific keys present
        assert_eq!(obj.len(), 8);
        assert_eq!(slim["oauthAccount"]["accountUuid"], "uuid-123");
        assert_eq!(slim["userID"], "user-123");
        assert_eq!(slim["groveConfigCache"]["key"], "value");
        assert_eq!(slim["cachedChromeExtensionInstalled"], true);
        assert_eq!(slim["subscriptionNoticeCount"], 3);
        assert_eq!(slim["s1mAccessCache"]["cache"], true);
        assert_eq!(slim["recommendedSubscription"], "pro");
        assert_eq!(slim["hasAvailableSubscription"], true);

        // Portable keys excluded
        assert!(obj.get("hasCompletedOnboarding").is_none());
        assert!(obj.get("primaryApiKey").is_none());
        assert!(obj.get("customSetting").is_none());
    }

    #[test]
    fn test_extract_account_fields_handles_missing_keys() {
        let config = serde_json::json!({
            "oauthAccount": {"accountUuid": "uuid-only"},
            "hasCompletedOnboarding": true
        });

        let slim = extract_account_fields(&config);
        let obj = slim.as_object().unwrap();

        // Only the one account field present
        assert_eq!(obj.len(), 1);
        assert_eq!(slim["oauthAccount"]["accountUuid"], "uuid-only");
    }

    #[test]
    fn test_patch_account_fields_overwrites_existing_keys() {
        let mut config = serde_json::json!({
            "oauthAccount": {"accountUuid": "old-uuid"},
            "userID": "old-user",
            "hasCompletedOnboarding": true
        });

        let profile = serde_json::json!({
            "oauthAccount": {"accountUuid": "new-uuid"},
            "userID": "new-user"
        });

        patch_account_fields(&mut config, &profile);

        assert_eq!(config["oauthAccount"]["accountUuid"], "new-uuid");
        assert_eq!(config["userID"], "new-user");
        // Portable field untouched
        assert_eq!(config["hasCompletedOnboarding"], true);
    }

    #[test]
    fn test_patch_account_fields_removes_absent_keys() {
        let mut config = serde_json::json!({
            "oauthAccount": {"accountUuid": "uuid"},
            "userID": "user-id",
            "groveConfigCache": {"old": true},
            "hasCompletedOnboarding": true
        });

        // Profile only has oauthAccount — userID and groveConfigCache should be removed
        let profile = serde_json::json!({
            "oauthAccount": {"accountUuid": "new-uuid"}
        });

        patch_account_fields(&mut config, &profile);

        assert_eq!(config["oauthAccount"]["accountUuid"], "new-uuid");
        assert!(config.get("userID").is_none());
        assert!(config.get("groveConfigCache").is_none());
        // Portable field untouched
        assert_eq!(config["hasCompletedOnboarding"], true);
    }

    #[test]
    fn test_patch_account_fields_leaves_portable_fields_untouched() {
        let mut config = serde_json::json!({
            "oauthAccount": {"accountUuid": "old"},
            "hasCompletedOnboarding": true,
            "primaryApiKey": "sk-key",
            "customSetting": "value",
            "editorTheme": "dark"
        });

        let profile = serde_json::json!({
            "oauthAccount": {"accountUuid": "new"}
        });

        patch_account_fields(&mut config, &profile);

        // Portable fields all untouched
        assert_eq!(config["hasCompletedOnboarding"], true);
        assert_eq!(config["primaryApiKey"], "sk-key");
        assert_eq!(config["customSetting"], "value");
        assert_eq!(config["editorTheme"], "dark");
        // Account field updated
        assert_eq!(config["oauthAccount"]["accountUuid"], "new");
    }
}
