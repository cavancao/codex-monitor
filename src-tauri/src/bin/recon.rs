use codex_monitor_lib::discovery::{run_recon, write_reports};
fn main() {
    let output = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let report = run_recon();
    if let Err(error) = write_reports(&report, &output) { eprintln!("侦察报告写入失败: {error}"); std::process::exit(1); }
    println!("侦察完成：{} 个候选文件。报告位于 {}", report.files.len(), output.display());
}
