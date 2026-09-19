//! `scan <dir>`: walk a folder for projects, and report what was found.
//!
//! The walk registers nothing. Bulk registration is durable and a terminal
//! offers no undo, so reviewing the list and importing it stay two steps, as
//! they are in the app.

use std::path::PathBuf;

use clap::Args;
use indexer_core::application::ImportSelection;
use indexer_core::domain::scan::{ScanMode, ScanReport, ScanRequest};

use super::{Outcome, TrackerKind};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct ScanArgs {
    /// The folder to scan inside — `~/code`, not a project's own directory.
    pub directory: PathBuf,

    /// How far below the folder to look. 1 visits only its children.
    #[arg(long, default_value_t = 1)]
    pub depth: u32,

    /// Look inside `node_modules`, `target`, `.venv` and dot-directories too.
    #[arg(long)]
    pub include_ignored: bool,

    /// Only look for these trackers. Defaults to every kind this build has.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,

    #[arg(long)]
    pub import: bool,
}

pub fn run(args: ScanArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let request = ScanRequest {
        scan_root: args.directory.to_string_lossy().into_owned(),
        mode: mode(args.depth),
        detectors: detectors(ctx.scan.detector_kinds(), &args.tracker),
        include_ignored: args.include_ignored,
    };

    let report = ctx.scan.scan(&request)?;
    if !args.import {
        return Ok(Outcome::Scanned {
            root: request.scan_root,
            report,
        });
    }

    let report = ctx.scan.import(&selections(&report))?;
    Ok(Outcome::Imported { report })
}

/// `--depth` as the walker's mode. `Quick` is its depth 1, and the name the
/// app uses for the `~/code` case where every child is a project.
pub(crate) fn mode(depth: u32) -> ScanMode {
    if depth <= 1 {
        ScanMode::Quick
    } else {
        ScanMode::Deep { depth }
    }
}

/// The kinds to scan for: those `wanted`, or all of `available` when the flag
/// was not given.
///
/// Matched against the list the detectors report rather than passed through as
/// the user typed them: a detector decides the spelling of its own kind
/// ("Git"), the scan compares kinds exactly, and `--tracker` is lowercase.
/// Filtering the real list keeps the two in step through any rename.
pub(crate) fn detectors(available: Vec<String>, wanted: &[TrackerKind]) -> Vec<String> {
    if wanted.is_empty() {
        return available;
    }
    available
        .into_iter()
        .filter(|kind| {
            wanted
                .iter()
                .any(|wanted| kind.eq_ignore_ascii_case(wanted.kind()))
        })
        .collect()
}

/// Every candidate, under the name the scan settled on.
///
/// All of them, because a terminal has no checkboxes: narrowing is what
/// `--depth` and `--tracker` are for. Re-importing a tracked directory is a
/// no-op in core, so running this twice is safe.
pub(crate) fn selections(report: &ScanReport) -> Vec<ImportSelection> {
    report
        .candidates
        .iter()
        .map(|candidate| ImportSelection {
            directory: candidate.directory.clone(),
            name: candidate.suggested_name.clone(),
        })
        .collect()
}
