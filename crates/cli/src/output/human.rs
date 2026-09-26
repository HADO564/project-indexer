//! The default, human-readable rendering.

use std::io::{IsTerminal, Write};
use std::path::Path;

use indexer_core::domain::matching::SHORT_ID_LEN;
use indexer_core::domain::scan::Candidate;
use indexer_core::domain::Project;
use indexer_core::Tracker;

use crate::commands::{Outcome, TrackerKind, View};
use crate::output::color::Color;
use crate::output::Colors;

pub fn write(out: &mut impl Write, outcome: &Outcome, colors: Colors) -> anyhow::Result<()> {
    let terminal = std::io::stdout().is_terminal();
    let color = terminal && std::env::var_os("NO_COLOR").is_none();
    match outcome {
        Outcome::Projects {
            projects,
            query,
            tracker,
            view,
        } => {
            if projects.is_empty() {
                // stderr, so a script reading stdout sees no rows either way.
                //
                // A query explains the emptiness on its own; without one, the
                // view has to, or `--view binned` on an empty bin would claim
                // nothing is tracked at all.
                match query {
                    Some(query) => eprintln!("indexer: no projects match \"{query}\""),
                    None => match view {
                        View::All => eprintln!("indexer: no projects tracked yet"),
                        View::Favorites => eprintln!("indexer: no favourites yet"),
                        View::Binned => eprintln!("indexer: the bin is empty"),
                    },
                }
                return Ok(());
            }
            let projects: Vec<&Project> = projects.iter().collect();
            let style = TableStyle {
                folder_color: color.then_some(colors.folder),
                header_color: color.then_some(colors.header),
                width: if terminal { terminal_width() } else { None },
                tracker: tracker.clone(),
            };
            write!(out, "{}", project_table(&projects, &style))?;
        }
        Outcome::Project {
            project,
            tracker,
            group,
        } => {
            writeln!(out, "{}", project.name)?;
            writeln!(out, "  directory  {}", project.directory)?;
            writeln!(out, "  id         {}", project.id)?;
            if let Some(group) = group {
                writeln!(out, "  group      {}", group.name)?;
            }
            for kind in tracker {
                let details = kind_details(project, *kind);
                if details.is_empty() {
                    continue;
                }
                writeln!(out, "  {}", kind.kind())?;
                for (label, value) in details {
                    writeln!(out, "    {label:<9}{value}")?;
                }
            }
        }
        Outcome::Color { color: chosen, .. } => {
            let name = chosen.name();
            if color {
                writeln!(out, "{}", chosen.paint(&name))?;
            } else {
                writeln!(out, "{name}")?;
            }
        }
        // These four report an action rather than return data, so their human
        // output is prose on stderr and stdout stays empty — the same split
        // `Imported` uses below. A script that wants the project itself asks
        // for `--json`.
        Outcome::Added {
            project,
            already_tracked,
        } => {
            if *already_tracked {
                eprintln!(
                    "indexer: already tracking \"{}\" at {}",
                    project.name, project.directory
                );
            } else {
                eprintln!(
                    "indexer: tracking \"{}\" at {}",
                    project.name, project.directory
                );
            }
        }
        Outcome::Untracked { project } => {
            eprintln!(
                "indexer: stopped tracking \"{}\" — {} is untouched",
                project.name, project.directory
            );
        }
        Outcome::Opened { project } => {
            eprintln!("indexer: opening \"{}\"", project.name);
        }
        Outcome::Favorited { project, favorite } => {
            if *favorite {
                eprintln!("indexer: \"{}\" is now a favourite", project.name);
            } else {
                eprintln!("indexer: \"{}\" is no longer a favourite", project.name);
            }
        }
        Outcome::Cancelled => eprintln!("indexer: cancelled"),
        Outcome::Imported { report } => {
            // A summary, not a table: what matters is the counts and the rows
            // that failed. Prose, so it goes to stderr like every other
            // message that is not data.
            eprintln!(
                "indexer: registered {}, skipped {} already tracked",
                report.imported.len(),
                report.skipped
            );
            for failure in &report.failures {
                // Listed, never fatal: one bad directory must not cost the run.
                eprintln!("indexer: failed {}: {}", failure.directory, failure.message);
            }
        }
        Outcome::Scanned { root, report } => {
            if report.candidates.is_empty() {
                eprintln!("indexer: no projects found under {root}");
            } else {
                let style = TableStyle {
                    folder_color: color.then_some(colors.folder),
                    header_color: color.then_some(colors.header),
                    width: if terminal { terminal_width() } else { None },
                    tracker: Vec::new(),
                };
                write!(out, "{}", candidate_table(&report.candidates, root, &style))?;
            }
            // Prose on stderr, so `indexer scan … | wc -l` counts rows only.
            let tracked = report
                .candidates
                .iter()
                .filter(|c| c.already_tracked)
                .count();
            eprintln!(
                "indexer: {} found under {root}, {tracked} already tracked, {} directories visited",
                report.candidates.len(),
                report.visited
            );
            if !report.candidates.is_empty() {
                eprintln!("indexer: re-run with --import to register them:");
                eprintln!("  indexer scan {root} --import");
            }
            if report.stopped_early {
                // Never silent: an incomplete walk must not read as an empty disk.
                eprintln!(
                    "indexer: stopped at the {}-directory limit — results are incomplete",
                    indexer_core::domain::scan::MAX_DIRECTORIES
                );
            }
        }
    }
    Ok(())
}

