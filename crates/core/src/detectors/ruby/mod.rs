//! Everything the Ruby (Bundler) detector is: the `Detector` impl and the info
//! struct it produces. No error type of its own — a `Gemfile` whose `ruby`
//! directive can't be read degrades to a detected project with no version
//! rather than failing, so nothing here raises anything beyond the shared
//! `DetectorError::Io`.

pub mod detector;
pub mod info;

pub use detector::RubyDetector;
pub use info::RubyInfo;
