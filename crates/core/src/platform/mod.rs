pub mod app_discovery;
pub mod app_launching;
#[cfg(target_os = "linux")]
pub mod desktop_entry;
pub mod filesystem;

pub use app_discovery::list_installed_apps;
pub use app_launching::open_with_app_available;
pub use filesystem::{check_directory_status, remove_directory, DirectoryStatus};
