use serde::Deserialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::error::UnrealError;
use super::info::UnrealInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct UnrealDetector;

impl Detector for UnrealDetector {
    fn kind(&self) -> &'static str {
        "unreal"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let Some(uproject_path) = find_project_file(path)? else {
            return Ok(None);
        };

        let project_name = uproject_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_string();

        let descriptor = read_project_descriptor(&uproject_path)?;

        Ok(Some(Tracker::new(
            "Unreal",
            UnrealInfo {
                project_root: path.display().to_string(),
                project_name,
                uproject_path: uproject_path.display().to_string(),
                engine_association: non_empty(descriptor.engine_association.unwrap_or_default()),
                category: non_empty(descriptor.category),
                description: non_empty(descriptor.description),
                modules: descriptor.modules.into_iter().map(|m| m.name).collect(),
                plugins: descriptor
                    .plugins
                    .into_iter()
                    .filter(|p| p.enabled)
                    .map(|p| p.name)
                    .collect(),
                vcs_provider: vcs_provider(path)?,
            },
        )))
    }
}

/// Finds the `.uproject` file directly inside `path`, if any.
///
/// Unlike git, which can be discovered from any subdirectory of a work tree,
/// an Unreal project's `.uproject` file lives at the project root, so this
/// only looks at `path`'s immediate entries rather than walking up or down.
pub fn find_project_file(path: &Path) -> io::Result<Option<PathBuf>> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;

        if !entry.file_type()?.is_file() {
            continue;
        }

        let entry_path = entry.path();
        if entry_path.extension().and_then(|ext| ext.to_str()) == Some("uproject") {
            return Ok(Some(entry_path));
        }
    }

    Ok(None)
}

#[derive(Debug, Default, Deserialize)]
struct UprojectModule {
    #[serde(rename = "Name")]
    name: String,
}

#[derive(Debug, Default, Deserialize)]
struct UprojectPlugin {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Enabled", default)]
    enabled: bool,
}

/// The handful of `.uproject` fields this detector cares about. The real
/// file carries more (FileVersion, TargetPlatforms, ...) that we don't need
/// and silently ignore — `serde` skips unrecognized keys by default.
#[derive(Debug, Default, Deserialize)]
struct UprojectDescriptor {
    #[serde(rename = "EngineAssociation", default)]
    engine_association: Option<String>,
    #[serde(rename = "Category", default)]
    category: String,
    #[serde(rename = "Description", default)]
    description: String,
    #[serde(rename = "Modules", default)]
    modules: Vec<UprojectModule>,
    #[serde(rename = "Plugins", default)]
    plugins: Vec<UprojectPlugin>,
}

/// Reads and parses a `.uproject` file. Fields the descriptor doesn't
/// recognize are ignored rather than rejected — engine versions add fields
/// over time, and this detector only needs a handful of them.
fn read_project_descriptor(uproject_path: &Path) -> Result<UprojectDescriptor, DetectorError> {
    let contents = fs::read_to_string(uproject_path)?;
    serde_json::from_str(&contents).map_err(|e| DetectorError::from(UnrealError::ParseUproject(e)))
}

/// Collapses an empty string to `None`, matching how [`GitInfo::repo_url`]
/// treats an empty remote URL as "not configured" rather than a value.
fn non_empty(s: String) -> Option<String> {
    (!s.is_empty()).then_some(s)
}

/// The source-control provider configured for this project (e.g.
/// `"Perforce"`, `"Git"`), if one is.
///
/// Unreal stores this per-user, per-platform setting under
/// `Saved/Config/<Platform>Editor/SourceControlSettings.ini` — a file most
/// `.gitignore`s exclude from version control, so `None` on a freshly
/// cloned project is the common case, not a detection failure.
fn vcs_provider(project_root: &Path) -> Result<Option<String>, DetectorError> {
    let config_dir = project_root.join("Saved").join("Config");

    let entries = match fs::read_dir(&config_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let ini_path = entry.path().join("SourceControlSettings.ini");
        if let Some(provider) = read_provider_from_ini(&ini_path)? {
            return Ok(Some(provider));
        }
    }

    Ok(None)
}

/// Reads the `Provider=` value out of a `SourceControlSettings.ini`'s
/// `[SourceControl.SourceControlSettings]` section.
///
/// A minimal line-scan rather than a full ini parser — this is the one key
/// this detector needs, and pulling in an ini crate for it isn't worth it.
fn read_provider_from_ini(ini_path: &Path) -> Result<Option<String>, DetectorError> {
    let contents = match fs::read_to_string(ini_path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let mut in_section = false;
    for line in contents.lines() {
        let line = line.trim();

        if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            in_section = section == "SourceControl.SourceControlSettings";
            continue;
        }

        if !in_section {
            continue;
        }

        if let Some(value) = line.strip_prefix("Provider=") {
            let value = value.trim();
            // An explicitly configured "None" means the same thing here as
            // no file at all: no provider to report.
            return Ok((!value.is_empty() && value != "None").then(|| value.to_string()));
        }
    }

    Ok(None)
}
