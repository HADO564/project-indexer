use serde_json::Value;

/// Current schema version for a stored project's JSON.
///
/// Bump this and add a step in [`migrate`] whenever the stored shape needs
/// to change in a way `#[serde(default)]` on `Project` can't absorb on its
/// own (a rename, a type change, splitting/merging fields, etc).
pub const CURRENT_VERSION: u64 = 1;

/// Brings a stored project JSON value up to [`CURRENT_VERSION`].
///
/// Data written before this system existed has no `schema_version` field
/// at all, which reads as version `0` here — today's `Project` shape *is*
/// version 1, so there's nothing to upgrade for it yet, only the version
/// stamp. Every `save_project` call re-stamps the current version, so
/// stored data settles onto the latest version the next time it's saved
/// for any ordinary reason.
///
/// When a real schema change lands, branch on the existing version and
/// apply each step in order, e.g.:
///
/// ```ignore
/// let version = value.get("schema_version").and_then(Value::as_u64).unwrap_or(0);
/// if version < 2 {
///     value = v2_rename_client_field::migrate(value);
/// }
/// ```
pub fn migrate(mut value: Value) -> Value {
    value["schema_version"] = Value::from(CURRENT_VERSION);
    value
}
