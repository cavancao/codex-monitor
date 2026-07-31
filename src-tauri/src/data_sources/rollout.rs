use super::candidate::{AdapterResult, DiagnosticIssue, FieldCandidate};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::SystemTime,
};
use walkdir::WalkDir;

const MAX_ROLLOUTS: usize = 240;
const MAX_DEPTH: usize = 6;
const MAX_TAIL_BYTES: u64 = 256 * 1024;

#[derive(Clone, Debug, Default)]
pub struct RolloutCandidates {
    pub model: Option<FieldCandidate<String>>,
    pub reasoning_effort: Option<FieldCandidate<String>>,
    pub speed_mode: Option<FieldCandidate<String>>,
    pub subscription: Option<FieldCandidate<String>>,
    pub remaining_percent: Option<FieldCandidate<f64>>,
    pub reset_at: Option<FieldCandidate<String>>,
    pub client_version: Option<FieldCandidate<String>>,
}

pub fn discover_rollouts(root: &Path, hinted_paths: &[PathBuf]) -> Vec<PathBuf> {
    let Ok(root_key) = root.canonicalize() else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let mut seen = HashSet::new();

    for hinted in hinted_paths {
        let path = if hinted.is_absolute() {
            hinted.clone()
        } else {
            root.join(hinted)
        };
        push_rollout(&root_key, path, &mut seen, &mut found);
    }
    for entry in WalkDir::new(root)
        .max_depth(MAX_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        push_rollout(
            &root_key,
            entry.path().to_path_buf(),
            &mut seen,
            &mut found,
        );
    }

    found.sort_by_key(|path| {
        std::cmp::Reverse(
            fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH),
        )
    });
    found.truncate(MAX_ROLLOUTS);
    found
}

pub fn read_rollout_candidates(
    root: &Path,
    paths: &[PathBuf],
) -> AdapterResult<RolloutCandidates> {
    let mut result = AdapterResult::default();
    let Ok(root_key) = root.canonicalize() else {
        result
            .issues
            .push(DiagnosticIssue::new("rollout-root-invalid", "rollout"));
        return result;
    };

    for path in paths {
        let Ok(path_key) = path.canonicalize() else {
            result.issues.push(DiagnosticIssue::new(
                "rollout-path-missing",
                safe_file_name(path),
            ));
            continue;
        };
        if !path_key.starts_with(&root_key) {
            result.issues.push(DiagnosticIssue::new(
                "rollout-path-outside-root",
                safe_file_name(path),
            ));
            continue;
        }
        match read_tail(path) {
            Ok(text) => parse_rollout(path, &text, &mut result),
            Err(_) => result.issues.push(DiagnosticIssue::new(
                "rollout-read-failed",
                safe_file_name(path),
            )),
        }
    }
    result
}

fn push_rollout(
    root_key: &Path,
    path: PathBuf,
    seen: &mut HashSet<PathBuf>,
    found: &mut Vec<PathBuf>,
) {
    if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
        return;
    }
    let Ok(key) = path.canonicalize() else {
        return;
    };
    if key.starts_with(root_key) && seen.insert(key) {
        found.push(path);
    }
}

