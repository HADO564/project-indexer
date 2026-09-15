use crate::domain::Project;

pub enum Resolution<'a> {
    Found(&'a Project),
    NotFound,
    Ambiguous(Vec<&'a Project>),
}

pub fn resolve<'a>(projects: &'a [Project], query: &str) -> Resolution<'a> {
    let query = query.to_lowercase();
    if query.is_empty() {
        return Resolution::NotFound;
    }
    if let Some(project) = projects.iter().find(|p| p.name.to_lowercase() == query) {
        return Resolution::Found(project);
    }
    if query.contains('/') {
        let matches: Vec<&Project> = projects
            .iter()
            .filter(|p| path_matcher(p.directory.as_str(), &query))
            .collect();
        return pick(matches);
    }
    let mut matches: Vec<&Project> = projects
        .iter()
        .filter(|p| p.name.to_lowercase().contains(&query))
        .collect();
    matches.sort_by_key(|p| {
        let starts = if p.name.to_lowercase().starts_with(&query) {
            0
        } else {
            1
        };
        (starts, std::cmp::Reverse(p.last_opened_at))
    });
    pick(matches)
}

fn pick(matches: Vec<&Project>) -> Resolution<'_> {
    match matches.len() {
        0 => Resolution::NotFound,
        1 => Resolution::Found(matches[0]),
        _ => Resolution::Ambiguous(matches),
    }
}

fn path_matcher(directory: &str, query: &str) -> bool {
    let suffix = format!("/{query}");
    directory.to_lowercase().ends_with(&suffix)
}
