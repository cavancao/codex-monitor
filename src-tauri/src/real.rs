use crate::status::{CodexStatus, StatusField};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::{fs, path::{Path, PathBuf}};

#[derive(Debug)]
struct ThreadState {
    model: Option<String>, reasoning_effort: Option<String>, tokens_used: Option<f64>,
    cli_version: Option<String>, rollout_path: Option<PathBuf>,
}

#[derive(Debug)]
struct RateSnapshot { remaining: f64, reset_at: String, plan: Option<String>, speed: Option<f64>, service_tier: Option<String> }

fn field<T>(value: T, confidence: f32) -> StatusField<T> {
    StatusField { value: Some(value), source: Some("file".into()), observed_at: Some(Utc::now().to_rfc3339()), confidence: Some(confidence), stale: false }
}

/// 只根据目录结构识别数据根目录，不依赖用户名、盘符或安装绝对路径。
pub fn discover_data_root(home: &Path) -> Option<PathBuf> {
    fs::read_dir(home).ok()?.filter_map(Result::ok).filter(|entry| entry.file_type().ok().is_some_and(|t| t.is_dir())).map(|entry| {
        let path = entry.path();
        let has_sessions = path.join("sessions").is_dir();
        let has_models = path.join("models_cache.json").is_file();
        let has_state = fs::read_dir(&path).ok().into_iter().flatten().filter_map(Result::ok).any(|f| {
            let name = f.file_name().to_string_lossy().to_ascii_lowercase(); name.starts_with("state_") && name.ends_with(".sqlite")
        });
        let score = usize::from(has_sessions) + usize::from(has_models) + usize::from(has_state) * 2;
        (score, path)
    }).filter(|(score, _)| *score >= 3).max_by_key(|(score, _)| *score).map(|(_, path)| path)
}

fn latest_state_db(root: &Path) -> Option<PathBuf> {
    fs::read_dir(root).ok()?.filter_map(Result::ok).map(|e| e.path()).filter(|path| {
        path.file_name().and_then(|v| v.to_str()).is_some_and(|n| n.starts_with("state_") && n.ends_with(".sqlite"))
    }).max_by_key(|path| fs::metadata(path).and_then(|m| m.modified()).ok())
}

fn read_thread(root: &Path) -> Result<ThreadState, String> {
    let db = latest_state_db(root).ok_or("未发现状态数据库")?;
    let connection = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX).map_err(|e| e.to_string())?;
    connection.query_row(
        "SELECT model, reasoning_effort, tokens_used, cli_version, rollout_path
         FROM threads
         WHERE model IS NOT NULL AND LOWER(TRIM(model)) <> 'codex-auto-review'
         ORDER BY updated_at_ms DESC
         LIMIT 1",
        [], |row| Ok(ThreadState { model: row.get(0)?, reasoning_effort: row.get(1)?, tokens_used: row.get::<_, Option<i64>>(2)?.map(|v| v as f64), cli_version: row.get(3)?, rollout_path: row.get::<_, Option<String>>(4)?.map(PathBuf::from) })
    ).map_err(|e| format!("读取当前线程失败: {e}"))
}

fn latest_rate_limit(root: &Path, rollout: Option<&Path>) -> Option<RateSnapshot> {
    let path = rollout?.canonicalize().ok()?;
    if !path.starts_with(root.canonicalize().ok()?) { return None; }
    let content = fs::read_to_string(path).ok()?; let mut latest = None; let mut previous_output: Option<(DateTime<Utc>, f64)> = None; let mut service_tier = None;
    for line in content.lines() {
        let value: Value = match serde_json::from_str(line) { Ok(v) => v, Err(_) => continue };
        if let Some(tier)=value.pointer("/payload/thread_settings/service_tier").and_then(Value::as_str) { service_tier=Some(tier.to_owned()); }
        if value.pointer("/payload/type").and_then(Value::as_str) != Some("token_count") { continue; }
        let Some(primary) = value.pointer("/payload/rate_limits/primary") else { continue };
        let (Some(used), Some(reset_epoch)) = (primary.get("used_percent").and_then(Value::as_f64), primary.get("resets_at").and_then(Value::as_i64)) else { continue };
        let Some(reset) = Utc.timestamp_opt(reset_epoch, 0).single().map(|d| d.to_rfc3339()) else { continue };
        let plan = value.pointer("/payload/rate_limits/plan_type").and_then(Value::as_str).map(str::to_owned);
        let current_output = value.pointer("/payload/info/total_token_usage/output_tokens").and_then(Value::as_f64);
        let current_time = value.get("timestamp").and_then(Value::as_str).and_then(|v| DateTime::parse_from_rfc3339(v).ok()).map(|v| v.with_timezone(&Utc));
        let speed = match (previous_output, current_time, current_output) {
            (Some((before_time, before_tokens)), Some(now), Some(tokens)) => {
                let elapsed=(now-before_time).num_milliseconds() as f64/1000.0; let delta=tokens-before_tokens;
                (elapsed >= 0.1 && elapsed <= 120.0 && delta > 0.0).then_some((delta/elapsed*10.0).round()/10.0)
            }, _ => None
        };
        if let (Some(now), Some(tokens))=(current_time,current_output) { previous_output=Some((now,tokens)); }
        latest = Some(RateSnapshot { remaining: (100.0-used).clamp(0.0,100.0), reset_at: reset, plan, speed, service_tier: None });
    }
    latest.map(|mut value| { value.service_tier=service_tier; value })
}

