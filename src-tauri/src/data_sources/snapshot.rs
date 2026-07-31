use super::{
    auth::read_auth_candidates,
    candidate::FieldCandidate,
    diagnostics::{write_diagnostics, DiagnosticReport, DiagnosticRoot},
    discovery::{discover_roots, DataRootCandidate, DiscoveryInputs, RootEvidence},
    rollout::{discover_rollouts, read_rollout_candidates},
    sqlite::read_thread_candidates,
};
use crate::status::{CodexStatus, StatusField};
use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct SnapshotCandidates {
    pub username: Vec<FieldCandidate<String>>,
    pub model: Vec<FieldCandidate<String>>,
    pub reasoning_effort: Vec<FieldCandidate<String>>,
    pub reasoning_speed: Vec<FieldCandidate<f64>>,
    pub speed_mode: Vec<FieldCandidate<String>>,
    pub subscription: Vec<FieldCandidate<String>>,
    pub remaining_percent: Vec<FieldCandidate<f64>>,
    pub reset_at: Vec<FieldCandidate<String>>,
    pub client_version: Vec<FieldCandidate<String>>,
}

#[derive(Clone, Debug, Default)]
pub struct SnapshotOptions {
    pub force_discovery: bool,
    pub diagnostics_path: Option<PathBuf>,
}

#[derive(Default)]
pub struct SnapshotCollector {
    roots: Option<Vec<DataRootCandidate>>,
}

impl SnapshotCollector {
    pub fn collect(
        &mut self,
        inputs: &DiscoveryInputs,
        options: &SnapshotOptions,
    ) -> Result<CodexStatus, String> {
        let cache_is_valid = self
            .roots
            .as_ref()
            .is_some_and(|roots| roots.iter().all(|root| root.path.is_dir()));
        if options.force_discovery || !cache_is_valid {
            self.roots = Some(discover_roots(inputs));
        }
        collect_snapshot_from_roots(inputs, options, self.roots.as_deref().unwrap_or_default())
    }
}

#[derive(Clone, Copy)]
enum StatusKey {
    Username,
    Model,
    ReasoningEffort,
    ReasoningSpeed,
    SpeedMode,
    Subscription,
    RemainingPercent,
    ResetAt,
    ClientVersion,
}

pub fn collect_snapshot(
    inputs: &DiscoveryInputs,
    options: &SnapshotOptions,
) -> Result<CodexStatus, String> {
    let roots = discover_roots(inputs);
    collect_snapshot_from_roots(inputs, options, &roots)
}

fn collect_snapshot_from_roots(
    inputs: &DiscoveryInputs,
    options: &SnapshotOptions,
    roots: &[DataRootCandidate],
) -> Result<CodexStatus, String> {
    let mut candidates = SnapshotCandidates::default();
    let mut report = DiagnosticReport::default();

    for root in roots {
        report.roots.push(DiagnosticRoot {
            path: root.path.clone(),
            evidence: root.evidence.iter().map(evidence_name).collect(),
            tables: Vec::new(),
            columns: Vec::new(),
            event_types: Vec::new(),
        });

        let sqlite = read_thread_candidates(&root.path)?;
        let hinted_paths: Vec<_> = sqlite
            .value
            .rollout_paths
            .iter()
            .map(|candidate| candidate.value.clone())
            .collect();
        push_option(&mut candidates.model, sqlite.value.model);
        push_option(
            &mut candidates.reasoning_effort,
            sqlite.value.reasoning_effort,
        );
        push_option(&mut candidates.client_version, sqlite.value.client_version);
        report.issues.extend(sqlite.issues);

        let rollout_paths = discover_rollouts(&root.path, &hinted_paths);
        let rollout = read_rollout_candidates(&root.path, &rollout_paths);
        push_option(&mut candidates.model, rollout.value.model);
        push_option(
            &mut candidates.reasoning_effort,
            rollout.value.reasoning_effort,
        );
        push_option(&mut candidates.speed_mode, rollout.value.speed_mode);
        push_option(&mut candidates.subscription, rollout.value.subscription);
        push_option(
            &mut candidates.remaining_percent,
            rollout.value.remaining_percent,
        );
        push_option(&mut candidates.reset_at, rollout.value.reset_at);
        push_option(&mut candidates.client_version, rollout.value.client_version);
        report.issues.extend(rollout.issues);

        let auth = read_auth_candidates(&root.path);
        push_option(&mut candidates.username, auth.value.username);
        push_option(&mut candidates.subscription, auth.value.subscription);
        report.issues.extend(auth.issues);
    }

    if let Some(path) = options.diagnostics_path.as_deref() {
        let redacted = report.redacted_for(&inputs.home);
        write_diagnostics(path, &redacted)?;
    }
    Ok(build_status(candidates))
}

