//! End-to-end tests for claudectx CLI
//!
//! These tests run the actual binary in a sandboxed environment using
//! temporary directories as HOME to avoid interfering with real config files.
//!
//! Note: Tests that would launch claude are limited since claude is not
//! installed in the CI environment. We test save/list/delete thoroughly
//! and verify in-place config patching works correctly.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Create a test environment with a temporary HOME directory
struct TestEnv {
    home_dir: TempDir,
}

impl TestEnv {
    fn new() -> Self {
        let home_dir = TempDir::new().expect("Failed to create temp directory");
        Self { home_dir }
    }

    /// Get the path to the home directory
    fn home_path(&self) -> &Path {
        self.home_dir.path()
    }

    /// Get path to .claude.json in test environment
    fn claude_config_path(&self) -> std::path::PathBuf {
        self.home_dir.path().join(".claude.json")
    }

    /// Get path to .claudectx/ directory in test environment
    fn claudectx_dir(&self) -> std::path::PathBuf {
        self.home_dir.path().join(".claudectx")
    }

    /// Get path to a profile file
    fn profile_path(&self, name: &str) -> std::path::PathBuf {
        self.claudectx_dir().join(format!("{}.claude.json", name))
    }

    /// Create a valid .claude.json config file (as a regular file)
    fn create_claude_config(&self, account: &serde_json::Value) {
        let config_path = self.claude_config_path();
        let config = json!({
            "oauthAccount": account,
            "lastAccountUUID": account["accountUuid"],
            "primaryApiKey": "sk-ant-test-key",
            "hasCompletedOnboarding": true
        });
        fs::write(
            &config_path,
            serde_json::to_string_pretty(&config).expect("serialize"),
        )
        .expect("Failed to write claude config");
    }

    /// Create a slim profile file (only account-specific fields)
    fn create_profile(&self, name: &str, account: &serde_json::Value) {
        fs::create_dir_all(self.claudectx_dir()).expect("Failed to create claudectx dir");
        let config = json!({
            "oauthAccount": account,
            "userID": format!("user-id-{}", name)
        });
        fs::write(
            self.profile_path(name),
            serde_json::to_string_pretty(&config).expect("serialize"),
        )
        .expect("Failed to write profile");
    }

    /// Read a profile file
    fn read_profile(&self, name: &str) -> serde_json::Value {
        let content = fs::read_to_string(self.profile_path(name)).expect("Failed to read profile");
        serde_json::from_str(&content).expect("Failed to parse profile")
    }

    /// Read ~/.claude.json as parsed JSON
    fn read_claude_config(&self) -> serde_json::Value {
        let content =
            fs::read_to_string(self.claude_config_path()).expect("Failed to read claude config");
        serde_json::from_str(&content).expect("Failed to parse claude config")
    }

    /// List profile files in the claudectx directory
    fn list_profile_files(&self) -> Vec<String> {
        if !self.claudectx_dir().exists() {
            return vec![];
        }
        fs::read_dir(self.claudectx_dir())
            .expect("Failed to read claudectx dir")
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".bak") {
                    return None;
                }
                name.strip_suffix(".claude.json").map(String::from)
            })
            .collect()
    }

    /// Run claudectx command with this test environment
    fn cmd(&self) -> assert_cmd::Command {
        let mut cmd = Command::cargo_bin("claudectx").expect("Failed to find binary");
        // Use CLAUDECTX_HOME for reliable cross-platform home directory override
        cmd.env("CLAUDECTX_HOME", self.home_path());
        assert_cmd::Command::from_std(cmd)
    }
}

/// Create a sample OAuth account for testing
fn sample_account(suffix: &str) -> serde_json::Value {
    json!({
        "accountUuid": format!("uuid-{}", suffix),
        "emailAddress": format!("user-{}@example.com", suffix),
        "organizationUuid": format!("org-uuid-{}", suffix),
        "displayName": format!("User {}", suffix),
        "organizationRole": "member",
        "organizationName": format!("Org {}", suffix),
        "hasExtraUsageEnabled": false,
        "workspaceRole": null
    })
}

// =============================================================================
// HELP AND VERSION TESTS
// =============================================================================

#[test]
fn test_help_flag() {
    let env = TestEnv::new();
    env.cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Launch Claude Code with different profiles",
        ))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("save"))
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn test_version_flag() {
    let env = TestEnv::new();
    env.cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("claudectx"));
}

