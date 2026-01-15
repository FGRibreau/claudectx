use std::process::Command;

use crate::profiles::switch_to_profile;

/// Switch to profile (via symlink) and launch claude.
/// On Unix, this replaces the current process with claude.
/// On Windows, this spawns claude and waits for it to exit.
pub fn switch_and_launch_claude(profile_name: &str, extra_args: &[String]) -> ! {
    // First, switch the symlink to point to the profile
    switch_to_profile(profile_name);

    // Then launch claude (it will read from the symlinked ~/.claude.json)
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = Command::new("claude").args(extra_args).exec();

        panic!("Failed to launch claude: {}", err);
    }

    #[cfg(windows)]
    {
        let status = Command::new("claude")
            .args(extra_args)
            .status()
            .expect("Failed to launch claude");

        std::process::exit(status.code().unwrap_or(1));
    }
}