pub fn build_status(candidates: SnapshotCandidates) -> CodexStatus {
    let mut status = CodexStatus {
        username: into_status(candidates.username, StatusKey::Username),
        model: into_status(candidates.model, StatusKey::Model),
        reasoning_effort: into_status(candidates.reasoning_effort, StatusKey::ReasoningEffort),
        reasoning_speed: into_status(candidates.reasoning_speed, StatusKey::ReasoningSpeed),
        speed_mode: into_status(candidates.speed_mode, StatusKey::SpeedMode),
        subscription: into_status(candidates.subscription, StatusKey::Subscription),
        remaining_percent: into_status(candidates.remaining_percent, StatusKey::RemainingPercent),
        reset_at: into_status(candidates.reset_at, StatusKey::ResetAt),
        client_version: into_status(candidates.client_version, StatusKey::ClientVersion),
        ..CodexStatus::default()
    };
    let available = [
        status.username.value.is_some(),
        status.model.value.is_some(),
        status.reasoning_effort.value.is_some(),
        status.speed_mode.value.is_some(),
        status.subscription.value.is_some(),
        status.remaining_percent.value.is_some(),
        status.reset_at.value.is_some(),
        status.client_version.value.is_some(),
    ]
    .into_iter()
    .any(|value| value);
    if available {
        status.sync_state = "connected".into();
        status.message = Some("已连接本地只读状态".into());
    } else {
        status.sync_state = "recon-required".into();
        status.message = Some("未发现可信数据源，请确认 Codex 已登录并至少运行过一个任务".into());
    }
    status
}

fn into_status<T>(candidates: Vec<FieldCandidate<T>>, key: StatusKey) -> StatusField<T> {
    let Some(candidate) = select_candidate(candidates, key) else {
        return StatusField::default();
    };
    StatusField {
        value: Some(candidate.value),
        source: Some("file".into()),
        observed_at: Some(candidate.observed_at.to_rfc3339()),
        confidence: Some(candidate.confidence),
        stale: false,
    }
}

fn select_candidate<T>(
    candidates: Vec<FieldCandidate<T>>,
    key: StatusKey,
) -> Option<FieldCandidate<T>> {
    candidates.into_iter().max_by(|left, right| {
        source_priority(key, left.source)
            .cmp(&source_priority(key, right.source))
            .then_with(|| left.observed_at.cmp(&right.observed_at))
            .then_with(|| left.confidence.total_cmp(&right.confidence))
    })
}

fn source_priority(key: StatusKey, source: &str) -> u8 {
    match (key, source) {
        (StatusKey::Subscription, "rollout") => 4,
        (StatusKey::Subscription, "auth") => 3,
        (StatusKey::Username, "auth") => 4,
        (
            StatusKey::Model
            | StatusKey::ReasoningEffort
            | StatusKey::ReasoningSpeed
            | StatusKey::SpeedMode
            | StatusKey::ClientVersion,
            "rollout",
        ) => 4,
        (StatusKey::Model | StatusKey::ReasoningEffort | StatusKey::ClientVersion, "sqlite") => 3,
        (StatusKey::RemainingPercent | StatusKey::ResetAt, "rollout") => 4,
        (_, "mapping") => 1,
        _ => 2,
    }
}

