use std::process::Command;

use dialoguer::{Confirm, Input};

use crate::config::get_oauth_account;
use crate::launcher::switch_and_launch_claude;
use crate::profiles::{
    backup_claude_config, claude_config_exists, list_profiles, profile_exists,
    restore_claude_config, save_profile, slugify,
};
use crate::ui::select_profile;

/// Run the login workflow:
/// 1. Backup existing ~/.claude.json (if any)
/// 2. Run `claude /login`
/// 3. Prompt for profile name
/// 4. Save new config as profile
/// 5. Restore original config (or clean up if none existed)
/// 6. Offer to launch with new profile or select another
pub fn run_login_workflow() {
    println!("Starting Claude login workflow...\n");

    // Step 1: Backup existing config
    let had_backup = backup_claude_config();
    if had_backup {
        println!("Backed up existing config to ~/.claude.json.bak");
    }

    // Step 2: Run claude /login
    println!("Launching Claude login...\n");
    let status = Command::new("claude")
        .arg("/login")
        .status()
        .expect("Failed to launch 'claude /login' - is Claude Code installed?");

    if !status.success() {
        eprintln!("\nClaude login failed or was cancelled.");
        restore_claude_config(had_backup);
        if had_backup {
            println!("Restored original config.");
        }
        panic!("Login process exited with status: {}", status);
    }

    // Check that login created a new config
    if !claude_config_exists() {
        eprintln!("\nNo config file created after login.");
        restore_claude_config(had_backup);
        if had_backup {
            println!("Restored original config.");
        }
        panic!("Login did not create a config file");
    }

    // Show the new account info
    let new_config = crate::config::read_claude_config();
    let new_account = get_oauth_account(&new_config);
    println!(
        "\nLogged in as: {} @ {}",
        new_account.display_name, new_account.organization_name
    );

    // Step 3: Prompt for profile name
    let profile_name: String = Input::new()
        .with_prompt("Enter a name for this profile")
        .interact_text()
        .expect("Failed to read profile name");

    let slug = slugify(&profile_name);

    // Check if profile exists and ask for confirmation
    if profile_exists(&profile_name) {
        let overwrite = Confirm::new()
            .with_prompt(format!("Profile '{}' already exists. Overwrite?", slug))
            .interact()
            .expect("Failed to prompt");

        if !overwrite {
            println!("Cancelled. Cleaning up...");
            restore_claude_config(had_backup);
            if had_backup {
                println!("Restored original config.");
            }
            return;
        }
    }

    // Step 4: Save new config as profile
    save_profile(&profile_name);
    println!("Saved profile '{}'", slug);

    // Step 5: Restore original config
    restore_claude_config(had_backup);
    if had_backup {
        println!("Restored original config.");
    } else {
        println!("Cleaned up temporary config.");
    }

    // Step 6: Offer to launch
    let launch_new = Confirm::new()
        .with_prompt(format!("Launch Claude with profile '{}'?", slug))
        .default(true)
        .interact()
        .expect("Failed to prompt");

    if launch_new {
        switch_and_launch_claude(&profile_name, &[]);
    }

    // If not launching the new profile, offer to select another
    let profiles = list_profiles();
    if !profiles.is_empty() {
        let select_other = Confirm::new()
            .with_prompt("Select a different profile to launch?")
            .default(false)
            .interact()
            .expect("Failed to prompt");

        if select_other {
            if let Some(selected) = select_profile(&profiles, Some(&slug)) {
                switch_and_launch_claude(&selected, &[]);
            }
        }
    }

    println!("\nDone. Use 'claudectx' to launch with any profile.");
}