fn terminal_width() -> Option<usize> {
    terminal_size::terminal_size().map(|(terminal_size::Width(width), _)| usize::from(width))
}

/// How [`project_table`] is drawn. The default is plain — no colour and no
/// terminal width, so the table is only as wide as its cells and not centred —
/// which suits anything that isn't a terminal, like an error message or a pipe.
#[derive(Debug, Default, Clone)]
pub struct TableStyle {
    pub folder_color: Option<Color>,
    pub header_color: Option<Color>,
    /// The terminal's width in columns, when known.
    pub width: Option<usize>,
    /// The tracker kinds whose columns the table shows, in order; empty for
    /// the default TRACKERS column.
    pub tracker: Vec<TrackerKind>,
}

/// With a known terminal width, a narrower table grows to this share of it.
const MIN_WIDTH_PERCENT: usize = 70;

/// The column headers for `trackers`: NAME, DIRECTORY and ID, then each
/// kind's own columns in the order asked for, then LAST OPENED.
///
/// Without `--tracker` the middle is TRACKERS, every kind the project has.
/// With one or more, the kinds are already known, so the space goes to those
/// trackers' own detail instead.
fn headers(trackers: &[TrackerKind]) -> Vec<&'static str> {
    let mut headers = vec!["NAME", "DIRECTORY", "ID"];
    if trackers.is_empty() {
        headers.push("TRACKERS");
    }
    for kind in trackers {
        headers.extend(kind_headers(*kind));
    }
    headers.push("LAST OPENED");
    headers
}

/// One kind's own columns. Paired with [`kind_cells`]: both must stay the
/// same length, or a row would not line up with its headers.
fn kind_headers(kind: TrackerKind) -> Vec<&'static str> {
    match kind {
        TrackerKind::Git => vec!["BRANCH", "CHANGES"],
        TrackerKind::Unreal => vec!["ENGINE"],
    }
}

/// One project's cells for the middle columns, matching [`headers`] for the
/// same trackers.
fn middle_cells(project: &Project, trackers: &[TrackerKind]) -> Vec<String> {
    if trackers.is_empty() {
        let kinds: Vec<&str> = project.trackers.iter().map(|t| t.kind()).collect();
        let joined = kinds.join(", ");
        return vec![if joined.is_empty() {
            "-".to_string()
        } else {
            joined
        }];
    }
    trackers
        .iter()
        .flat_map(|kind| kind_cells(project, *kind))
        .collect()
}

