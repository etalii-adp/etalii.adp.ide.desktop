//! The three-state matcher: declared dependencies against what actually sits in `charts/` (etalii.adp spec 030, task
//! T080).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart/
//! DependencyResolution.cs`. A dependency is resolved by the first entry whose name is its chart name or its alias,
//! ordinally; one nothing answers is an open end, never a finding; an entry nothing claims is undeclared. No version
//! is ever evaluated.

use std::collections::HashSet;

use crate::model::{DependencyDeclaration, DependencyResolution, ResolvedDependency, VendoredEntry};

/// Matches every declaration and accounts for every vendored entry.
pub fn matched(dependencies: &[DependencyDeclaration], vendored: &[VendoredEntry]) -> DependencyResolution {
    let mut claimed: HashSet<&str> = HashSet::new();
    let mut resolved = Vec::with_capacity(dependencies.len());
    for dependency in dependencies {
        let found = vendored.iter().find(|entry| {
            entry.entry_name == dependency.name
                || dependency
                    .alias
                    .as_deref()
                    .is_some_and(|alias| !alias.is_empty() && entry.entry_name == alias)
        });
        if let Some(entry) = found {
            claimed.insert(&entry.relative_path);
        }
        resolved.push(ResolvedDependency {
            dependency: dependency.clone(),
            vendored: found.cloned(),
        });
    }
    let mut undeclared: Vec<VendoredEntry> = vendored
        .iter()
        .filter(|entry| !claimed.contains(entry.relative_path.as_str()))
        .cloned()
        .collect();
    undeclared.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    DependencyResolution {
        dependencies: resolved,
        undeclared,
    }
}