fn jwt_claims(root: &Path) -> Option<(Option<String>, Option<String>)> {
    let auth: Value = serde_json::from_str(&fs::read_to_string(root.join("auth.json")).ok()?).ok()?;
    let token = auth.pointer("/tokens/id_token")?.as_str()?; let payload = token.split('.').nth(1)?;
    let claims: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()?;
    let user = claims.get("name").and_then(Value::as_str).or_else(|| claims.get("email").and_then(Value::as_str)).map(str::to_owned);
    let plan = claims.pointer("/https:~1~1api.openai.com~1auth/chatgpt_plan_type").and_then(Value::as_str).map(str::to_owned);
    Some((user, plan))
}

pub fn snapshot() -> Result<CodexStatus, String> {
    let home = dirs::home_dir().ok_or("无法解析当前用户 home 目录")?;
    let root = discover_data_root(&home).ok_or("未发现可信的本地状态目录")?;
    let thread = read_thread(&root)?; let rate = latest_rate_limit(&root, thread.rollout_path.as_deref());
    let claims = jwt_claims(&root).unwrap_or_default();
    let mut status = CodexStatus { sync_state: "connected".into(), message: Some("已连接本地只读状态".into()), ..CodexStatus::default() };
    if let Some(v)=thread.model { status.model=field(v,0.98); }
    if let Some(v)=thread.reasoning_effort { status.reasoning_effort=field(v,0.98); }
    if let Some(v)=thread.cli_version { status.client_version=field(v,0.98); }
    if let Some(v)=claims.0 { status.username=field(v,0.9); }
    if let Some(v)=claims.1.or_else(|| rate.as_ref().and_then(|v| v.plan.clone())) { status.subscription=field(v,0.9); }
    if let Some(rate)=rate {
        status.remaining_percent=field(rate.remaining,0.98); status.reset_at=field(rate.reset_at,0.98);
        if let Some(speed)=rate.speed { status.reasoning_speed=field(speed,0.85); }
        if let Some(tier)=rate.service_tier { status.speed_mode=field(tier,0.98); }
    }
    // tokens_used 是当前线程累计量，不能冒充“本月使用量”。
    let _ = thread.tokens_used;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_by_structure_without_directory_name() {
        let temp=tempfile::tempdir().unwrap(); let candidate=temp.path().join("random-client-data");
        fs::create_dir_all(candidate.join("sessions")).unwrap(); fs::write(candidate.join("models_cache.json"), "{}").unwrap(); fs::write(candidate.join("state_1.sqlite"), "").unwrap();
        assert_eq!(discover_data_root(temp.path()), Some(candidate));
    }

    #[test]
    fn live_snapshot_contains_core_fields_when_local_data_exists() {
        let Some(home)=dirs::home_dir() else { return }; if discover_data_root(&home).is_none() { return; }
        let status=snapshot().expect("已发现本地数据时应能生成快照");
        assert!(status.model.value.is_some());
        assert!(status.client_version.value.is_some());
        assert!(status.reasoning_speed.value.is_some());
        assert!(status.speed_mode.value.is_some());
    }

    #[test]
    fn ignores_internal_auto_review_thread_when_selecting_current_model() {
        let temp = tempfile::tempdir().unwrap();
        let db = temp.path().join("state_1.sqlite");
        let connection = Connection::open(&db).unwrap();
        connection.execute_batch(
            "CREATE TABLE threads (
                model TEXT,
                reasoning_effort TEXT,
                tokens_used INTEGER,
                cli_version TEXT,
                rollout_path TEXT,
                updated_at_ms INTEGER
            );
            INSERT INTO threads VALUES ('gpt-5.6-sol', 'high', 100, '1.0.0', NULL, 1000);
            INSERT INTO threads VALUES ('codex-auto-review', 'medium', 20, '1.0.0', NULL, 2000);"
        ).unwrap();
        drop(connection);

        let thread = read_thread(temp.path()).unwrap();
        assert_eq!(thread.model.as_deref(), Some("gpt-5.6-sol"));
        assert_eq!(thread.reasoning_effort.as_deref(), Some("high"));
    }
}
