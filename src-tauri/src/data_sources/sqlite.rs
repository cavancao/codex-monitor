use super::candidate::{AdapterResult, DiagnosticIssue, FieldCandidate};
use chrono::{TimeZone, Utc};
use rusqlite::{Connection, OpenFlags};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Clone, Debug, Default)]
pub struct ThreadCandidates {
    pub model: Option<FieldCandidate<String>>,
    pub reasoning_effort: Option<FieldCandidate<String>>,
    pub client_version: Option<FieldCandidate<String>>,
    pub rollout_paths: Vec<FieldCandidate<PathBuf>>,
}

pub fn read_thread_candidates(root: &Path) -> Result<AdapterResult<ThreadCandidates>, String> {
    let mut result = AdapterResult::default();
    for database in state_databases(root) {
        match read_database(&database) {
            Ok(value) => merge_threads(&mut result.value, value),
            Err(code) => result.issues.push(DiagnosticIssue::new(
                code,
                database
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("state-db"),
            )),
        }
    }
    Ok(result)
}

fn state_databases(root: &Path) -> Vec<PathBuf> {
    let mut databases: Vec<_> = fs::read_dir(root)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| {
                    let name = name.to_ascii_lowercase();
                    name.starts_with("state_")
                        && (name.ends_with(".sqlite")
                            || name.ends_with(".sqlite3")
                            || name.ends_with(".db"))
                })
                .unwrap_or(false)
        })
        .collect();
    databases.sort_by_key(|path| {
        std::cmp::Reverse(
            fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH),
        )
    });
    databases
}

fn read_database(path: &Path) -> Result<ThreadCandidates, &'static str> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| "sqlite-open-failed")?;
    let table = thread_table(&connection).ok_or("sqlite-thread-table-missing")?;
    let columns = table_columns(&connection, &table).map_err(|_| "sqlite-schema-failed")?;

    let model = select_column(&columns, &["model", "model_slug"]);
    let effort = select_column(&columns, &["reasoning_effort", "effort"]);
    let version = select_column(&columns, &["cli_version", "app_version", "client_version"]);
    let rollout = select_column(&columns, &["rollout_path", "log_path"]);
    let updated = select_column(
        &columns,
        &["updated_at_ms", "updated_at", "modified_at_ms"],
    );
    if model.is_none() && effort.is_none() && version.is_none() && rollout.is_none() {
        return Err("sqlite-supported-columns-missing");
    }

    let model_expr = expression(model.as_deref());
    let effort_expr = expression(effort.as_deref());
    let version_expr = expression(version.as_deref());
    let rollout_expr = expression(rollout.as_deref());
    let updated_expr = expression(updated.as_deref());
    let order = updated
        .as_deref()
        .map(|name| format!(" ORDER BY {} DESC", quote_identifier(name)))
        .unwrap_or_default();
    let sql = format!(
        "SELECT {model_expr}, {effort_expr}, {version_expr}, {rollout_expr}, {updated_expr} \
         FROM {}{order} LIMIT 32",
        quote_identifier(&table)
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|_| "sqlite-query-prepare-failed")?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i64>>(4)?,
            ))
        })
        .map_err(|_| "sqlite-query-failed")?;

    let file_time = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map(chrono::DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now());
    let mut output = ThreadCandidates::default();
    for row in rows.filter_map(Result::ok) {
        let observed_at = row.4.and_then(epoch_time).unwrap_or(file_time);
        if let Some(value) = row
            .0
            .filter(|value| !value.trim().eq_ignore_ascii_case("codex-auto-review"))
        {
            set_newest(
                &mut output.model,
                FieldCandidate::new(value, "sqlite", observed_at, 0.86),
            );
        }
        if let Some(value) = row.1 {
            set_newest(
                &mut output.reasoning_effort,
                FieldCandidate::new(value, "sqlite", observed_at, 0.86),
            );
        }
        if let Some(value) = row.2 {
            set_newest(
                &mut output.client_version,
                FieldCandidate::new(value, "sqlite", observed_at, 0.86),
            );
        }
        if let Some(value) = row.3 {
            output.rollout_paths.push(FieldCandidate::new(
                PathBuf::from(value),
                "sqlite",
                observed_at,
                0.70,
            ));
        }
    }
    Ok(output)
}

