use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::IconError;
use crate::icons::sanitize_svg;

/// One custom icon, as the frontend consumes it.
///
/// `svg` is the **sanitized** source, not a data URI: the frontend builds
/// `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}` itself, which
/// saves core a base64 dependency for a string the browser assembles in one
/// expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredIcon {
    pub name: String,
    pub svg: String,
}

/// A directory of user-supplied icons, sanitized on the way in.
///
/// Knows nothing about Tauri — `src-tauri` resolves the config directory and
/// passes the path in, which is what keeps this in `core`.
pub struct IconStore {
    dir: PathBuf,
}

impl IconStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Reads `source`, sanitizes it, and writes the result under a name derived
    /// from the file stem. **Nothing unsanitized is ever written**, so a
    /// rejected icon leaves the store untouched.
    pub fn import(&self, source: &Path) -> Result<StoredIcon, IconError> {
        let raw = std::fs::read_to_string(source)
            .map_err(|e| IconError::Io(format!("{}: {e}", source.display())))?;
        let svg = sanitize_svg(&raw)?;

        let stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let name = self.unique_name(&slugify(&stem))?;

        // This read-then-write is safe only because Tauri's ExecutionContext::Blocking
        // serializes non-async commands: `unique_name` lists the directory to pick an
        // unused name, then we write the file. If commands were concurrent, another
        // import could choose the same name, silently overwriting the first icon.
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| IconError::Io(format!("{}: {e}", self.dir.display())))?;
        let path = self.dir.join(format!("{name}.svg"));
        std::fs::write(&path, &svg)
            .map_err(|e| IconError::Io(format!("{}: {e}", path.display())))?;

        Ok(StoredIcon { name, svg })
    }

    /// Every stored icon. A store that has never been written to is empty
    /// rather than an error — the directory is created lazily on first import.
    /// A junk entry — a directory, a non-UTF-8 file, a name outside the
    /// slugified alphabet — is skipped rather than failing the whole listing,
    /// since a single bad file must not disable both `list` and `import`
    /// (which calls `list` via `unique_name`) with no way to recover from the
    /// UI. Only a failure to read the store directory itself is fatal.
    pub fn list(&self) -> Result<Vec<StoredIcon>, IconError> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let entries = std::fs::read_dir(&self.dir)
            .map_err(|e| IconError::Io(format!("{}: {e}", self.dir.display())))?;

        let mut out = Vec::new();
        for entry in entries {
            // A single bad entry — a permission error mid-iteration, a
            // directory or an unreadable file masquerading as an icon —
            // must not take the whole listing down with it. Skip it and keep
            // going; only a failure to open the store directory itself
            // (above) is treated as fatal, mirroring the lazy-directory rule
            // that a store degrades rather than errors.
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("svg") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            // Filters `list` through the same alphabet `delete` requires, so
            // the two ends always agree: a name `delete` would refuse is
            // never offered to the frontend as one it can act on.
            let Ok(name) = safe_name(stem) else {
                continue;
            };
            let Ok(svg) = std::fs::read_to_string(&path) else {
                continue;
            };
            out.push(StoredIcon { name, svg });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub fn delete(&self, name: &str) -> Result<(), IconError> {
        let path = self.dir.join(format!("{}.svg", safe_name(name)?));
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(IconError::Io(format!("{}: {e}", path.display()))),
        }
    }

    /// Appends `-2`, `-3`, … rather than overwriting. Importing a second
    /// `logo.svg` from a different folder must not silently replace the first.
    fn unique_name(&self, base: &str) -> Result<String, IconError> {
        let taken: Vec<String> = self.list()?.into_iter().map(|i| i.name).collect();
        if !taken.contains(&base.to_string()) {
            return Ok(base.to_string());
        }
        for n in 2..1000 {
            let candidate = format!("{base}-{n}");
            if !taken.contains(&candidate) {
                return Ok(candidate);
            }
        }
        Err(IconError::Io(format!("too many icons named {base}")))
    }
}

/// Lowercase, ASCII alphanumerics and hyphens only. Everything else collapses
/// to a hyphen, so a name can never carry a separator, a `..`, or anything else
/// that would let it address a file outside the store.
fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_hyphen = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_hyphen = false;
        } else if !last_hyphen && !out.is_empty() {
            out.push('-');
            last_hyphen = true;
        }
    }
    let trimmed = out.trim_end_matches('-').to_string();
    if trimmed.is_empty() {
        "icon".to_string()
    } else {
        trimmed
    }
}