#[test]
fn test_help_subcommand() {
    let env = TestEnv::new();
    env.cmd()
        .arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Launch Claude Code with different profiles",
        ));
}

// =============================================================================
// LIST COMMAND TESTS
// =============================================================================

#[test]
fn test_list_empty_profiles() {
    let env = TestEnv::new();
    let account = sample_account("current");
    env.create_claude_config(&account);
    // No profiles directory

    env.cmd()
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No profiles found."));
}

#[test]
fn test_list_with_profiles() {
    let env = TestEnv::new();
    let current_account = sample_account("current");
    env.create_claude_config(&current_account);

    // Create profile files directly
    env.create_profile("work", &sample_account("work"));
    env.create_profile("personal", &sample_account("personal"));

    env.cmd()
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("work"))
        .stdout(predicate::str::contains("personal"))
        .stdout(predicate::str::contains("User work"))
        .stdout(predicate::str::contains("User personal"));
}

// =============================================================================
// SAVE COMMAND TESTS
// =============================================================================

#[test]
fn test_save_creates_new_profile() {
    let env = TestEnv::new();
    let account = sample_account("alice");
    env.create_claude_config(&account);

    env.cmd()
        .args(["save", "alice-profile"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Saved current config as 'alice-profile'",
        ));

    // Verify profile file was created
    assert!(env.profile_path("alice-profile").exists());
    let profile = env.read_profile("alice-profile");
    assert_eq!(
        profile["oauthAccount"]["emailAddress"],
        "user-alice@example.com"
    );
}

#[test]
fn test_save_slugifies_profile_name() {
    let env = TestEnv::new();
    let account = sample_account("test");
    env.create_claude_config(&account);

    env.cmd()
        .args(["save", "My Work Profile"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Saved current config as 'my-work-profile'",
        ));

    // Verify slugified filename
    assert!(env.profile_path("my-work-profile").exists());
}

#[test]
fn test_save_slugifies_special_characters() {
    let env = TestEnv::new();
    let account = sample_account("test");
    env.create_claude_config(&account);

    env.cmd()
        .args(["save", "FG@Company"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Saved current config as 'fg-company'",
        ));

    assert!(env.profile_path("fg-company").exists());
}

