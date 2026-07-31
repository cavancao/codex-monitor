use std::{
    collections::HashMap,
    env,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};
use walkdir::WalkDir;

#[derive(Clone, Debug)]
pub struct DiscoveryInputs {
    pub home: PathBuf,
    pub codex_home: Option<PathBuf>,
    pub system_roots: Vec<PathBuf>,
}

impl DiscoveryInputs {
    pub fn current() -> Result<Self, String> {
        let home = dirs::home_dir().ok_or("无法解析当前用户 home 目录")?;
        let codex_home = env::var_os("CODEX_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let mut system_roots = [
            dirs::config_dir(),
            dirs::data_dir(),
            dirs::data_local_dir(),
            dirs::cache_dir(),
        ]
        .into_iter()
        .flatten()
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
        system_roots.sort();
        system_roots.dedup();
        Ok(Self {
            home,
            codex_home,
            system_roots,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootEvidence {
    Sessions,
    ModelsCache,
    Auth,
    StateDatabase,
    RolloutLog,
}

#[derive(Clone, Debug)]
pub struct DataRootCandidate {
    pub path: PathBuf,
    pub score: u16,
    pub evidence: Vec<RootEvidence>,
}

#[derive(Debug)]
struct ScoredRoot {
    candidate: DataRootCandidate,
    explicit: bool,
    newest: SystemTime,
}

pub fn discover_roots(inputs: &DiscoveryInputs) -> Vec<DataRootCandidate> {
    let explicit = inputs
        .codex_home
        .as_deref()
        .and_then(|path| path.canonicalize().ok());
    let mut roots = Vec::new();
    if let Some(path) = inputs.codex_home.as_ref() {
        roots.push((path.clone(), 0));
    }
    roots.push((inputs.home.clone(), 2));
    roots.extend(inputs.system_roots.iter().cloned().map(|path| (path, 4)));

    let mut found: HashMap<PathBuf, ScoredRoot> = HashMap::new();
    for (root, depth) in roots {
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(root)
            .max_depth(depth)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_dir())
        {
            let Some((score, evidence, newest)) = inspect_root(entry.path()) else {
                continue;
            };
            let Ok(key) = entry.path().canonicalize() else {
                continue;
            };
            let is_explicit = explicit.as_ref().is_some_and(|value| value == &key);
            let scored = ScoredRoot {
                candidate: DataRootCandidate {
                    path: entry.path().to_path_buf(),
                    score,
                    evidence,
                },
                explicit: is_explicit,
                newest,
            };
            found
                .entry(key)
                .and_modify(|current| {
                    if scored.explicit || scored.candidate.score > current.candidate.score {
                        *current = ScoredRoot {
                            candidate: scored.candidate.clone(),
                            explicit: current.explicit || scored.explicit,
                            newest: scored.newest,
                        };
                    }
                })
                .or_insert(scored);
        }
    }

    let mut values: Vec<_> = found.into_values().collect();
    values.sort_by(|left, right| {
        right
            .explicit
            .cmp(&left.explicit)
            .then_with(|| right.candidate.score.cmp(&left.candidate.score))
            .then_with(|| right.newest.cmp(&left.newest))
    });
    values.into_iter().map(|value| value.candidate).collect()
}

fn inspect_root(path: &Path) -> Option<(u16, Vec<RootEvidence>, SystemTime)> {
    let mut score = 0;
    let mut evidence = Vec::new();
    let mut newest = SystemTime::UNIX_EPOCH;

    if path.join("sessions").is_dir() {
        score += 2;
        evidence.push(RootEvidence::Sessions);
        newest = newest.max(modified(&path.join("sessions")));
    }
    if path.join("models_cache.json").is_file() {
        score += 2;
        evidence.push(RootEvidence::ModelsCache);
        newest = newest.max(modified(&path.join("models_cache.json")));
    }
    if path.join("auth.json").is_file() {
        score += 1;
        evidence.push(RootEvidence::Auth);
        newest = newest.max(modified(&path.join("auth.json")));
    }

    for entry in fs::read_dir(path).ok()?.filter_map(Result::ok) {
        let file_type = entry.file_type().ok()?;
        if !file_type.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name.starts_with("state_") && (name.ends_with(".sqlite") || name.ends_with(".db")) {
            if !evidence.contains(&RootEvidence::StateDatabase) {
                score += 3;
                evidence.push(RootEvidence::StateDatabase);
            }
            newest = newest.max(modified(&entry.path()));
        } else if name.ends_with(".jsonl") {
            if !evidence.contains(&RootEvidence::RolloutLog) {
                score += 1;
                evidence.push(RootEvidence::RolloutLog);
            }
            newest = newest.max(modified(&entry.path()));
        }
    }

    (score >= 3).then_some((score, evidence, newest))
}

fn modified(path: &Path) -> SystemTime {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

#[cfg(test)]
mod tests {
    use super::{discover_roots, DiscoveryInputs};
    use std::path::Path;

    fn fixture_root(path: &Path) {
        std::fs::create_dir_all(path.join("sessions")).unwrap();
        std::fs::write(path.join("models_cache.json"), "{}").unwrap();
        rusqlite::Connection::open(path.join("state_1.sqlite")).unwrap();
    }

    #[test]
    fn codex_home_has_priority_without_a_fixed_directory_name() {
        let temp = tempfile::tempdir().unwrap();
        let custom = temp.path().join("portable-data");
        fixture_root(&custom);
        let inputs = DiscoveryInputs {
            home: temp.path().join("home"),
            codex_home: Some(custom.clone()),
            system_roots: vec![],
        };

        assert_eq!(discover_roots(&inputs)[0].path, custom);
    }

    #[test]
    fn discovers_nested_structural_candidate_and_deduplicates_it() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("Local").join("client-state");
        fixture_root(&candidate);
        let inputs = DiscoveryInputs {
            home: temp.path().to_path_buf(),
            codex_home: Some(candidate.clone()),
            system_roots: vec![temp.path().join("Local")],
        };

        assert_eq!(
            discover_roots(&inputs)
                .iter()
                .filter(|value| value.path == candidate)
                .count(),
            1
        );
    }
}
