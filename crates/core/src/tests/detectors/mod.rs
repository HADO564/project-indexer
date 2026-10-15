//! Test modules, mirroring the source tree.
//!
//! Kept out of the source files so each module reads as code, and in
//! `src/` rather than `crates/core/tests/` so they stay unit tests with
//! access to `pub(crate)` internals — an integration test could only
//! reach the public API, which would mean widening it to suit tests.

mod cmake;
mod dart;
mod docker;
mod dotnet;
mod elixir;
mod git;
mod go;
mod godot;
mod gradle;
mod maven;
mod node;
mod php;
mod python;
mod ruby;
mod runner;
mod rust;
mod swift;
mod unity;
mod unreal;