fn thread_table(connection: &Connection) -> Option<String> {
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .ok()?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(0))
        .ok()?
        .filter_map(Result::ok);
    names
        .filter(|name| {
            let lower = name.to_ascii_lowercase();
            lower == "threads" || lower == "thread"
        })
        .max_by_key(|name| usize::from(name.eq_ignore_ascii_case("threads")))
}

fn table_columns(
    connection: &Connection,
    table: &str,
) -> rusqlite::Result<HashMap<String, String>> {
    let sql = format!("PRAGMA table_info({})", quote_identifier(table));
    let mut statement = connection.prepare(&sql)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(Result::ok)
        .map(|name| (name.to_ascii_lowercase(), name))
        .collect();
    Ok(columns)
}

fn select_column(columns: &HashMap<String, String>, aliases: &[&str]) -> Option<String> {
    aliases
        .iter()
        .find_map(|alias| columns.get(*alias).cloned())
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn expression(column: Option<&str>) -> String {
    column
        .map(quote_identifier)
        .unwrap_or_else(|| "NULL".to_owned())
}

fn epoch_time(value: i64) -> Option<chrono::DateTime<Utc>> {
    if value.abs() >= 10_000_000_000 {
        Utc.timestamp_millis_opt(value).single()
    } else {
        Utc.timestamp_opt(value, 0).single()
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

fn merge_threads(target: &mut ThreadCandidates, source: ThreadCandidates) {
    if let Some(value) = source.model {
        set_newest(&mut target.model, value);
    }
    if let Some(value) = source.reasoning_effort {
        set_newest(&mut target.reasoning_effort, value);
    }
    if let Some(value) = source.client_version {
        set_newest(&mut target.client_version, value);
    }
    target.rollout_paths.extend(source.rollout_paths);
}

#[cfg(test)]
mod tests {
    use super::read_thread_candidates;
    use std::path::Path;

    struct SqliteFixture(tempfile::TempDir);

    impl SqliteFixture {
        fn path(&self) -> &Path {
            self.0.path()
        }
    }

    fn sqlite_fixture(schema_and_rows: &str) -> SqliteFixture {
        let temp = tempfile::tempdir().unwrap();
        let connection =
            rusqlite::Connection::open(temp.path().join("state_1.sqlite")).unwrap();
        connection.execute_batch(schema_and_rows).unwrap();
        drop(connection);
        SqliteFixture(temp)
    }

    #[test]
    fn missing_tokens_column_does_not_hide_model_and_effort() {
        let fixture = sqlite_fixture(
            "CREATE TABLE threads(
                model TEXT,
                reasoning_effort TEXT,
                cli_version TEXT,
                updated_at_ms INTEGER
            );
            INSERT INTO threads VALUES('gpt-5.6-sol', 'high', '1.2.3', 1000);",
        );

        let result = read_thread_candidates(fixture.path()).unwrap();

        assert_eq!(result.value.model.unwrap().value, "gpt-5.6-sol");
        assert_eq!(result.value.reasoning_effort.unwrap().value, "high");
    }

    #[test]
    fn supports_version_alias_without_requiring_all_known_columns() {
        let fixture = sqlite_fixture(
            "CREATE TABLE threads(
                model_slug TEXT,
                effort TEXT,
                app_version TEXT,
                updated_at INTEGER
            );
            INSERT INTO threads VALUES('gpt-5.6-sol', 'medium', '2.0.0', 2000);",
        );

        let result = read_thread_candidates(fixture.path()).unwrap();

        assert_eq!(result.value.client_version.unwrap().value, "2.0.0");
    }
}
