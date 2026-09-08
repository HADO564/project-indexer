//! Test modules, mirroring the source tree.
//!
//! Kept out of the source files so each module reads as code, and in
//! `src/` rather than `crates/core/tests/` so they stay unit tests with
//! access to `pub(crate)` internals — an integration test could only
//! reach the public API, which would mean widening it to suit tests.

mod group_service;
mod inspection;
mod scan_service;
mod service;
