use std::path::Path;
use std::process::Command;

/// Launch claude with the specified settings file.
/// On Unix, this replaces the current process with claude.
/// On Windows, this spawns claude and waits for it to exit.
pub fn launch_claude(settings_path: &Path, extra_args: &[String]) -> ! {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = Command::new("claude")
            .arg("--settings")
            .arg(settings_path)
            .args(extra_args)
            .exec();

        panic!("Failed to launch claude: {}", err);
    }

    #[cfg(windows)]
    {
        let status = Command::new("claude")
            .arg("--settings")
            .arg(settings_path)
            .args(extra_args)
            .status()
            .expect("Failed to launch claude");

        std::process::exit(status.code().unwrap_or(1));
    }
}