#[test]
fn test_save_fails_without_claude_config() {
    let env = TestEnv::new();
    // No .claude.json

    env.cmd()
        .args(["save", "myprofile"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read Claude config"));
}

#[test]
fn test_save_multiple_profiles() {
    let env = TestEnv::new();

    // Save first profile
    let account1 = sample_account("first");
    env.create_claude_config(&account1);
    env.cmd().args(["save", "profile1"]).assert().success();

    // Save second profile (create new config for different account)
    let account2 = sample_account("second");
    env.create_claude_config(&account2);
    env.cmd().args(["save", "profile2"]).assert().success();

    // Verify both profiles exist
    let profiles = env.list_profile_files();
    assert!(profiles.contains(&"profile1".to_string()));
    assert!(profiles.contains(&"profile2".to_string()));
}

#[test]
fn test_save_keeps_claude_json_as_regular_file() {
    let env = TestEnv::new();
    let account = sample_account("regular");
    env.create_claude_config(&account);

    env.cmd().args(["save", "my-profile"]).assert().success();

    // .claude.json must remain a regular file, NOT a symlink
    let config_path = env.claude_config_path();
    assert!(
        !config_path.is_symlink(),
        ".claude.json should remain a regular file after save"
    );
    assert!(
        config_path.exists(),
        ".claude.json should still exist after save"
    );
}

#[test]
fn test_saved_profile_has_only_account_fields() {
    let env = TestEnv::new();
    let account = json!({
        "accountUuid": "uuid-integrity",
        "emailAddress": "integrity@example.com",
        "organizationUuid": "org-uuid-integrity",
        "displayName": "Integrity User",
        "organizationRole": "admin",
        "organizationName": "Integrity Org",
        "hasExtraUsageEnabled": true,
        "workspaceRole": "owner"
    });

    // Create config with extra portable fields
    let config = json!({
        "oauthAccount": account,
        "lastAccountUUID": account["accountUuid"],
        "primaryApiKey": "sk-ant-test-key",
        "hasCompletedOnboarding": true,
        "customField": "custom-value",
        "nestedField": {
            "inner": "value"
        }
    });
    fs::write(
        env.claude_config_path(),
        serde_json::to_string_pretty(&config).expect("serialize"),
    )
    .expect("Failed to write config");

    env.cmd()
        .args(["save", "integrity-test"])
        .assert()
        .success();

    let profile = env.read_profile("integrity-test");
    let obj = profile.as_object().unwrap();

    // Profile should contain ONLY account-specific fields
    assert_eq!(profile["oauthAccount"]["accountUuid"], "uuid-integrity");
    // Portable fields should NOT be in the profile
    assert!(
        obj.get("primaryApiKey").is_none(),
        "primaryApiKey should not be in slim profile"
    );
    assert!(
        obj.get("hasCompletedOnboarding").is_none(),
        "hasCompletedOnboarding should not be in slim profile"
    );
    assert!(
        obj.get("customField").is_none(),
        "customField should not be in slim profile"
    );
    assert!(
        obj.get("nestedField").is_none(),
        "nestedField should not be in slim profile"
    );
    assert!(
        obj.get("lastAccountUUID").is_none(),
        "lastAccountUUID should not be in slim profile"
    );
}

// =============================================================================
// DELETE COMMAND TESTS
// =============================================================================

#[test]
fn test_delete_removes_profile() {
    let env = TestEnv::new();
    let account = sample_account("current");
    env.create_claude_config(&account);

    env.create_profile("to-delete", &sample_account("delete-me"));
    env.create_profile("to-keep", &sample_account("keep-me"));

    env.cmd()
        .args(["delete", "to-delete"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted profile 'to-delete'"));

    // Verify profile was deleted
    assert!(!env.profile_path("to-delete").exists());
    assert!(env.profile_path("to-keep").exists());
}

#[test]
fn test_delete_nonexistent_profile_panics() {
    let env = TestEnv::new();
    let account = sample_account("current");
    env.create_claude_config(&account);

    env.cmd()
        .args(["delete", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Profile 'nonexistent' not found"));
}

// =============================================================================
// NO-ARGS (INTERACTIVE MODE) TESTS
// =============================================================================

#[test]
fn test_no_args_first_launch_no_profiles() {
    let env = TestEnv::new();
    let account = sample_account("firstuser");
    env.create_claude_config(&account);
    // No profiles

    env.cmd()
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Current account: User firstuser @ Org firstuser",
        ))
        .stdout(predicate::str::contains("No profiles saved yet"))
        .stdout(predicate::str::contains("claudectx save"));
}

#[test]
fn test_no_args_fails_without_claude_config() {
    let env = TestEnv::new();
    // No .claude.json, no profiles - should try interactive mode and fail

    env.cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read Claude config"));
}

// =============================================================================
// LAUNCH PROFILE TESTS (in-place patching + claude launch)
// =============================================================================

#[test]
fn test_launch_nonexistent_profile_panics() {
    let env = TestEnv::new();

    // Create a config file
    let account = sample_account("current");
    env.create_claude_config(&account);

    // Try to launch nonexistent profile (will prompt to create)
    // Since we can't interact with prompts in tests, this should fail
    // The test binary runs without a TTY so dialoguer will fail
    env.cmd().arg("nonexistent").assert().failure();
}

#[test]
fn test_launch_patches_config_with_target_account() {
    let env = TestEnv::new();
    let account = sample_account("current");
    env.create_claude_config(&account);

    // Create a profile
    env.create_profile("work", &sample_account("work"));

    // Launch - this should patch ~/.claude.json with work account fields
    // then try to launch claude (which will fail in CI)
    let _ = env.cmd().arg("work").assert();

    // ~/.claude.json should have the work account's UUID
    let config = env.read_claude_config();
    assert_eq!(
        config["oauthAccount"]["accountUuid"], "uuid-work",
        "Config should have work profile's accountUuid after launch"
    );

    // The profile file should still exist and be unchanged
    assert!(env.profile_path("work").exists());
}

#[test]
fn test_launch_switches_account_between_profiles() {
    let env = TestEnv::new();

    // Create profiles
    env.create_profile("work", &sample_account("work"));
    env.create_profile("personal", &sample_account("personal"));

    // Create initial config
    let account = sample_account("initial");
    env.create_claude_config(&account);

    // Launch work profile
    let _ = env.cmd().arg("work").assert();
    let config = env.read_claude_config();
    assert_eq!(
        config["oauthAccount"]["accountUuid"], "uuid-work",
        "Should have work accountUuid"
    );

    // Launch personal profile
    let _ = env.cmd().arg("personal").assert();
    let config = env.read_claude_config();
    assert_eq!(
        config["oauthAccount"]["accountUuid"], "uuid-personal",
        "Should have personal accountUuid"
    );
}

// =============================================================================
// EDGE CASES AND ERROR HANDLING
// =============================================================================

#[test]
fn test_malformed_profile_panics() {
    let env = TestEnv::new();
    // Write invalid JSON to profile
    fs::create_dir_all(env.claudectx_dir()).expect("Failed to create dir");
    fs::write(env.profile_path("bad"), "not valid json {{{")
        .expect("Failed to write invalid profile");

    env.cmd()
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to parse profile"));
}

// =============================================================================
// INTEGRATION TESTS - FULL WORKFLOWS
// =============================================================================

#[test]
fn test_workflow_save_list_launch_delete() {
    let env = TestEnv::new();
    let account = sample_account("workflow");
    env.create_claude_config(&account);

    // 1. Save a profile
    env.cmd().args(["save", "test-profile"]).assert().success();

    // 2. List profiles - should show the saved profile
    env.cmd()
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("test-profile"))
        .stdout(predicate::str::contains("User workflow"));

    // 3. Launch the profile (patches config in-place)
    let _ = env.cmd().arg("test-profile").assert();
    let config = env.read_claude_config();
    assert_eq!(config["oauthAccount"]["accountUuid"], "uuid-workflow");

    // 4. List again - test-profile should be marked with *
    let output = env.cmd().arg("list").assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout
        .lines()
        .any(|l| l.contains("test-profile") && l.contains(" *")));

    // 5. Delete the profile
    env.cmd()
        .args(["delete", "test-profile"])
        .assert()
        .success();

    // 6. List again - should be empty
    env.cmd()
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No profiles found."));
}

#[test]
fn test_workflow_multiple_accounts() {
    let env = TestEnv::new();

    // Save work account
    let work_account = sample_account("work");
    env.create_claude_config(&work_account);
    env.cmd().args(["save", "work"]).assert().success();

    // Save personal account
    let personal_account = sample_account("personal");
    env.create_claude_config(&personal_account);
    env.cmd().args(["save", "personal"]).assert().success();

    // Save side-project account
    let side_account = sample_account("side");
    env.create_claude_config(&side_account);
    env.cmd().args(["save", "side-project"]).assert().success();

    // Launch work profile
    let _ = env.cmd().arg("work").assert();

    // List all profiles - work should be marked current
    let output = env.cmd().arg("list").assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("work"));
    assert!(stdout.contains("personal"));
    assert!(stdout.contains("side-project"));
    // work should be marked with *
    assert!(stdout
        .lines()
        .any(|l| l.contains("work") && l.contains(" *")));
}

#[test]
fn test_profiles_persistence_across_commands() {
    let env = TestEnv::new();
    let account = sample_account("persist");
    env.create_claude_config(&account);

    // Save profile
    env.cmd()
        .args(["save", "persistent-profile"])
        .assert()
        .success();

    // Verify the file exists
    assert!(env.profile_path("persistent-profile").exists());

    // Run list in a new command invocation
    env.cmd()
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("persistent-profile"));
}