/// One kind's cells for one project, in the order [`kind_headers`] lists them.
///
/// A project without that tracker shows `-` in each of its columns: with one
/// kind `with_tracker` has already dropped those, but `--tracker git,unreal`
/// keeps a project carrying either, so the gaps are real and have to be
/// filled. The `match` over `Tracker` names every variant, so a new detector
/// fails the build here until someone decides what it shows.
fn kind_cells(project: &Project, kind: TrackerKind) -> Vec<String> {
    match kind {
        TrackerKind::Git => {
            let info = project.trackers.iter().find_map(|t| match t {
                Tracker::Git(git) => Some(git),
                Tracker::Unreal(_) => None,
            });
            match info {
                Some(info) => {
                    let branch = if info.detached_head {
                        "detached".to_string()
                    } else {
                        info.curr_branch.clone().unwrap_or_else(|| "-".to_string())
                    };
                    let changes = if info.dirty { "dirty" } else { "clean" };
                    vec![branch, changes.to_string()]
                }
                None => vec!["-".to_string(), "-".to_string()],
            }
        }
        TrackerKind::Unreal => {
            let info = project.trackers.iter().find_map(|t| match t {
                Tracker::Unreal(unreal) => Some(unreal),
                Tracker::Git(_) => None,
            });
            match info {
                Some(info) => vec![info
                    .engine_association
                    .clone()
                    .unwrap_or_else(|| "-".to_string())],
                None => vec!["-".to_string()],
            }
        }
    }
}

/// One kind's details for `show`, as label/value pairs — the same facts the
/// table's columns hold, with room for what does not fit a column.
///
/// Empty when the project has no tracker of that kind, so `show` prints no
/// section for it rather than a heading with nothing under it. The `match`
/// names every `Tracker` variant, as [`kind_cells`] does.
fn kind_details(project: &Project, kind: TrackerKind) -> Vec<(&'static str, String)> {
    match kind {
        TrackerKind::Git => {
            let Some(info) = project.trackers.iter().find_map(|t| match t {
                Tracker::Git(git) => Some(git),
                Tracker::Unreal(_) => None,
            }) else {
                return Vec::new();
            };
            let mut details = vec![(
                "branch",
                if info.detached_head {
                    "detached".to_string()
                } else {
                    info.curr_branch.clone().unwrap_or_else(|| "-".to_string())
                },
            )];
            details.push((
                "changes",
                if info.dirty { "dirty" } else { "clean" }.to_string(),
            ));
            if let Some(remote) = info.web_url.clone().or_else(|| info.repo_url.clone()) {
                details.push(("remote", remote));
            }
            details
        }
        TrackerKind::Unreal => {
            let Some(info) = project.trackers.iter().find_map(|t| match t {
                Tracker::Unreal(unreal) => Some(unreal),
                Tracker::Git(_) => None,
            }) else {
                return Vec::new();
            };
            let mut details = vec![("project", info.project_name.clone())];
            if let Some(engine) = info.engine_association.clone() {
                details.push(("engine", engine));
            }
            if let Some(vcs) = info.vcs_provider.clone() {
                details.push(("vcs", vcs));
            }
            details
        }
    }
}

/// The one column whose text is part-coloured: `parent/` plain, the folder in
/// the folder colour.
const DIRECTORY_COLUMN: usize = 1;

