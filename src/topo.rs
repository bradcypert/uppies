use crate::config::App;
use std::collections::HashMap;

/// Groups `apps` into dependency levels using Kahn's topological-sort algorithm.
///
/// Apps within the same level have no ordering dependency on each other and may
/// be processed concurrently. Levels must be processed in order: all apps in
/// level N must complete before any app in level N+1 begins.
///
/// Returns `Err` if a cycle is detected or if a `depends_on` entry names an
/// app that is not present in the list.
pub fn topological_levels(apps: Vec<App>) -> anyhow::Result<Vec<Vec<App>>> {
    if apps.is_empty() {
        return Ok(vec![]);
    }

    // Build name → index map and validate all depends_on references.
    let name_to_idx: HashMap<&str, usize> = apps
        .iter()
        .enumerate()
        .map(|(i, a)| (a.name.as_str(), i))
        .collect();

    for app in &apps {
        for dep in &app.depends_on {
            if !name_to_idx.contains_key(dep.as_str()) {
                anyhow::bail!(
                    "App '{}' depends on '{}' which is not registered",
                    app.name,
                    dep
                );
            }
        }
    }

    let n = apps.len();

    // Build adjacency list: graph[i] = list of indices that depend on i.
    // Also compute in-degree for each node.
    let mut graph: Vec<Vec<usize>> = vec![vec![]; n];
    let mut in_degree: Vec<usize> = vec![0; n];

    for (i, app) in apps.iter().enumerate() {
        for dep in &app.depends_on {
            let dep_idx = name_to_idx[dep.as_str()];
            // dep must complete before i → edge dep_idx → i
            graph[dep_idx].push(i);
            in_degree[i] += 1;
        }
    }

    // Kahn's algorithm: process nodes level by level.
    let mut apps_opt: Vec<Option<App>> = apps.into_iter().map(Some).collect();
    let mut levels: Vec<Vec<App>> = Vec::new();
    let mut current_level: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut processed = 0;

    while !current_level.is_empty() {
        // Drain the current level into a Vec<App>.
        let level_apps: Vec<App> = current_level
            .iter()
            .map(|&i| apps_opt[i].take().expect("node visited twice"))
            .collect();
        processed += level_apps.len();

        // Decrement in-degree for all dependents and collect next level.
        let mut next_level: Vec<usize> = Vec::new();
        for &idx in &current_level {
            for &dependent in &graph[idx] {
                in_degree[dependent] -= 1;
                if in_degree[dependent] == 0 {
                    next_level.push(dependent);
                }
            }
        }

        levels.push(level_apps);
        current_level = next_level;
    }

    if processed != n {
        anyhow::bail!("Circular dependency detected in app configuration");
    }

    Ok(levels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScriptConfig;
    use crate::version::CompareMode;

    fn make_app(name: &str, depends_on: Vec<&str>) -> App {
        App {
            name: name.to_string(),
            description: None,
            local: ScriptConfig::Inline {
                inline: "echo 1".to_string(),
            },
            remote: ScriptConfig::Inline {
                inline: "echo 1".to_string(),
            },
            update: ScriptConfig::Inline {
                inline: "true".to_string(),
            },
            compare_mode: CompareMode::String,
            tags: vec![],
            pinned: false,
            timeout_secs: None,
            depends_on: depends_on.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn test_no_deps_single_level() {
        let apps = vec![make_app("a", vec![]), make_app("b", vec![])];
        let levels = topological_levels(apps).unwrap();
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].len(), 2);
    }

    #[test]
    fn test_linear_chain() {
        // c depends on b, b depends on a → three levels
        let apps = vec![
            make_app("a", vec![]),
            make_app("b", vec!["a"]),
            make_app("c", vec!["b"]),
        ];
        let levels = topological_levels(apps).unwrap();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0][0].name, "a");
        assert_eq!(levels[1][0].name, "b");
        assert_eq!(levels[2][0].name, "c");
    }

    #[test]
    fn test_diamond_dependency() {
        // b and c both depend on a; d depends on b and c
        let apps = vec![
            make_app("a", vec![]),
            make_app("b", vec!["a"]),
            make_app("c", vec!["a"]),
            make_app("d", vec!["b", "c"]),
        ];
        let levels = topological_levels(apps).unwrap();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0].len(), 1); // a
        assert_eq!(levels[1].len(), 2); // b and c (parallel)
        assert_eq!(levels[2].len(), 1); // d
    }

    #[test]
    fn test_cycle_detected() {
        let apps = vec![make_app("a", vec!["b"]), make_app("b", vec!["a"])];
        assert!(topological_levels(apps).is_err());
    }

    #[test]
    fn test_unknown_dep_error() {
        let apps = vec![make_app("a", vec!["nonexistent"])];
        assert!(topological_levels(apps).is_err());
    }

    #[test]
    fn test_empty() {
        let levels = topological_levels(vec![]).unwrap();
        assert!(levels.is_empty());
    }
}
