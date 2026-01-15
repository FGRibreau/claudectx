use std::fs;
use std::path::PathBuf;

use crate::config::{claude_config_path, home_dir};

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