/// A bordered table of projects — `list`'s view, and `show`'s when a query
/// matches several. The columns come from [`headers`]: NAME, DIRECTORY, ID,
/// TRACKERS and LAST OPENED by default, or those trackers' own columns when
/// `style.tracker` names any.
///
/// DIRECTORY is only `parent/folder`, the part `show parent/folder` takes
/// back. Given a terminal width, the table grows to at least
/// [`MIN_WIDTH_PERCENT`] of it, sharing the extra space between the columns,
/// and is centred. Padding is always worked out on the plain text, so colour
/// codes never push a column out of line.
pub fn project_table(projects: &[&Project], style: &TableStyle) -> String {
    let rows: Vec<Row> = projects
        .iter()
        .map(|p| Row::of(p, &style.tracker))
        .collect();
    let cells: Vec<Vec<Cell>> = rows
        .iter()
        .map(|row| {
            // Only the folder is coloured, and only the plain text is measured,
            // so the colour codes never push a column out of line.
            row.cells()
                .into_iter()
                .enumerate()
                .map(|(column, text)| {
                    if column == DIRECTORY_COLUMN {
                        Cell {
                            shown: format!(
                                "{}{}",
                                row.parent,
                                paint(style.folder_color, &row.folder)
                            ),
                            plain: text,
                        }
                    } else {
                        Cell::plain(text)
                    }
                })
                .collect()
        })
        .collect();

    table(&headers(&style.tracker), cells, style)
}

/// A bordered table of scan candidates — directory, the name it would get,
/// what matched, and whether it is already tracked.
///
/// Directories are shown relative to `root`, the folder that was scanned:
/// absolute paths are mostly the same prefix repeated, and the summary line
/// names the root anyway.
pub fn candidate_table(candidates: &[Candidate], root: &str, style: &TableStyle) -> String {
    let rows: Vec<Vec<Cell>> = candidates
        .iter()
        .map(|candidate| {
            let relative = candidate
                .directory
                .strip_prefix(root)
                .map(|rest| rest.trim_start_matches(['/', '\\']))
                .filter(|rest| !rest.is_empty())
                .unwrap_or(&candidate.directory);
            let path = Path::new(relative);
            let folder = path
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| relative.to_string());
            let parent = relative
                .strip_suffix(&folder)
                .unwrap_or_default()
                .to_string();
            vec![
                Cell {
                    shown: format!("{}{}", parent, paint(style.folder_color, &folder)),
                    plain: format!("{parent}{folder}"),
                },
                // A renamed candidate is flagged: the scan qualified it to
                // avoid colliding with a name already in use.
                Cell::plain(if candidate.disambiguated {
                    format!("{} (renamed)", candidate.suggested_name)
                } else {
                    candidate.suggested_name.clone()
                }),
                Cell::plain(candidate.matched_kinds.join(", ")),
                Cell::plain(
                    if candidate.already_tracked {
                        "yes"
                    } else {
                        "-"
                    }
                    .to_string(),
                ),
            ]
        })
        .collect();

    table(&["DIRECTORY", "NAME", "KINDS", "TRACKED"], rows, style)
}

/// Draws `rows` under `headers`: one column per header, padded to the widest
/// cell, bordered, and — given a terminal width — stretched to
/// [`MIN_WIDTH_PERCENT`] of it and centred.
fn table(headers: &[&str], rows: Vec<Vec<Cell>>, style: &TableStyle) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|h| text_width(h)).collect();
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(text_width(&cell.plain));
        }
    }

    let mut indent = String::new();
    if let Some(terminal) = style.width {
        stretch(&mut widths, (terminal * MIN_WIDTH_PERCENT).div_ceil(100));
        let table = table_width(&widths);
        if terminal > table {
            indent = " ".repeat((terminal - table) / 2);
        }
    }

    let header: Vec<Cell> = headers
        .iter()
        .map(|h| Cell {
            plain: h.to_string(),
            shown: paint(style.header_color, h),
        })
        .collect();

    let mut lines = vec![
        rule(&widths, ['╭', '┬', '╮']),
        cells_line(&header, &widths),
        rule(&widths, ['├', '┼', '┤']),
    ];
    for row in &rows {
        lines.push(cells_line(row, &widths));
    }
    lines.push(rule(&widths, ['╰', '┴', '╯']));

    lines
        .iter()
        .map(|line| format!("{indent}{line}\n"))
        .collect()
}