/// Guards the one path where a name arrives from outside rather than being
/// produced by `slugify`. Path traversal is the risk: the store sits next to
/// `projects.db`.
fn safe_name(name: &str) -> Result<String, IconError> {
    let ok = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(name.to_string())
    } else {
        Err(IconError::Io(format!("not a valid icon name: {name:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"<svg viewBox="0 0 24 24"><path d="M3 6h18"/></svg>"#;

    fn store() -> (tempfile::TempDir, IconStore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = IconStore::new(dir.path().join("icons"));
        (dir, store)
    }

    fn source(dir: &std::path::Path, file: &str, body: &str) -> std::path::PathBuf {
        let path = dir.join(file);
        std::fs::write(&path, body).expect("write source");
        path
    }

    #[test]
    fn imports_sanitizes_and_lists_an_icon() {
        let (dir, store) = store();
        let src = source(dir.path(), "My Logo.svg", GOOD);

        let imported = store.import(&src).expect("import");
        assert_eq!(imported.name, "my-logo");
        assert!(imported.svg.contains("M3 6h18"));

        let listed = store.list().expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "my-logo");
    }

    #[test]
    fn import_rejects_a_hostile_svg_rather_than_storing_it() {
        let (dir, store) = store();
        let src = source(
            dir.path(),
            "bad.svg",
            r#"<svg viewBox="0 0 1 1"><script>alert(1)</script></svg>"#,
        );

        assert!(store.import(&src).is_err());
        assert!(
            store.list().expect("list").is_empty(),
            "nothing hostile may reach the store"
        );
    }

    #[test]
    fn import_stores_the_sanitized_form_not_the_original() {
        let (dir, store) = store();
        let src = source(
            dir.path(),
            "mixed.svg",
            r#"<svg viewBox="0 0 1 1"><script>alert(1)</script><path d="M0 0"/></svg>"#,
        );

        store.import(&src).expect("import");
        let stored = &store.list().expect("list")[0];
        assert!(!stored.svg.contains("script"));
        assert!(!stored.svg.contains("alert"));
    }

    #[test]
    fn a_second_icon_with_the_same_name_does_not_overwrite_the_first() {
        let (dir, store) = store();
        let a = source(dir.path(), "logo.svg", GOOD);
        std::fs::create_dir_all(dir.path().join("other")).expect("subdir");
        let b = source(&dir.path().join("other"), "logo.svg", GOOD);

        let first = store.import(&a).expect("first");
        let second = store.import(&b).expect("second");

        assert_eq!(first.name, "logo");
        assert_eq!(second.name, "logo-2");
        assert_eq!(store.list().expect("list").len(), 2);
    }

    #[test]
    fn deletes_an_icon() {
        let (dir, store) = store();
        let src = source(dir.path(), "logo.svg", GOOD);
        store.import(&src).expect("import");

        store.delete("logo").expect("delete");

        assert!(store.list().expect("list").is_empty());
    }

    #[test]
    fn delete_refuses_a_name_that_could_escape_the_store() {
        let (_dir, store) = store();
        assert!(store.delete("../projects.db").is_err());
        assert!(store.delete("nested/name").is_err());
        assert!(store.delete("..").is_err());
        // The native Windows separator, distinct from the forward-slash case
        // above: `dir.join("nested\\name.svg")` would otherwise reach into a
        // subdirectory just as readily as `/` does.
        assert!(store.delete("nested\\name").is_err());
        // An absolute, drive-rooted path: `PathBuf::join` replaces the whole
        // base when the joined component is itself absolute, so this is the
        // most direct way a name could address a file outside the store.
        assert!(store.delete("c:\\windows\\system32\\config").is_err());
    }

    #[test]
    fn listing_an_absent_directory_is_empty_not_an_error() {
        let (_dir, store) = store();
        assert!(store
            .list()
            .expect("list must tolerate a store never written to")
            .is_empty());
    }

    #[test]
    fn list_skips_a_directory_masquerading_as_an_icon_and_import_still_works() {
        let (dir, store) = store();
        let good = source(dir.path(), "logo.svg", GOOD);
        store.import(&good).expect("import the real icon");

        // A directory literally named `trap.svg` — same extension, same
        // apparent stem, but `read_to_string` on it fails ("Access is
        // denied" on Windows). It must be skipped, not propagated.
        let store_dir = dir.path().join("icons");
        std::fs::create_dir_all(store_dir.join("trap.svg")).expect("plant a trap directory");

        let listed = store.list().expect("list must skip the trap, not fail");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "logo");

        // `import` calls `list` via `unique_name`, so the trap must not
        // disable importing either.
        let other = source(dir.path(), "other.svg", GOOD);
        let imported = store
            .import(&other)
            .expect("import must not be disabled by a bad entry elsewhere in the store");
        assert_eq!(imported.name, "other");
    }

    #[test]
    fn list_skips_a_non_utf8_svg_file_and_import_still_works() {
        let (dir, store) = store();
        let good = source(dir.path(), "logo.svg", GOOD);
        store.import(&good).expect("import the real icon");

        // A renamed PNG or a truncated write: valid filename, invalid UTF-8
        // contents. Must be skipped, not propagated.
        let store_dir = dir.path().join("icons");
        std::fs::write(store_dir.join("trap.svg"), [0xFF, 0xFE, 0xFD])
            .expect("plant a non-utf8 file");

        let listed = store
            .list()
            .expect("list must skip invalid utf-8, not fail");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "logo");

        let other = source(dir.path(), "other.svg", GOOD);
        let imported = store
            .import(&other)
            .expect("import must not be disabled by a bad entry elsewhere in the store");
        assert_eq!(imported.name, "other");
    }

    #[test]
    fn list_does_not_offer_a_name_that_delete_would_refuse() {
        let (dir, store) = store();
        let store_dir = dir.path().join("icons");
        std::fs::create_dir_all(&store_dir).expect("create store dir");
        // Written directly to disk, bypassing `slugify` — the only way a
        // non-slug stem can land in the store.
        std::fs::write(store_dir.join("Weird Name.svg"), GOOD).expect("plant a non-slug stem");

        let listed = store.list().expect("list");
        assert!(
            listed.is_empty(),
            "a name delete would refuse must not be listed either"
        );
    }
}
