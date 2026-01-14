use std::path::Path;
use std::process::Command;

/// Launch claude with the specified settings file
/// On Unix: replaces the current process with claude (does not return)
/// On Windows: spawns claude and exits with its exit code
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
