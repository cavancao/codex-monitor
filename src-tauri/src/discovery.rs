use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs::{self, File}, io::Read, net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream}, path::{Path, PathBuf}, time::{Duration, SystemTime}};
use sysinfo::System;
use walkdir::WalkDir;
use crate::data_sources::discovery::{discover_roots, DiscoveryInputs};

const MAX_FILES: usize = 240;
const MAX_SAMPLE: u64 = 64 * 1024;
const MAX_DEPTH: usize = 4;
const KEYWORDS: &[&str] = &["quota","usage","used","remain","percent","reset","resetat","model","plan","subscription","token","tokens_per_second","reasoning","effort","version","user","email"];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateFile { pub path: PathBuf, pub size: u64, pub modified_at: Option<DateTime<Utc>>, pub score: usize, pub hits: Vec<String>, pub snippets: Vec<String> }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateProcess { pub name: String, pub executable: Option<PathBuf>, pub score: usize }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopbackResult { pub port: u16, pub reachable: bool }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconReport { pub generated_at: DateTime<Utc>, pub roots: Vec<PathBuf>, pub files: Vec<CandidateFile>, pub processes: Vec<CandidateProcess>, pub loopback: Vec<LoopbackResult>, pub recommendation: String }

pub fn system_roots() -> Vec<PathBuf> {
    DiscoveryInputs::current()
        .map(|inputs| system_roots_for(&inputs))
        .unwrap_or_default()
}

pub fn system_roots_for(inputs: &DiscoveryInputs) -> Vec<PathBuf> {
    let mut roots = inputs.system_roots.clone();
    roots.extend(discover_roots(inputs).into_iter().map(|candidate| candidate.path));
    roots.sort(); roots.dedup(); roots.into_iter().filter(|p| p.exists()).collect()
}

fn candidate_extension(path: &Path) -> bool {
    path.extension().and_then(|v| v.to_str()).map(|v| matches!(v.to_ascii_lowercase().as_str(), "json"|"log"|"db"|"sqlite"|"sqlite3"|"ldb")).unwrap_or(false)
}

pub fn sanitize(input: &str) -> String {
    let email = Regex::new(r"(?i)([a-z0-9._%+-])[^@\s]*@([^\s.]+(?:\.[^\s.]+)+)").unwrap();
    let token = Regex::new(r#"(?i)(bearer\s+|token["'\s:=]+|sk-)[a-z0-9._-]{8,}"#).unwrap();
    let cleaned = email.replace_all(input, "$1***@$2");
    token.replace_all(&cleaned, "$1***").into_owned()
}

fn inspect_file(path: &Path) -> Option<CandidateFile> {
    let metadata = fs::metadata(path).ok()?; let file = File::open(path).ok()?;
    let mut bytes = Vec::new(); file.take(MAX_SAMPLE).read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
    let hits: Vec<String> = KEYWORDS.iter().filter(|k| text.contains(**k)).map(|k| (*k).to_string()).collect();
    if hits.is_empty() { return None; }
    let snippets = text.lines().filter(|line| hits.iter().any(|k| line.contains(k))).take(3).map(|s| sanitize(&s.chars().take(240).collect::<String>())).collect();
    Some(CandidateFile { path: path.to_path_buf(), size: metadata.len(), modified_at: metadata.modified().ok().map(DateTime::<Utc>::from), score: hits.len(), hits, snippets })
}

pub fn run_recon() -> ReconReport {
    let roots = system_roots(); let recent = SystemTime::now().checked_sub(Duration::from_secs(180 * 86400)).unwrap_or(SystemTime::UNIX_EPOCH);
    let mut seen = HashSet::new(); let mut files = Vec::new();
    for root in &roots {
        for entry in WalkDir::new(root).max_depth(MAX_DEPTH).follow_links(false).into_iter().filter_map(Result::ok) {
            if files.len() >= MAX_FILES { break; }
            let path = entry.path();
            if !entry.file_type().is_file() || !candidate_extension(path) || !seen.insert(path.to_path_buf()) { continue; }
            if entry.metadata().ok().and_then(|m| m.modified().ok()).is_some_and(|m| m < recent) { continue; }
            if let Some(candidate) = inspect_file(path) { files.push(candidate); }
        }
    }
    files.sort_by(|a,b| b.score.cmp(&a.score).then_with(|| b.modified_at.cmp(&a.modified_at)));
    let mut system = System::new_all(); system.refresh_all();
    let mut processes: Vec<CandidateProcess> = system.processes().values().filter_map(|process| {
        let name = process.name().to_string_lossy().into_owned();
        let command = process.cmd().iter().map(|v| v.to_string_lossy()).collect::<Vec<_>>().join(" ").to_ascii_lowercase();
        let score = KEYWORDS.iter().filter(|key| name.to_ascii_lowercase().contains(**key) || command.contains(**key)).count();
        (score > 0).then(|| CandidateProcess { name, executable: process.exe().map(Path::to_path_buf), score })
    }).collect(); processes.sort_by(|a,b| b.score.cmp(&a.score)); processes.truncate(24);
    let loopback = [3000, 8000, 8080, 8787, 1420].into_iter().map(|port| LoopbackResult {
        port, reachable: TcpStream::connect_timeout(&SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port), Duration::from_millis(80)).is_ok()
    }).collect();
    ReconReport { generated_at: Utc::now(), roots, recommendation: if files.is_empty() { "未发现可靠来源；auto 模式将显示 --" } else { "优先根据高分文件生成 field-mapping.json，再启用 file/log provider" }.into(), files, processes, loopback }
}