fn push_option<T>(target: &mut Vec<FieldCandidate<T>>, value: Option<FieldCandidate<T>>) {
    if let Some(value) = value {
        target.push(value);
    }
}

fn evidence_name(value: &RootEvidence) -> String {
    match value {
        RootEvidence::Sessions => "sessions",
        RootEvidence::ModelsCache => "models-cache",
        RootEvidence::Auth => "auth",
        RootEvidence::StateDatabase => "state-database",
        RootEvidence::RolloutLog => "rollout-log",
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::{build_status, SnapshotCandidates, SnapshotCollector, SnapshotOptions};
    use crate::data_sources::candidate::FieldCandidate;
    use crate::data_sources::discovery::DiscoveryInputs;
    use chrono::{TimeZone, Utc};
    use rusqlite::Connection;
    use std::fs;

    fn utc(epoch: i64) -> chrono::DateTime<Utc> {
        Utc.timestamp_opt(epoch, 0).single().unwrap()
    }

    #[test]
    fn recent_rate_plan_overrides_older_free_auth_claim() {
        let mut candidates = SnapshotCandidates::default();
        candidates.subscription.push(FieldCandidate::new(
            "free".into(),
            "auth",
            utc(1_700_000_000),
            0.80,
        ));
        candidates.subscription.push(FieldCandidate::new(
            "plus".into(),
            "rollout",
            utc(1_800_000_000),
            0.98,
        ));

        let status = build_status(candidates);

        assert_eq!(status.subscription.value.as_deref(), Some("plus"));
    }

    #[test]
    fn missing_sqlite_effort_does_not_hide_rollout_quota() {
        let mut candidates = SnapshotCandidates::default();
        candidates.remaining_percent.push(FieldCandidate::new(
            87.0,
            "rollout",
            utc(1_800_000_000),
            0.98,
        ));

        let status = build_status(candidates);

        assert_eq!(status.reasoning_effort.value, None);
        assert_eq!(status.remaining_percent.value, Some(87.0));
        assert_eq!(status.sync_state, "connected");
    }

    #[test]
    fn force_discovery_replaces_cached_roots() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first-client-state");
        let second = temp.path().join("second-client-state");
        write_model_fixture(&first, "gpt-first");
        write_model_fixture(&second, "gpt-second");
        let mut inputs = DiscoveryInputs {
            home: temp.path().join("empty-home"),
            codex_home: Some(first),
            system_roots: vec![],
        };
        let mut collector = SnapshotCollector::default();

        let first_status = collector
            .collect(&inputs, &SnapshotOptions::default())
            .unwrap();
        inputs.codex_home = Some(second);
        let cached_status = collector
            .collect(&inputs, &SnapshotOptions::default())
            .unwrap();
        let refreshed_status = collector
            .collect(
                &inputs,
                &SnapshotOptions {
                    force_discovery: true,
                    diagnostics_path: None,
                },
            )
            .unwrap();

        assert_eq!(first_status.model.value.as_deref(), Some("gpt-first"));
        assert_eq!(cached_status.model.value.as_deref(), Some("gpt-first"));
        assert_eq!(refreshed_status.model.value.as_deref(), Some("gpt-second"));
    }

    fn write_model_fixture(root: &std::path::Path, model: &str) {
        fs::create_dir_all(root.join("sessions")).unwrap();
        fs::write(root.join("models_cache.json"), "{}").unwrap();
        let connection = Connection::open(root.join("state_1.sqlite")).unwrap();
        connection
            .execute_batch(&format!(
                "CREATE TABLE threads(model TEXT, updated_at_ms INTEGER);
                 INSERT INTO threads VALUES('{model}', 1800000000000);"
            ))
            .unwrap();
    }
}
