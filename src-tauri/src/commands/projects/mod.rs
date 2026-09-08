//! Project commands, grouped the same way `ProjectService` is, so a command
//! and the method behind it live in files of the same name.

pub mod bin;
pub mod crud;
pub mod detection;
pub mod directory;
pub mod launching;
pub mod queries;

pub use bin::*;
pub use crud::*;
pub use detection::*;
pub use directory::*;
pub use launching::*;
pub use queries::*;
