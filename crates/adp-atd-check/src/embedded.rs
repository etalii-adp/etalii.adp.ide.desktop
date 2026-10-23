//! The schemas and the capability catalogue the checker reads, embedded at build time (ATD §13.6, §14.6).

include!(concat!(env!("OUT_DIR"), "/embedded.rs"));

/// The names of the catalogue's capabilities, with their versions.
pub fn catalogue() -> Vec<(String, String)> {
    let mut names: Vec<(String, String)> = CAPABILITIES
        .iter()
        .filter_map(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .filter_map(|contract| {
            Some((
                contract["name"].as_str()?.to_owned(),
                contract["version"].as_str().unwrap_or("0.0.0").to_owned(),
            ))
        })
        .collect();
    names.sort();
    names
}

/// A capability's contract, by name.
pub fn contract(name: &str) -> Option<serde_json::Value> {
    CAPABILITIES
        .iter()
        .filter_map(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .find(|contract| contract["name"] == name)
}
