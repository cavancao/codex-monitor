use codex_monitor_lib::{discovery::{run_recon as discover, write_reports}, mapping::{read_json_value, FieldMapping}, status::{CodexStatus, StatusField}};
use codex_monitor_lib::real;
use regex::Regex;
use std::{fs, io::{Read, Seek, SeekFrom}, path::{Path, PathBuf}};
use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder, AppHandle, Emitter, Manager, PhysicalPosition, Position, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

const TRAY_SHOW: &str = "show";
const TRAY_TOP: &str = "top";
const TRAY_QUIT: &str = "quit";

#[cfg(test)]
fn tray_menu_ids() -> [&'static str; 3] {
    [TRAY_SHOW, TRAY_TOP, TRAY_QUIT]
}

fn app_data(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|e| format!("无法解析应用数据目录: {e}"))
}

fn positioned(window: &tauri::WebviewWindow) -> Result<(), String> {
    let monitor = window.primary_monitor().map_err(|e| e.to_string())?.ok_or("未找到主显示器")?;
    let work = monitor.work_area();
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let x = work.position.x + work.size.width as i32 - size.width as i32 - 24;
    let y = work.position.y + 24;
    window.set_position(Position::Physical(PhysicalPosition::new(x, y))).map_err(|e| e.to_string())
}

fn mapped_status(app: &AppHandle) -> Result<CodexStatus, String> {
    let path = app_data(app)?.join("field-mapping.json");
    if !path.exists() { return Ok(CodexStatus::unavailable("未发现可信数据源，请运行重新侦察并配置字段映射")); }
    let mapping = FieldMapping::load(&path)?;
    let base = mapping.base_directory.as_deref().ok_or("映射缺少运行时发现的 baseDirectory")?;
    if !base.exists() { return Ok(CodexStatus::unavailable("数据位置已变化，请重新侦察")); }
    let mut out = CodexStatus { sync_state: "connected".into(), ..CodexStatus::default() };
    let now = chrono::Utc::now().to_rfc3339();
    for rule in &mapping.rules {
        let value = match rule.provider.as_str() {
            "file" => match read_json_value(base, rule) { Ok(v) => v, Err(_) => continue },
            "log" => match read_log_value(base, &rule.relative_path, &rule.selector) { Ok(v) => v, Err(_) => continue },
            _ => continue
        };
        let string_field = |v: String| StatusField { value: Some(v), source: Some("file".into()), observed_at: Some(now.clone()), confidence: Some(0.9), stale: false };
        let number_field = |v: f64| StatusField { value: Some(v * rule.scale.unwrap_or(1.0)), source: Some("file".into()), observed_at: Some(now.clone()), confidence: Some(0.9), stale: false };
        match rule.field.as_str() {
            "username" => if let Some(v)=value.as_str(){out.username=string_field(v.into())},
            "model" => if let Some(v)=value.as_str(){out.model=string_field(v.into())},
            "reasoningEffort" => if let Some(v)=value.as_str(){out.reasoning_effort=string_field(v.into())},
            "subscription" => if let Some(v)=value.as_str(){out.subscription=string_field(v.into())},
            "resetAt" => if let Some(v)=value.as_str(){out.reset_at=string_field(v.into())},
            "clientVersion" => if let Some(v)=value.as_str(){out.client_version=string_field(v.into())},
            "reasoningSpeed" => if let Some(v)=value.as_f64(){out.reasoning_speed=number_field(v)},
            "remainingPercent" => if let Some(v)=value.as_f64(){out.remaining_percent=number_field(v.clamp(0.0,100.0))},
            "monthlyUsage" => if let Some(v)=value.as_f64(){out.monthly_usage=number_field(v)},
            "weeklyDurationSeconds" => if let Some(v)=value.as_f64(){out.weekly_duration_seconds=number_field(v)},
            _ => {}
        }
    }
    Ok(out)
}

fn read_log_value(base: &Path, relative: &Path, pattern: &str) -> Result<serde_json::Value, String> {
    if relative.is_absolute() || relative.components().any(|c| matches!(c, std::path::Component::ParentDir)) { return Err("日志路径必须是安全相对路径".into()); }
    let mut file = fs::File::open(base.join(relative)).map_err(|e| format!("只读打开日志失败: {e}"))?;
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    file.seek(SeekFrom::Start(length.saturating_sub(64 * 1024))).map_err(|e| e.to_string())?;
    let mut text = String::new(); file.read_to_string(&mut text).map_err(|e| e.to_string())?;
    let regex = Regex::new(pattern).map_err(|e| format!("无效正则: {e}"))?;
    let capture = regex.captures_iter(&text).last().and_then(|c| c.get(1)).ok_or("日志未命中捕获组")?.as_str();
    Ok(capture.parse::<f64>().map(serde_json::Value::from).unwrap_or_else(|_| serde_json::Value::String(capture.into())))
}

#[tauri::command]
fn get_status(app: AppHandle, provider: String) -> Result<CodexStatus, String> {
    match provider.as_str() {
        "mitm" => Ok(CodexStatus::unsupported("MITM、TLS hook 与读内存首版不实现")),
        "auto" => real::snapshot().or_else(|_| mapped_status(&app)),
        "file" | "log" | "loopback" | "recon" => mapped_status(&app),
        _ => Err("无效 provider".into())
    }
}

#[tauri::command]
fn run_recon(app: AppHandle) -> Result<(), String> {
    let report = discover(); let output = app_data(&app)?.join("recon");
    write_reports(&report, &output)?;
    app.emit("recon-finished", &report).map_err(|e| e.to_string())?;
    app.emit("status-changed", mapped_status(&app)?).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![get_status, run_recon])
        .setup(|app| {
            let app_handle = app.handle().clone();
            let data_dir = app_data(&app_handle).map_err(std::io::Error::other)?;
            fs::create_dir_all(data_dir).map_err(std::io::Error::other)?;
            if let Some(window) = app.get_webview_window("main") { positioned(&window).map_err(std::io::Error::other)?; }
            let show = MenuItem::with_id(app, TRAY_SHOW, "显示/隐藏", true, None::<&str>)?;
            let top = MenuItem::with_id(app, TRAY_TOP, "切换置顶", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, TRAY_QUIT, "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &top, &quit])?;
            let tray_icon = app.default_window_icon().cloned().ok_or_else(|| std::io::Error::other("应用图标未配置"))?;
            TrayIconBuilder::new().icon(tray_icon).menu(&menu).on_menu_event(|app, event| {
                if let Some(window) = app.get_webview_window("main") {
                    match event.id.as_ref() {
                        TRAY_SHOW => { if window.is_visible().unwrap_or(false) { let _=window.hide(); } else { let _=window.show(); let _=window.set_focus(); } },
                        TRAY_TOP => { let next=!window.is_always_on_top().unwrap_or(true); let _=window.set_always_on_top(next); },
                        TRAY_QUIT => app.exit(0), _ => {}
                    }
                }
            }).build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| if let WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _=window.hide(); })
        .run(tauri::generate_context!()).expect("启动 Codex Monitor 失败");
}

#[cfg(test)]
mod tests {
    use super::tray_menu_ids;

    #[test]
    fn tray_menu_excludes_recon() {
        assert_eq!(tray_menu_ids(), ["show", "top", "quit"]);
    }

    #[test]
    fn bundle_defines_application_icons() {
        let config: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert!(!config["bundle"]["icon"].as_array().unwrap().is_empty());
    }
}
