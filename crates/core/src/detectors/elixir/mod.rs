//! Everything the Elixir detector is: the `Detector` impl and the info struct
//! it produces. No error type of its own — a `mix.exs` whose `app:` can't be
//! read degrades to a detected project with no app name rather than failing,
//! so nothing here raises anything beyond the shared `DetectorError::Io`.

pub mod detector;
pub mod info;

pub use detector::ElixirDetector;
pub use info::ElixirInfo;
