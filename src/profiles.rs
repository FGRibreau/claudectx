use std::fs;
use std::path::PathBuf;

use crate::config::claude_config_path;

/// Get the profiles directory path (~/.claudectx/)
pub fn profiles_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Cannot find home directory")
        .join(".claudectx")
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
/// If ~/.claude.json is a symlink, copy the target file
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

    // Read the content (follows symlinks automatically)
    let content = fs::read_to_string(&source).unwrap_or_else(|_| {
        panic!(
            "Failed to read Claude config at {:?} - is Claude Code installed?",
            source
        )
    });

    // Write to the profile file
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

/// Get the current profile name if ~/.claude.json is a symlink to a profile
pub fn get_current_profile() -> Option<String> {
    let config_path = claude_config_path();

    if !config_path.is_symlink() {
        return None;
    }

    let target = fs::read_link(&config_path).ok()?;
    let target_name = target.file_name()?.to_string_lossy().to_string();
    target_name.strip_suffix(".claude.json").map(String::from)
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
}
