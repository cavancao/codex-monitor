use super::candidate::DiagnosticIssue;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticRoot {
    pub path: PathBuf,
    pub evidence: Vec<String>,
    pub tables: Vec<String>,
    pub columns: Vec<String>,
    pub event_types: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub roots: Vec<DiagnosticRoot>,
    pub issues: Vec<DiagnosticIssue>,
}

impl DiagnosticReport {
    pub fn redacted_for(&self, home: &Path) -> Self {
        Self {
            roots: self
                .roots
                .iter()
                .map(|root| DiagnosticRoot {
                    path: mask_home(&root.path, home),
                    evidence: sanitize_names(&root.evidence),
                    tables: sanitize_names(&root.tables),
                    columns: sanitize_names(&root.columns),
                    event_types: sanitize_names(&root.event_types),
                })
                .collect(),
            issues: self
                .issues
                .iter()
                .map(|issue| DiagnosticIssue {
                    code: safe_name(&issue.code),
                    source: safe_name(&issue.source),
                })
                .collect(),
        }
    }
}

pub fn write_diagnostics(path: &Path, report: &DiagnosticReport) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let json = serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| error.to_string())
}

fn mask_home(path: &Path, home: &Path) -> PathBuf {
    path.strip_prefix(home)
        .map(|relative| PathBuf::from("%USER_HOME%").join(relative))
        .unwrap_or_else(|_| PathBuf::from(safe_name(&path.to_string_lossy())))
}

fn sanitize_names(values: &[String]) -> Vec<String> {
    values.iter().map(|value| safe_name(value)).collect()
}

fn safe_name(value: &str) -> String {
    let trimmed = value.trim();
    if !trimmed.is_empty()
        && trimmed.len() <= 80
        && trimmed
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
    {
        trimmed.to_owned()
    } else {
        "redacted".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{DiagnosticReport, DiagnosticRoot};
    use crate::data_sources::candidate::DiagnosticIssue;
    use std::path::{Path, PathBuf};

    #[test]
    fn report_contains_structure_but_not_values_or_home_prefix() {
        let report = DiagnosticReport {
            roots: vec![DiagnosticRoot {
                path: PathBuf::from("C:/Users/Alice/.codex"),
                evidence: vec!["state-database".into()],
                tables: vec!["threads".into()],
                columns: vec!["model".into(), "reasoning_effort".into()],
                event_types: vec!["token_count".into()],
            }],
            issues: vec![DiagnosticIssue::new(
                "auth-token-invalid",
                "alice@example.com eyJsecret",
            )],
        };

        let redacted = report.redacted_for(Path::new("C:/Users/Alice"));
        let json = serde_json::to_string(&redacted).unwrap();

        assert!(json.contains("threads"));
        assert!(!json.contains("Alice"));
        assert!(!json.contains("alice@example.com"));
        assert!(!json.contains("eyJ"));
    }
}
