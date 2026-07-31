use crate::{
    data_sources::{
        discovery::DiscoveryInputs,
        snapshot::{collect_snapshot, SnapshotOptions},
    },
    status::CodexStatus,
};
use std::path::PathBuf;

pub fn snapshot() -> Result<CodexStatus, String> {
    snapshot_with_diagnostics(None, false)
}

pub fn snapshot_with_diagnostics(
    diagnostics_path: Option<PathBuf>,
    force_discovery: bool,
) -> Result<CodexStatus, String> {
    let inputs = DiscoveryInputs::current()?;
    collect_snapshot(
        &inputs,
        &SnapshotOptions {
            force_discovery,
            diagnostics_path,
        },
    )
}

#[cfg(test)]
fn snapshot_for_inputs(
    inputs: &DiscoveryInputs,
    diagnostics_path: Option<&std::path::Path>,
) -> Result<CodexStatus, String> {
    collect_snapshot(
        inputs,
        &SnapshotOptions {
            force_discovery: true,
            diagnostics_path: diagnostics_path.map(std::path::Path::to_path_buf),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::fs;

    #[test]
    fn integration_fixture_does_not_depend_on_real_user_home() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable-codex-data");
        fs::create_dir_all(root.join("sessions")).unwrap();
        fs::write(root.join("models_cache.json"), "{}").unwrap();

        let connection = Connection::open(root.join("state_1.sqlite")).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE threads(
                    model TEXT,
                    reasoning_effort TEXT,
                    cli_version TEXT,
                    updated_at_ms INTEGER
                );
                INSERT INTO threads VALUES(
                    'gpt-5.6-sol',
                    'high',
                    '2.0.0',
                    1800000000000
                );",
            )
            .unwrap();
        drop(connection);

        fs::write(
            root.join("sessions").join("active.jsonl"),
            r#"{"timestamp":"2027-01-15T08:00:00Z","payload":{"type":"token_count","rate_limits":{"primary":{"used_percent":13,"resets_at":1800003600},"plan_type":"plus"}}}"#,
        )
        .unwrap();
        let inputs = DiscoveryInputs {
            home: temp.path().join("empty-home"),
            codex_home: Some(root),
            system_roots: vec![],
        };

        let status = snapshot_for_inputs(&inputs, None).unwrap();

        assert_eq!(status.model.value.as_deref(), Some("gpt-5.6-sol"));
        assert_eq!(status.subscription.value.as_deref(), Some("plus"));
        assert_eq!(status.remaining_percent.value, Some(87.0));
    }
}
