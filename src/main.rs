mod config;
mod profiles;
mod ui;

use clap::{Parser, Subcommand};
use dialoguer::Confirm;

use config::{get_oauth_account, read_claude_config};
use profiles::{
    delete_profile, get_current_profile, get_profile_path, list_profiles, profile_exists,
    save_profile, slugify, switch_to_profile,
};
use ui::select_profile;

#[derive(Parser, Debug)]
#[command(author, version, about = "Switch Claude Code profiles via symlinks", long_about = None)]
struct Args {
    /// Profile name to switch to (interactive selection if omitted)
    profile: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all saved profiles
    List,

    /// Save current config as a new profile
    Save {
        /// Profile name
        name: String,
    },

    /// Delete a profile
    Delete {
        /// Profile name
        name: String,
    },
}

fn main() {
    let args = Args::parse();

    match args.command {
        None => {
            // Switch mode
            let profile_name = args.profile.unwrap_or_else(|| {
                // Interactive selection
                let profiles = list_profiles();

                if profiles.is_empty() {
                    let current_config = read_claude_config();
                    let current_account = get_oauth_account(&current_config);
                    println!(
                        "Current account: {} @ {}",
                        current_account.display_name, current_account.organization_name
                    );
                    println!(
                        "\nNo profiles saved yet. Use 'claudectx save <name>' to save this profile."
                    );
                    std::process::exit(0);
                }

                let current_profile = get_current_profile();

                select_profile(&profiles, current_profile.as_deref())
                    .expect("No profile selected")
            });

            let path = get_profile_path(&profile_name);

            if !path.exists() {
                // Profile doesn't exist - offer to create it
                let slug = slugify(&profile_name);
                let create = Confirm::new()
                    .with_prompt(format!(
                        "Profile '{}' not found. Save current config as this profile?",
                        slug
                    ))
                    .interact()
                    .expect("Failed to prompt");

                if create {
                    save_profile(&profile_name);
                    println!("Profile '{}' saved.", slug);
                } else {
                    panic!("Profile '{}' not found", slug);
                }
            }

            switch_to_profile(&profile_name);
            let slug = slugify(&profile_name);
            println!("Switched to profile '{}'", slug);
        }
        Some(Commands::List) => {
            let profiles = list_profiles();

            if profiles.is_empty() {
                println!("No profiles found.");
                return;
            }

            let current_profile = get_current_profile();

            for name in profiles {
                let path = get_profile_path(&name);
                let config: serde_json::Value = serde_json::from_str(
                    &std::fs::read_to_string(&path).expect("Failed to read profile"),
                )
                .expect("Failed to parse profile");

                let account = get_oauth_account(&config);
                let marker = if current_profile.as_ref() == Some(&name) {
                    " *"
                } else {
                    ""
                };
                println!(
                    "{} - {} @ {}{}",
                    name, account.display_name, account.organization_name, marker
                );
            }
        }
        Some(Commands::Save { name }) => {
            let slug = slugify(&name);

            if profile_exists(&name) {
                let overwrite = Confirm::new()
                    .with_prompt(format!("Profile '{}' already exists. Overwrite?", slug))
                    .interact()
                    .expect("Failed to prompt");

                if !overwrite {
                    println!("Cancelled.");
                    return;
                }
            }

            save_profile(&name);
            println!("Saved current config as '{}'", slug);
        }
        Some(Commands::Delete { name }) => {
            if !profile_exists(&name) {
                panic!("Profile '{}' not found", slugify(&name));
            }

            delete_profile(&name);
            println!("Deleted profile '{}'", slugify(&name));
        }
    }
}
