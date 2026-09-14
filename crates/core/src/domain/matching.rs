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
    let matches: Vec<&Project> = projects
        .iter()
        .filter(|p| p.name.to_lowercase().contains(&query))
        .collect();
    match matches.len() {
        0 => Resolution::NotFound,
        1 => Resolution::Found(matches[0]),
        _ => Resolution::Ambiguous(matches),
    }
}
