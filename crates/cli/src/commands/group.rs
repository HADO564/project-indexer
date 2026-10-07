//! Groups as the command line names them. Only `find` for now, for
//! `edit --group`; `group create|edit|list|delete` will live here too.

use anyhow::anyhow;
use indexer_core::Group;

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
