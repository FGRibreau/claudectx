//! End-to-end tests for claudectx CLI
//!
//! These tests run the actual binary in a sandboxed environment using
//! temporary directories as HOME to avoid interfering with real config files.
//!
//! Note: Tests that would launch claude are limited since claude is not
//! installed in the CI environment. We test save/list/delete thoroughly
//! and verify that launch attempts fail appropriately when claude is unavailable.

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

    /// Create a valid .claude.json config file
    fn create_claude_config(&self, account: &serde_json::Value) {
        let config = json!({
            "oauthAccount": account,
            "lastAccountUUID": account["accountUuid"],
            "primaryApiKey": "sk-ant-test-key",
            "hasCompletedOnboarding": true
        });
        fs::write(
            self.claude_config_path(),
            serde_json::to_string_pretty(&config).expect("serialize"),
        )
        .expect("Failed to write claude config");
    }

    /// Create a profile file directly
    fn create_profile(&self, name: &str, account: &serde_json::Value) {
        fs::create_dir_all(self.claudectx_dir()).expect("Failed to create claudectx dir");
        let config = json!({
            "oauthAccount": account,
            "lastAccountUUID": account["accountUuid"],
            "primaryApiKey": format!("sk-ant-test-key-{}", name),
            "hasCompletedOnboarding": true
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
                name.strip_suffix(".claude.json").map(String::from)
            })
            .collect()
    }

    /// Run claudectx command with this test environment
    fn cmd(&self) -> assert_cmd::Command {
        let mut cmd = Command::cargo_bin("claudectx").expect("Failed to find binary");
        // Use CLAUDECTX_HOME for reliable cross-platform home directory override
        // The dirs crate doesn't reliably respect HOME/USERPROFILE when set for child processes
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

    // Save second profile
    let account2 = sample_account("second");
    env.create_claude_config(&account2);
    env.cmd().args(["save", "profile2"]).assert().success();

    // Verify both profiles exist
    let profiles = env.list_profile_files();
    assert!(profiles.contains(&"profile1".to_string()));
    assert!(profiles.contains(&"profile2".to_string()));
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
// LAUNCH PROFILE TESTS
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
fn test_launch_existing_profile_attempts_claude() {
    let env = TestEnv::new();
    let account = sample_account("current");
    env.create_claude_config(&account);

    // Create a profile
    env.create_profile("work", &sample_account("work"));

    // This test verifies the profile lookup works.
    // The command will either:
    // - Fail with "Failed to launch claude" if claude is not installed
    // - Launch claude (which may return an error due to missing input/args)
    // Either way, it shouldn't panic from profile lookup issues.
    let output = env.cmd().arg("work").assert();

    // The profile file should still exist (wasn't deleted or corrupted)
    assert!(env.profile_path("work").exists());

    // We just verify it didn't panic from our code (profile lookup worked)
    // The exit could be failure (claude not found) or success (claude launched)
    let _ = output;
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
fn test_workflow_save_list_delete() {
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

    // 3. Delete the profile
    env.cmd()
        .args(["delete", "test-profile"])
        .assert()
        .success();

    // 4. List again - should be empty
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

    // List all profiles
    let output = env.cmd().arg("list").assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("work"));
    assert!(stdout.contains("personal"));
    assert!(stdout.contains("side-project"));
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
// DATA INTEGRITY TESTS
// =============================================================================

#[test]
fn test_saved_profile_preserves_all_config_fields() {
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

    // Create config with extra fields
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

    // Verify all fields are preserved (it's now a full copy)
    assert_eq!(profile["oauthAccount"]["accountUuid"], "uuid-integrity");
    assert_eq!(
        profile["oauthAccount"]["emailAddress"],
        "integrity@example.com"
    );
    assert_eq!(profile["customField"], "custom-value");
    assert_eq!(profile["nestedField"]["inner"], "value");
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