fn read_tail(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let start = length.saturating_sub(MAX_TAIL_BYTES);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    if start > 0 {
        if let Some(line_end) = bytes.iter().position(|byte| *byte == b'\n') {
            bytes.drain(..=line_end);
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn parse_rollout(
    path: &Path,
    text: &str,
    result: &mut AdapterResult<RolloutCandidates>,
) {
    let file_time = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now());

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => {
                result.issues.push(DiagnosticIssue::new(
                    "rollout-invalid-line",
                    safe_file_name(path),
                ));
                continue;
            }
        };
        let observed_at = value
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or(file_time);

        set_string(
            &mut result.value.model,
            first_string(
                &value,
                &[
                    "/payload/thread_settings/model",
                    "/payload/model",
                    "/payload/model_slug",
                ],
            ),
            observed_at,
            0.96,
        );
        set_string(
            &mut result.value.reasoning_effort,
            first_string(
                &value,
                &[
                    "/payload/thread_settings/reasoning_effort",
                    "/payload/reasoning_effort",
                    "/payload/effort",
                ],
            ),
            observed_at,
            0.96,
        );
        set_string(
            &mut result.value.speed_mode,
            first_string(
                &value,
                &[
                    "/payload/thread_settings/service_tier",
                    "/payload/service_tier",
                    "/payload/speed_mode",
                ],
            ),
            observed_at,
            0.96,
        );
        set_string(
            &mut result.value.client_version,
            first_string(
                &value,
                &[
                    "/payload/cli_version",
                    "/payload/client_version",
                    "/payload/version",
                ],
            ),
            observed_at,
            0.92,
        );

        if value.pointer("/payload/type").and_then(Value::as_str) != Some("token_count") {
            continue;
        }
        let Some(primary) = value.pointer("/payload/rate_limits/primary") else {
            continue;
        };
        if let Some(used) = primary.get("used_percent").and_then(Value::as_f64) {
            set_newest(
                &mut result.value.remaining_percent,
                FieldCandidate::new(
                    (100.0 - used).clamp(0.0, 100.0),
                    "rollout",
                    observed_at,
                    0.98,
                ),
            );
        }
        if let Some(epoch) = primary.get("resets_at").and_then(Value::as_i64) {
            if let Some(reset) = Utc.timestamp_opt(epoch, 0).single() {
                set_newest(
                    &mut result.value.reset_at,
                    FieldCandidate::new(reset.to_rfc3339(), "rollout", observed_at, 0.98),
                );
            }
        }
        set_string(
            &mut result.value.subscription,
            first_string(
                &value,
                &[
                    "/payload/rate_limits/plan_type",
                    "/payload/rate_limits/plan",
                    "/payload/plan_type",
                ],
            ),
            observed_at,
            0.98,
        );
    }
}

fn first_string(value: &Value, pointers: &[&str]) -> Option<String> {
    pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
        .map(str::to_owned)
}

fn set_string(
    slot: &mut Option<FieldCandidate<String>>,
    value: Option<String>,
    observed_at: DateTime<Utc>,
    confidence: f32,
) {
    if let Some(value) = value {
        set_newest(
            slot,
            FieldCandidate::new(value, "rollout", observed_at, confidence),
        );
    }
}

fn set_newest<T>(slot: &mut Option<FieldCandidate<T>>, candidate: FieldCandidate<T>) {
    if slot
        .as_ref()
        .is_none_or(|current| candidate.observed_at > current.observed_at)
    {
        *slot = Some(candidate);
    }
}

fn safe_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("rollout")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{discover_rollouts, read_rollout_candidates};
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use std::{fs, path::Path};

    fn write_rollout(root: &Path, name: &str, text: &str) {
        let sessions = root.join("sessions").join("2026").join("07");
        fs::create_dir_all(&sessions).unwrap();
        fs::write(sessions.join(name), text).unwrap();
    }

    fn token_count_event(plan: &str, used: f64, epoch: i64) -> String {
        json!({
            "timestamp": Utc.timestamp_opt(epoch, 0).single().unwrap().to_rfc3339(),
            "payload": {
                "type": "token_count",
                "rate_limits": {
                    "primary": {
                        "used_percent": used,
                        "resets_at": epoch + 3600
                    },
                    "plan_type": plan
                }
            }
        })
        .to_string()
    }

    #[test]
    fn newer_rate_event_wins_across_multiple_rollouts() {
        let temp = tempfile::tempdir().unwrap();
        write_rollout(
            temp.path(),
            "old.jsonl",
            &token_count_event("free", 40.0, 1_700_000_000),
        );
        write_rollout(
            temp.path(),
            "new.jsonl",
            &token_count_event("plus", 12.0, 1_800_000_000),
        );

        let paths = discover_rollouts(temp.path(), &[]);
        let result = read_rollout_candidates(temp.path(), &paths);

        assert_eq!(result.value.subscription.unwrap().value, "plus");
        assert_eq!(result.value.remaining_percent.unwrap().value, 88.0);
    }

    #[test]
    fn invalid_lines_do_not_discard_valid_tail_events() {
        let temp = tempfile::tempdir().unwrap();
        let valid = token_count_event("plus", 5.0, 1_800_000_000);
        write_rollout(
            temp.path(),
            "mixed.jsonl",
            &format!("bad json\n{valid}\n"),
        );

        let result =
            read_rollout_candidates(temp.path(), &discover_rollouts(temp.path(), &[]));

        assert_eq!(result.value.remaining_percent.unwrap().value, 95.0);
        assert!(result
            .issues
            .iter()
            .any(|issue| issue.code == "rollout-invalid-line"));
    }
}