// =============================================================================
// SUBCOMMAND HELP TESTS
// =============================================================================

#[test]
fn test_save_help() {
    let env = TestEnv::new();
    env.cmd()
        .args(["save", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Save current config as a new profile",
        ))
        .stdout(predicate::str::contains("<NAME>"));
}

#[test]
fn test_delete_help() {
    let env = TestEnv::new();
    env.cmd()
        .args(["delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Delete a profile"))
        .stdout(predicate::str::contains("<NAME>"));
}

#[test]
fn test_list_help() {
    let env = TestEnv::new();
    env.cmd()
        .args(["list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List all saved profiles"));
}

// =============================================================================
// ARGUMENT VALIDATION TESTS
// =============================================================================

#[test]
fn test_save_requires_name_argument() {
    let env = TestEnv::new();
    env.cmd()
        .arg("save")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_delete_requires_name_argument() {
    let env = TestEnv::new();
    env.cmd()
        .arg("delete")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

// =============================================================================
// SLUGIFY TESTS (via CLI)
// =============================================================================

#[test]
fn test_slugify_uppercase_to_lowercase() {
    let env = TestEnv::new();
    let account = sample_account("test");
    env.create_claude_config(&account);

    env.cmd()
        .args(["save", "UPPERCASE"])
        .assert()
        .success()
        .stdout(predicate::str::contains("'uppercase'"));

    assert!(env.profile_path("uppercase").exists());
}

#[test]
fn test_slugify_handles_multiple_dashes() {
    let env = TestEnv::new();
    let account = sample_account("test");
    env.create_claude_config(&account);

    env.cmd()
        .args(["save", "test---name"])
        .assert()
        .success()
        .stdout(predicate::str::contains("'test-name'"));

    assert!(env.profile_path("test-name").exists());
}

// =============================================================================
// LOGIN COMMAND TESTS
// =============================================================================

#[test]
fn test_login_help() {
    let env = TestEnv::new();
    env.cmd()
        .args(["login", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Login to a new Claude account and save it as a profile",
        ));
}

#[test]
fn test_help_includes_login_command() {
    let env = TestEnv::new();
    env.cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("login"));
}

// =============================================================================
// BACKUP/RESTORE TESTS
// =============================================================================

impl TestEnv {
    /// Get path to .claude.json.bak in test environment
    fn claude_config_backup_path(&self) -> std::path::PathBuf {
        self.home_dir.path().join(".claude.json.bak")
    }
}

#[test]
fn test_backup_file_location() {
    let env = TestEnv::new();
    let account = sample_account("backup-test");
    env.create_claude_config(&account);

    // The backup path should be in the test home directory
    let backup_path = env.claude_config_backup_path();
    assert!(backup_path.starts_with(env.home_path()));
    assert!(backup_path.ends_with(".claude.json.bak"));
}

// =============================================================================
// CURRENT PROFILE DETECTION TESTS
// =============================================================================

#[test]
fn test_list_marks_current_profile_when_config_matches_profile_content() {
    let env = TestEnv::new();

    // Create two profiles directly
    let work_account = sample_account("work");
    let personal_account = sample_account("personal");
    env.create_profile("work", &work_account);
    env.create_profile("personal", &personal_account);

    // Set .claude.json to same account as "work" profile (regular file)
    env.create_claude_config(&work_account);

    // Verify it's not a symlink
    assert!(
        !env.claude_config_path().is_symlink(),
        ".claude.json should be a regular file, not a symlink"
    );

    // List should show asterisk for "work" profile because content matches
    let output = env.cmd().arg("list").assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    assert!(
        stdout
            .lines()
            .any(|l| l.contains("work") && l.contains(" *")),
        "Profile 'work' should be marked with asterisk when config content matches. Output:\n{}",
        stdout
    );

    // The "personal" profile should NOT be marked
    assert!(
        stdout
            .lines()
            .any(|l| l.contains("personal") && !l.contains(" *")),
        "Profile 'personal' should NOT be marked with asterisk. Output:\n{}",
        stdout
    );
}

#[test]
fn test_list_no_asterisk_when_config_matches_no_profile() {
    let env = TestEnv::new();

    // Create two profiles
    env.create_profile("work", &sample_account("work"));
    env.create_profile("personal", &sample_account("personal"));

    // Set .claude.json to different content (doesn't match any profile)
    let different_account = sample_account("different");
    env.create_claude_config(&different_account);

    // List should show NO asterisk for any profile
    let output = env.cmd().arg("list").assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // No profile should be marked
    assert!(
        !stdout.contains(" *"),
        "No profile should be marked when config doesn't match any profile. Output:\n{}",
        stdout
    );
}

#[test]
fn test_save_then_list_shows_asterisk_for_saved_profile() {
    let env = TestEnv::new();

    // Create a claude config and save it as "my-profile"
    let account = sample_account("my-account");
    env.create_claude_config(&account);
    env.cmd().args(["save", "my-profile"]).assert().success();

    // .claude.json should remain a regular file (no symlink)
    assert!(
        !env.claude_config_path().is_symlink(),
        ".claude.json should be a regular file after save"
    );

    // List should show asterisk for "my-profile" because accountUuid matches
    let output = env.cmd().arg("list").assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    assert!(
        stdout
            .lines()
            .any(|l| l.contains("my-profile") && l.contains(" *")),
        "Just-saved profile should be marked as current. Output:\n{}",
        stdout
    );
}

// =============================================================================
// PORTABLE SETTINGS MERGE TESTS (in-place patching)
// =============================================================================

#[test]
fn test_switch_preserves_portable_settings_in_config() {
    let env = TestEnv::new();

    // Create current config with portable settings and account-specific fields
    let current_config = json!({
        "oauthAccount": sample_account("current"),
        "userID": "current-user-id",
        "hasCompletedOnboarding": true,
        "primaryApiKey": "sk-current-key",
        "customSetting": "my-custom-value",
        "editorTheme": "dark"
    });
    fs::write(
        env.claude_config_path(),
        serde_json::to_string_pretty(&current_config).expect("serialize"),
    )
    .expect("write");

    // Create target profile (slim: only account-specific fields)
    fs::create_dir_all(env.claudectx_dir()).expect("mkdir");
    let target_profile = json!({
        "oauthAccount": sample_account("target"),
        "userID": "target-user-id"
    });
    fs::write(
        env.profile_path("target"),
        serde_json::to_string_pretty(&target_profile).expect("serialize"),
    )
    .expect("write");

    // Switch to target profile
    let _ = env.cmd().arg("target").assert();

    // Read ~/.claude.json (NOT the profile file — the main config)
    let config = env.read_claude_config();

    // Account-specific fields should come from the TARGET profile
    assert_eq!(config["oauthAccount"]["accountUuid"], "uuid-target");
    assert_eq!(config["userID"], "target-user-id");

    // Portable settings should be PRESERVED from original config
    assert_eq!(config["hasCompletedOnboarding"], true);
    assert_eq!(config["primaryApiKey"], "sk-current-key");
    assert_eq!(config["customSetting"], "my-custom-value");
    assert_eq!(config["editorTheme"], "dark");
}

#[test]
fn test_switch_preserves_account_specific_fields_from_target() {
    let env = TestEnv::new();

    // Current config with all account-specific fields
    let current_config = json!({
        "oauthAccount": sample_account("current"),
        "userID": "current-user-id",
        "groveConfigCache": {"current": true},
        "cachedChromeExtensionInstalled": true,
        "subscriptionNoticeCount": 5,
        "s1mAccessCache": {"current": "data"},
        "recommendedSubscription": "pro",
        "hasAvailableSubscription": true,
        "portableSetting": "from-current"
    });
    fs::write(
        env.claude_config_path(),
        serde_json::to_string_pretty(&current_config).expect("serialize"),
    )
    .expect("write");

    // Target profile with its own account-specific fields
    fs::create_dir_all(env.claudectx_dir()).expect("mkdir");
    let target_profile = json!({
        "oauthAccount": sample_account("target"),
        "userID": "target-user-id",
        "groveConfigCache": {"target": true},
        "cachedChromeExtensionInstalled": false,
        "subscriptionNoticeCount": 0,
        "s1mAccessCache": {"target": "data"},
        "recommendedSubscription": "free",
        "hasAvailableSubscription": false
    });
    fs::write(
        env.profile_path("target"),
        serde_json::to_string_pretty(&target_profile).expect("serialize"),
    )
    .expect("write");

    // Switch to target
    let _ = env.cmd().arg("target").assert();

    // Read ~/.claude.json
    let config = env.read_claude_config();

    // ALL account-specific fields must come from the TARGET profile
    assert_eq!(config["oauthAccount"]["accountUuid"], "uuid-target");
    assert_eq!(config["userID"], "target-user-id");
    assert_eq!(config["groveConfigCache"]["target"], true);
    assert_eq!(config["cachedChromeExtensionInstalled"], false);
    assert_eq!(config["subscriptionNoticeCount"], 0);
    assert_eq!(config["s1mAccessCache"]["target"], "data");
    assert_eq!(config["recommendedSubscription"], "free");
    assert_eq!(config["hasAvailableSubscription"], false);

    // Portable setting should be preserved from CURRENT
    assert_eq!(config["portableSetting"], "from-current");
}

#[test]
fn test_switch_when_no_current_config_exists() {
    let env = TestEnv::new();

    // No .claude.json exists at all
    assert!(!env.claude_config_path().exists());

    // Create target profile
    env.create_profile("target", &sample_account("target"));

    // Switch should work — creates config from scratch with profile fields
    let _ = env.cmd().arg("target").assert();

    // Should be a regular file (not a symlink)
    assert!(
        !env.claude_config_path().is_symlink(),
        "Should create a regular file, not a symlink"
    );
    assert!(env.claude_config_path().exists());

    // Content should have the target account
    let config = env.read_claude_config();
    assert_eq!(config["oauthAccount"]["accountUuid"], "uuid-target");
}

#[test]
fn test_switch_does_not_modify_profile_file() {
    let env = TestEnv::new();

    let account = sample_account("current");
    env.create_claude_config(&account);

    // Create target profile with specific content
    fs::create_dir_all(env.claudectx_dir()).expect("mkdir");
    let target_profile = json!({
        "oauthAccount": sample_account("target"),
        "userID": "target-user-id"
    });
    let profile_json = serde_json::to_string_pretty(&target_profile).expect("serialize");
    fs::write(env.profile_path("target"), &profile_json).expect("write");

    // Switch to target
    let _ = env.cmd().arg("target").assert();

    // Profile file should be unchanged
    let profile_after = fs::read_to_string(env.profile_path("target")).expect("read");
    assert_eq!(
        profile_after, profile_json,
        "Profile file content should not be modified by switch"
    );
}

#[test]
fn test_switch_removes_stale_account_fields() {
    let env = TestEnv::new();

    // Current config has groveConfigCache and s1mAccessCache
    let current_config = json!({
        "oauthAccount": sample_account("current"),
        "userID": "current-user",
        "groveConfigCache": {"stale": true},
        "s1mAccessCache": {"stale": "data"},
        "hasCompletedOnboarding": true
    });
    fs::write(
        env.claude_config_path(),
        serde_json::to_string_pretty(&current_config).expect("serialize"),
    )
    .expect("write");

    // Target profile has ONLY oauthAccount (no groveConfigCache, no s1mAccessCache, no userID)
    fs::create_dir_all(env.claudectx_dir()).expect("mkdir");
    let target_profile = json!({
        "oauthAccount": sample_account("target")
    });
    fs::write(
        env.profile_path("target"),
        serde_json::to_string_pretty(&target_profile).expect("serialize"),
    )
    .expect("write");

    // Switch to target
    let _ = env.cmd().arg("target").assert();

    // Read config
    let config = env.read_claude_config();

    // Account fields present in profile should be set
    assert_eq!(config["oauthAccount"]["accountUuid"], "uuid-target");

    // Account fields absent from profile should be REMOVED (not carried over)
    assert!(
        config.get("userID").is_none(),
        "userID should be removed since it's not in the target profile"
    );
    assert!(
        config.get("groveConfigCache").is_none(),
        "groveConfigCache should be removed since it's not in the target profile"
    );
    assert!(
        config.get("s1mAccessCache").is_none(),
        "s1mAccessCache should be removed since it's not in the target profile"
    );

    // Portable field should be preserved
    assert_eq!(config["hasCompletedOnboarding"], true);
}

// =============================================================================
// MIGRATION TESTS
// =============================================================================

#[test]
fn test_migration_resolves_symlink_and_converts_profiles() {
    let env = TestEnv::new();

    // Create a full (old-style) profile file
    fs::create_dir_all(env.claudectx_dir()).expect("mkdir");
    let old_profile = json!({
        "oauthAccount": sample_account("migrated"),
        "userID": "migrated-user",
        "hasCompletedOnboarding": true,
        "primaryApiKey": "sk-old-key",
        "customSetting": "old-value"
    });
    fs::write(
        env.profile_path("old-profile"),
        serde_json::to_string_pretty(&old_profile).expect("serialize"),
    )
    .expect("write");

    // Create symlink .claude.json -> old-profile (simulating old architecture)
    #[cfg(unix)]
    std::os::unix::fs::symlink(env.profile_path("old-profile"), env.claude_config_path())
        .expect("Failed to create symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(env.profile_path("old-profile"), env.claude_config_path())
        .expect("Failed to create symlink");

    assert!(env.claude_config_path().is_symlink());

    // Run any command — migration runs at startup
    env.cmd().arg("list").assert().success();

    // 1. .claude.json should now be a regular file (not a symlink)
    assert!(
        !env.claude_config_path().is_symlink(),
        ".claude.json should be a regular file after migration"
    );
    assert!(env.claude_config_path().exists());

    // 2. .claude.json should have the full content (read through the old symlink)
    let config = env.read_claude_config();
    assert_eq!(config["oauthAccount"]["accountUuid"], "uuid-migrated");
    assert_eq!(config["hasCompletedOnboarding"], true);
    assert_eq!(config["primaryApiKey"], "sk-old-key");

    // 3. Profile should now be slim (only account fields)
    let profile = env.read_profile("old-profile");
    let obj = profile.as_object().unwrap();
    assert_eq!(profile["oauthAccount"]["accountUuid"], "uuid-migrated");
    assert_eq!(profile["userID"], "migrated-user");
    assert!(
        obj.get("hasCompletedOnboarding").is_none(),
        "Portable field should be stripped from slim profile"
    );
    assert!(
        obj.get("primaryApiKey").is_none(),
        "Portable field should be stripped from slim profile"
    );
    assert!(
        obj.get("customSetting").is_none(),
        "Portable field should be stripped from slim profile"
    );

    // 4. Backup should exist
    let backup_path = env.profile_path("old-profile").with_extension("json.bak");
    assert!(
        backup_path.exists(),
        "Backup file should be created during migration"
    );

    // 5. Backup should contain the original full content
    let backup_content = fs::read_to_string(&backup_path).expect("read backup");
    let backup: serde_json::Value = serde_json::from_str(&backup_content).expect("parse backup");
    assert_eq!(backup["customSetting"], "old-value");
    assert_eq!(backup["hasCompletedOnboarding"], true);
}

#[test]
fn test_migration_prints_message() {
    let env = TestEnv::new();

    // Create old-style setup with symlink
    fs::create_dir_all(env.claudectx_dir()).expect("mkdir");
    let profile = json!({
        "oauthAccount": sample_account("msg-test"),
        "userID": "msg-user"
    });
    fs::write(
        env.profile_path("msg"),
        serde_json::to_string_pretty(&profile).expect("serialize"),
    )
    .expect("write");

    #[cfg(unix)]
    std::os::unix::fs::symlink(env.profile_path("msg"), env.claude_config_path())
        .expect("Failed to create symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(env.profile_path("msg"), env.claude_config_path())
        .expect("Failed to create symlink");

    // Run a command
    let output = env.cmd().arg("list").assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    assert!(
        stdout.contains("Migrated profiles to slim format"),
        "Migration should print an info message. Output:\n{}",
        stdout
    );
}

#[test]
fn test_migration_skipped_when_no_symlink() {
    let env = TestEnv::new();

    // Create regular file (not symlink) — should NOT trigger migration
    let account = sample_account("no-migration");
    env.create_claude_config(&account);

    // Create a profile
    env.create_profile("regular", &sample_account("regular"));

    // Run command
    let output = env.cmd().arg("list").assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // No migration message
    assert!(
        !stdout.contains("Migrated"),
        "Migration should NOT run when .claude.json is a regular file. Output:\n{}",
        stdout
    );

    // No .bak files should be created
    let bak_exists = fs::read_dir(env.claudectx_dir())
        .expect("read dir")
        .any(|e| {
            e.ok()
                .map(|e| e.file_name().to_string_lossy().ends_with(".bak"))
                .unwrap_or(false)
        });
    assert!(
        !bak_exists,
        "No .bak files should be created when migration is skipped"
    );
}

#[test]
fn test_migration_is_one_shot() {
    let env = TestEnv::new();

    // Create old-style setup with symlink
    fs::create_dir_all(env.claudectx_dir()).expect("mkdir");
    let profile = json!({
        "oauthAccount": sample_account("oneshot"),
        "userID": "oneshot-user",
        "portableSetting": "value"
    });
    fs::write(
        env.profile_path("oneshot"),
        serde_json::to_string_pretty(&profile).expect("serialize"),
    )
    .expect("write");

    #[cfg(unix)]
    std::os::unix::fs::symlink(env.profile_path("oneshot"), env.claude_config_path())
        .expect("Failed to create symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(env.profile_path("oneshot"), env.claude_config_path())
        .expect("Failed to create symlink");

    // First run — triggers migration
    let output1 = env.cmd().arg("list").assert().success();
    let stdout1 = String::from_utf8_lossy(&output1.get_output().stdout);
    assert!(stdout1.contains("Migrated"));

    // Second run — no migration (not a symlink anymore)
    let output2 = env.cmd().arg("list").assert().success();
    let stdout2 = String::from_utf8_lossy(&output2.get_output().stdout);
    assert!(
        !stdout2.contains("Migrated"),
        "Second run should NOT trigger migration. Output:\n{}",
        stdout2
    );
}
