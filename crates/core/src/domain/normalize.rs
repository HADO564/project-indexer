/// Removes spaces from a name, replacing them with underscores.
pub fn remove_spaces(name: &str) -> String {
    name.replace(' ', "_")
}

/// Trims a tag and title-cases it (first character uppercase, the rest
/// lowercase).
pub fn normalize_tag(tag: &str) -> String {
    let trimmed = tag.trim();
    let mut chars = trimmed.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}

/// Normalizes each tag with [`normalize_tag`], dropping empty and duplicate
/// results while preserving first-seen order.
pub fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = normalize_tag(&tag);
        if !tag.is_empty() && !normalized.contains(&tag) {
            normalized.push(tag);
        }
    }
    normalized
}

/// Normalizes a directory path so equivalent paths compare equal, e.g.
/// `D:\Projects\Friction\` and `D:\Projects\Friction` (trailing separator)
/// or `D:/Projects/Friction` (mixed slash style) all collapse to the same
/// string. Drive roots (`C:\`) are left intact rather than being stripped
/// down to `C:`, which would change their meaning.
///
/// This does not normalize case, so `C:\Foo` and `c:\foo` are still
/// treated as different directories.
pub fn normalize_directory(directory: &str) -> String {
    let trimmed = directory.trim();
    let normalized_seps: String = trimmed
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' {
                std::path::MAIN_SEPARATOR
            } else {
                c
            }
        })
        .collect();

    let stripped = normalized_seps.trim_end_matches(std::path::MAIN_SEPARATOR);

    if stripped.is_empty() {
        normalized_seps
    } else if stripped.ends_with(':') {
        format!("{stripped}{}", std::path::MAIN_SEPARATOR)
    } else {
        stripped.to_string()
    }
}