pub fn write_reports(report: &ReconReport, output: &Path) -> Result<(), String> {
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    fs::write(output.join("recon-report.json"), serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let mut md = format!("# Codex Monitor Recon Report\n\n生成时间：{}\n\n> 报告仅来自本机只读抽样，敏感信息已脱敏。\n\n", report.generated_at);
    for item in &report.files { md.push_str(&format!("- `{}` — score {}, {} bytes, hits: {}\n", item.path.display(), item.score, item.size, item.hits.join(", "))); }
    md.push_str("\n## 候选进程\n\n"); for item in &report.processes { md.push_str(&format!("- `{}` — score {}\n", item.name, item.score)); }
    md.push_str("\n## 本机回环端口\n\n"); for item in &report.loopback { md.push_str(&format!("- 127.0.0.1:{} — {}\n", item.port, if item.reachable { "reachable" } else { "closed" })); }
    md.push_str(&format!("\n## 建议\n\n{}\n", report.recommendation)); fs::write(output.join("recon-report.md"), md).map_err(|e| e.to_string())
}

pub fn validate_loopback(host: IpAddr) -> bool { host == IpAddr::V4(Ipv4Addr::LOCALHOST) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_sources::discovery::DiscoveryInputs;
    #[test] fn roots_are_dynamic_and_existing() { assert!(system_roots().iter().all(|p| p.exists())); }
    #[test] fn redacts_secrets() { let s=sanitize("me@example.com bearer abcdefghijklmnop"); assert!(!s.contains("me@example.com")); assert!(!s.contains("abcdefghijklmnop")); }
    #[test] fn permits_only_ipv4_loopback() { assert!(validate_loopback("127.0.0.1".parse().unwrap())); assert!(!validate_loopback("8.8.8.8".parse().unwrap())); }

    #[test]
    fn recon_roots_include_structurally_discovered_codex_root() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("portable-data");
        fs::create_dir_all(candidate.join("sessions")).unwrap();
        fs::write(candidate.join("models_cache.json"), "{}").unwrap();
        rusqlite::Connection::open(candidate.join("state_1.sqlite")).unwrap();
        let inputs = DiscoveryInputs {
            home: temp.path().join("home"),
            codex_home: Some(candidate.clone()),
            system_roots: vec![],
        };

        assert!(system_roots_for(&inputs).contains(&candidate));
    }
}
