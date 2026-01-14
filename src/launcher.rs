use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

/// Launch claude with the specified settings file
/// This function replaces the current process with claude (does not return)
pub fn launch_claude(settings_path: &Path, extra_args: &[String]) -> ! {
    let err = Command::new("claude")
        .arg("--settings")
        .arg(settings_path)
        .args(extra_args)
        .exec();

    panic!("Failed to launch claude: {}", err);
}
