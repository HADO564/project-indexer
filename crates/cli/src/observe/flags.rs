//! dexily's own flags on an observed command — `dexily --add-tag rust --group
//! Work mkdir thing` — which set the new project up in the same breath.
//!
//! They are `edit`'s field flags, with its names, rules and checks, and they
//! go **before** the command: everything from the command's name on belongs to
//! it untouched, so dexily never has to guess whose `--name` it is reading.
//!
//! Every one is checked before the command runs. A group that does not exist
//! or a colour that is not one stops everything, and the command never starts:
//! all or nothing, rather than a folder left behind with half a project.

use std::path::Path;

use anyhow::bail;
use clap::Args;
use indexer_core::{Project, UpdateProject};

use crate::appearance;
use crate::commands::edit::{
    cleared_if_blank, cleared_if_empty, edited_properties, edited_tags, parse_property,
};
use crate::commands::group;
use crate::context::Context;

const HEADING: &str = "For the project an observed command creates";

/// The flags as typed. Declared on the top-level command, so clap reads them
/// before the observed command's name and hands the rest over as it is.
#[derive(Debug, Default, Args)]
pub struct ObserveFlags {
    /// The new project's name, instead of its folder's. For a command that
    /// makes one folder.
    #[arg(long, help_heading = HEADING)]
    pub name: Option<String>,

    /// The new project's description.
    #[arg(long, help_heading = HEADING)]
    pub description: Option<String>,

    /// Tags for the new project. Repeat the flag or separate tags with commas.
    #[arg(long, value_delimiter = ',', help_heading = HEADING)]
    pub add_tag: Vec<String>,

    /// A property for the new project, as name=value. Repeat for more.
    #[arg(long, value_name = "KEY=VALUE", value_parser = parse_property, help_heading = HEADING)]
    pub set: Vec<(String, String)>,

    /// Notes for the new project.
    #[arg(long, help_heading = HEADING)]
    pub notes: Option<String>,

    /// The app to open the new project with.
    #[arg(long, help_heading = HEADING)]
    pub open_with: Option<String>,

    /// The group to put the new project in, matched ignoring case.
    #[arg(long, help_heading = HEADING)]
    pub group: Option<String>,

    /// The new project's colour: a palette name or #rrggbb.
    #[arg(long, help_heading = HEADING)]
    pub color: Option<String>,

    /// The new project's icon: a bundled one, or custom:<name>.
    #[arg(long, help_heading = HEADING)]
    pub icon: Option<String>,
}

impl ObserveFlags {
    /// Whether none was given: the bare `dexily mkdir thing`, which asks.
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.description.is_none()
            && self.add_tag.is_empty()
            && self.set.is_empty()
            && self.notes.is_none()
            && self.open_with.is_none()
            && self.group.is_none()
            && self.color.is_none()
            && self.icon.is_none()
    }

    /// Whether checking these needs the database — a group to look up, a name
    /// to find free — so that it is opened before the command only then. With
    /// neither, a broken database cannot stand between the user and `mkdir`.
    pub fn need_the_database(&self) -> bool {
        self.group.is_some() || self.name.is_some()
    }

    /// The flags checked, as the fields to set on each new project. `ctx` is
    /// the database, when [`ObserveFlags::need_the_database`] said to open it;
    /// `folders` is how many folders the command should make.
    pub fn check(
        self,
        ctx: Option<&Context>,
        cwd: &Path,
        folders: usize,
    ) -> anyhow::Result<Fields> {
        let color = self.color.as_deref().map(appearance::color).transpose()?;
        let icon = self
            .icon
            .as_deref()
            .map(appearance::checked_icon)
            .transpose()?;

        let mut group_id = None;
        if let Some(name) = &self.group {
            if name.trim().is_empty() {
                bail!("--group needs a group's name: a new project is in no group to begin with");
            }
            let ctx = ctx.expect("the database is opened when a group is named");
            group_id = Some(Some(group::find(&ctx.groups.list()?, name)?.id.clone()));
        }

        if let Some(name) = &self.name {
            if folders > 1 {
                bail!("--name names one project, but this command makes {folders} folders");
            }
            let ctx = ctx.expect("the database is opened when a name is given");
            let taken = ctx.projects.active_project_names()?;
            if taken
                .iter()
                .any(|t| t.trim().eq_ignore_ascii_case(name.trim()))
            {
                bail!("a project named \"{}\" already exists", name.trim());
            }
        }

        let fields = Fields {
            name: self.name,
            description: self.description,
            add_tags: self.add_tag,
            set: self.set,
            notes: cleared_if_empty(self.notes),
            open_with: cleared_if_blank(self.open_with),
            group_id,
            color,
            icon,
        };

        // Core's own rules — an empty name, a property name with `:` — by
        // letting core apply the update to a project that is never saved, so
        // no second copy of a rule can drift from the first. Not without
        // flags: there is nothing to check, and a working folder core would
        // not take as a project's must not stand in a bare command's way.
        if !fields.is_empty() {
            let mut scratch = Project::new(
                "dexily".into(),
                cwd.to_string_lossy().into_owned(),
                None,
                None,
            )?;
            let update = fields.update(&scratch);
            scratch.update(update)?;
        }
        Ok(fields)
    }
}

/// The flags once checked: what to set on each project the command creates.
#[derive(Debug, Default, Clone)]
pub struct Fields {
    name: Option<String>,
    description: Option<String>,
    add_tags: Vec<String>,
    set: Vec<(String, String)>,
    notes: Option<Option<String>>,
    open_with: Option<Option<String>>,
    group_id: Option<Option<String>>,
    color: Option<Option<String>>,
    icon: Option<Option<String>>,
}

impl Fields {
    /// Whether no flag was given, so there is nothing to set.
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.description.is_none()
            && self.add_tags.is_empty()
            && self.set.is_empty()
            && self.notes.is_none()
            && self.open_with.is_none()
            && self.group_id.is_none()
            && self.color.is_none()
            && self.icon.is_none()
    }

    /// One update for `project`, built as `edit` builds its own: tags and
    /// properties are *added* to what the project has — core replaces the
    /// whole list — so flags on a folder that was already tracked never cost
    /// it a tag, and property names match ignoring case, as `edit --set` does.
    pub fn update(&self, project: &Project) -> UpdateProject {
        let fields = self.clone();
        let tags =
            (!fields.add_tags.is_empty()).then(|| edited_tags(&project.tags, fields.add_tags, &[]));
        let properties = (!fields.set.is_empty())
            .then(|| edited_properties(&project.properties, fields.set, &[]));
        UpdateProject {
            name: fields.name,
            description: fields.description,
            tags,
            properties,
            notes: fields.notes,
            open_with: fields.open_with,
            group_id: fields.group_id,
            color: fields.color,
            icon: fields.icon,
            ..Default::default()
        }
    }
}
