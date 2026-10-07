//! Groups: `group list|create|edit|delete`, and `find`, which `edit --group`
//! shares. Putting a project in a group is not here — that is `edit --group`,
//! a field of the project like any other.

use anyhow::{anyhow, bail};
use clap::{ArgGroup, Args, Subcommand};
use indexer_core::domain::sorting::SortOptions;
use indexer_core::{Group, UpdateGroup};

use super::Outcome;
use crate::appearance;
use crate::context::Context;

#[derive(Debug, Args)]
pub struct GroupArgs {
    #[command(subcommand)]
    pub action: GroupAction,
}

#[derive(Debug, Subcommand)]
pub enum GroupAction {
    /// List the groups in the sidebar's order, with how many projects each has.
    List,
    /// Make a new group, last in the sidebar.
    Create {
        /// Its name. Two groups cannot share one, ignoring case.
        name: String,

        /// cyan, gold, amber, rust, violet, green, blue or pink, or #rrggbb.
        #[arg(long, default_value = "cyan")]
        color: String,

        /// A bundled icon: folder, briefcase, code, rocket, … Groups take the
        /// bundled ones only, as in the app.
        #[arg(long, default_value = "briefcase")]
        icon: String,
    },
    /// Rename a group, or change its colour or icon.
    #[command(group(ArgGroup::new("change").required(true).multiple(true)))]
    Edit {
        /// The group, by name, ignoring case.
        group: String,

        /// A new name.
        #[arg(long, group = "change")]
        name: Option<String>,

        /// A new colour: a palette name or #rrggbb.
        #[arg(long, group = "change")]
        color: Option<String>,

        /// A new icon: a bundled one.
        #[arg(long, group = "change")]
        icon: Option<String>,
    },
    /// Delete a group. Asks first. Its projects are kept, ungrouped.
    Delete {
        /// The group, by name, ignoring case.
        group: String,
    },
}

pub fn run(args: GroupArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    match args.action {
        GroupAction::List => list(ctx),
        GroupAction::Create { name, color, icon } => {
            let group = ctx
                .groups
                .create(name, group_color(&color)?, group_icon(&icon)?)?;
            Ok(Outcome::GroupSaved {
                group: Box::new(group),
                created: true,
            })
        }
        GroupAction::Edit {
            group,
            name,
            color,
            icon,
        } => {
            let groups = ctx.groups.list()?;
            let id = find(&groups, &group)?.id.clone();
            let update = UpdateGroup {
                name,
                color: color.as_deref().map(group_color).transpose()?,
                icon: icon.as_deref().map(group_icon).transpose()?,
            };
            let group = ctx.groups.update(&id, update)?;
            Ok(Outcome::GroupSaved {
                group: Box::new(group),
                created: false,
            })
        }
        GroupAction::Delete { group } => {
            let groups = ctx.groups.list()?;
            let group = find(&groups, &group)?.clone();
            let members = members(ctx, &group)?;
            let prompt = match members {
                0 => format!("delete the group \"{}\"?", group.name),
                1 => format!(
                    "delete the group \"{}\"? its 1 project is kept, ungrouped",
                    group.name
                ),
                n => format!(
                    "delete the group \"{}\"? its {n} projects are kept, ungrouped",
                    group.name
                ),
            };
            if !ctx.confirmer.confirm(&prompt)? {
                return Ok(Outcome::Cancelled);
            }
            ctx.groups.delete(&group.id)?;
            Ok(Outcome::GroupDeleted {
                group: Box::new(group),
                members,
            })
        }
    }
}

/// Every group, each with how many projects are in it — live ones only, as
/// the sidebar counts them.
fn list(ctx: &Context) -> anyhow::Result<Outcome> {
    let projects = ctx.projects.list(SortOptions::default())?;
    let groups = ctx
        .groups
        .list()?
        .into_iter()
        .map(|group| {
            let count = projects
                .iter()
                .filter(|p| p.group_id.as_deref() == Some(group.id.as_str()))
                .count();
            (group, count)
        })
        .collect();
    Ok(Outcome::Groups { groups })
}

/// How many projects deleting `group` leaves ungrouped: binned ones too, as
/// core ungroups every member.
fn members(ctx: &Context, group: &Group) -> anyhow::Result<usize> {
    let live = ctx.projects.list(SortOptions::default())?;
    let binned = ctx.projects.list_deleted(SortOptions::default())?;
    Ok(live
        .iter()
        .chain(&binned)
        .filter(|p| p.group_id.as_deref() == Some(group.id.as_str()))
        .count())
}

/// A group's colour: as a project's, except that a group always has one.
fn group_color(typed: &str) -> anyhow::Result<String> {
    appearance::color(typed)?.ok_or_else(|| anyhow!("a group needs a colour"))
}

/// A group's icon: a bundled one only, as the app's group manager offers.
fn group_icon(typed: &str) -> anyhow::Result<String> {
    if appearance::is_custom(typed) {
        bail!("a group's icon is a bundled one, as in the app; custom icons are for projects");
    }
    appearance::icon(typed, &[])?.ok_or_else(|| anyhow!("a group needs an icon"))
}

/// The group named `typed`: trimmed and ignoring case, as core compares names
/// when it refuses a duplicate, so the two can never disagree about which
/// group a name means. An exact match only: `--group or` must not find "Work".
pub fn find<'a>(groups: &'a [Group], typed: &str) -> anyhow::Result<&'a Group> {
    let typed = typed.trim();
    groups
        .iter()
        .find(|g| g.name.trim().eq_ignore_ascii_case(typed))
        .ok_or_else(|| {
            if groups.is_empty() {
                anyhow!("no group named \"{typed}\": there are no groups yet")
            } else {
                let names: Vec<&str> = groups.iter().map(|g| g.name.as_str()).collect();
                anyhow!("no group named \"{typed}\". Groups: {}", names.join(", "))
            }
        })
}
