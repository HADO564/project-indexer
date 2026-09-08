//! Test modules, mirroring the source tree.
//!
//! Kept out of the source files so each module reads as code, and in
//! `src/` rather than `crates/core/tests/` so they stay unit tests with
//! access to `pub(crate)` internals — an integration test could only
//! reach the public API, which would mean widening it to suit tests.

// Mirrors `detectors/unreal/unreal.rs`, so it inherits that module's shape —
// including the inception clippy flags there. Kept faithful on purpose: the
// point of this tree is that a test file sits at the same path as the code it
// covers, and renaming it here would break that for one directory.
#[allow(clippy::module_inception)]
mod unreal;