/// One project's cells, as plain text.
struct Row {
    name: String,
    /// The parent folder with its trailing `/` (`"work/"`), or empty at the
    /// root of the filesystem.
    parent: String,
    folder: String,
    /// The project's id, cut to [`SHORT_ID_LEN`] — the shortest prefix
    /// `show` accepts, so a row can be copied from by id as well as by
    /// `parent/folder`.
    id: String,
    /// The columns between ID and LAST OPENED — one cell per header
    /// [`headers`] chose for this tracker.
    middle: Vec<String>,
    /// `YYYY-MM-DD`, or `never`.
    last_opened: String,
}

impl Row {
    fn of(project: &Project, tracker: &[TrackerKind]) -> Self {
        let path = Path::new(&project.directory);
        let parent = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|f| format!("{}/", f.to_string_lossy()))
            .unwrap_or_default();
        let folder = path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| project.directory.clone());

        // `chars`, not a byte slice: an id is hex today, but slicing a string
        // by bytes panics the moment one is not ASCII.
        let id = project.id.chars().take(SHORT_ID_LEN).collect();

        let last_opened = match project.last_opened_at {
            Some(date) => date.format("%Y-%m-%d").to_string(),
            None => "never".to_string(),
        };

        Row {
            name: project.name.clone(),
            parent,
            folder,
            id,
            middle: middle_cells(project, tracker),
            last_opened,
        }
    }

    /// The cells in column order, one per header.
    fn cells(&self) -> Vec<String> {
        let mut cells = vec![
            self.name.clone(),
            format!("{}{}", self.parent, self.folder),
            self.id.clone(),
        ];
        cells.extend(self.middle.iter().cloned());
        cells.push(self.last_opened.clone());
        cells
    }
}

/// A cell's text as measured (`plain`) and as printed (`shown`, which may
/// carry colour codes).
struct Cell {
    plain: String,
    shown: String,
}

impl Cell {
    fn plain(text: String) -> Self {
        Cell {
            shown: text.clone(),
            plain: text,
        }
    }
}

/// `│ a │ b │ … │`, each cell padded to its column's width.
fn cells_line(cells: &[Cell], widths: &[usize]) -> String {
    let mut line = String::from("│");
    for (cell, width) in cells.iter().zip(widths) {
        let padding = " ".repeat(width - text_width(&cell.plain));
        line.push_str(&format!(" {}{padding} │", cell.shown));
    }
    line
}

/// A horizontal border: `left`, a run of `─` over each column (and its two
/// spaces of padding), `middle` between columns, then `right`.
fn rule(widths: &[usize], [left, middle, right]: [char; 3]) -> String {
    let runs: Vec<String> = widths.iter().map(|w| "─".repeat(w + 2)).collect();
    format!("{left}{}{right}", runs.join(&middle.to_string()))
}

/// The table's printed width: every column, a space either side of each, and
/// one border character before, between and after them.
fn table_width(widths: &[usize]) -> usize {
    widths.iter().sum::<usize>() + 3 * widths.len() + 1
}

/// Widens the columns until the table is `target` wide, sharing the extra
/// evenly and giving any remainder to the leftmost columns. A table already
/// that wide is left alone.
fn stretch(widths: &mut [usize], target: usize) {
    let current = table_width(widths);
    if current >= target {
        return;
    }
    let extra = target - current;
    let columns = widths.len();
    for (i, width) in widths.iter_mut().enumerate() {
        *width += extra / columns + usize::from(i < extra % columns);
    }
}

fn paint(color: Option<Color>, text: &str) -> String {
    match color {
        Some(color) => color.paint(text),
        None => text.to_string(),
    }
}

/// Counted in characters rather than bytes, so a non-ASCII name lines up.
fn text_width(text: &str) -> usize {
    text.chars().count()
}
