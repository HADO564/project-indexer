use crate::error::LauncherError;

pub trait AppLauncher: Send + Sync {
    /// Open `directory`, with `open_with` if given, else the OS default.
    fn open(&self, directory: &str, open_with: Option<&str>) -> Result<(), LauncherError>;
    /// Whether `open_with` names an app that can currently be launched.
    fn is_available(&self, open_with: &str) -> bool;
}
