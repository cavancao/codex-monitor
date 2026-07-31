# Codex Monitor

一个基于 Tauri 2、Rust、Vue 3 和 TypeScript 的 Windows 悬浮监控窗口，用于只读展示本机 Codex 的额度、模型、推理配置、订阅和重置时间。

项目不依赖固定安装路径：数据位置通过当前用户目录和运行时结构动态发现，换电脑或 Codex 安装位置变化后可以重新刷新。

## 功能

- 400×680 透明无边框 HUD，启动定位到主屏幕右上角并保持置顶。
- 关闭按钮隐藏到托盘；托盘支持显示/隐藏、切换置顶和退出。
- SVG 剩余额度圆环、动态霓虹灯效、中文日期时间和重置日期。
- 展示模型、订阅套餐、推理强度、速度、用户名和客户端版本。
- 显式 Mock 模式；生产 `auto` 无可信值时显示 `--`。
- 只读 recon，限量抽样 JSON、日志、SQLite/LevelDB 候选并生成脱敏报告。

## 前置依赖

- Node.js 18 或更高版本（推荐当前 LTS）。
- Rust stable：安装 [rustup](https://rustup.rs/)。
- Tauri 2 系统依赖：
  - Windows：安装 Microsoft C++ Build Tools 与 WebView2 Runtime。
  - macOS：`xcode-select --install`。
  - Debian/Ubuntu：`sudo apt update && sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`。

## 安装、运行与构建

```powershell
npm install
npm run tauri dev
```

VS Code 打开 `codex-monitor` 后选择“Codex Monitor：F5 启动”并按 F5。前端断点由 Edge 调试器处理，Rust 输出显示在专用终端；安装 CodeLLDB 后可按需附加 Rust 进程。

```powershell
npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
npm run tauri build
```

## 数据源与动态路径

复制 `.env.example` 为 `.env`，可设置：

```dotenv
VITE_DATA_PROVIDER=auto
```

允许值为 `mock | recon | auto | file | log | loopback | mitm`。只有 `mock` 会生成模拟数据。`mitm` 在首版仅返回“未实现”状态，不会创建代理、安装证书、hook 或读内存。

执行完整侦察：

```powershell
npm run recon
```

命令在当前目录生成 `recon-report.json` 和 `recon-report.md`，二者已加入 `.gitignore`。应用内“刷新”会强制重新发现数据源，并把脱敏诊断保存为 Tauri 动态解析的应用 data 目录下的 `adaptive-diagnostics.json`。

程序代码不包含 Codex 安装绝对路径。运行时优先使用当前用户的 `CODEX_HOME`，再从 home、配置、数据和缓存目录中按结构发现候选来源。SQLite 会先以只读方式检查表和列，再读取当前版本实际存在的兼容字段；单个表、列或日志缺失只影响对应字段。额度信息会从受限深度内的近期 rollout 日志中选择最新可信事件，不依赖旧数据库保存的单一绝对路径。

`field-mapping.json` 仍可位于应用 data 目录作为低优先级补充来源，`base_directory` 必须来自本机侦察，规则的 `relativePath` 只能是安全相对路径。可参考 `field-mapping.example.json`。诊断缓存只包含遮盖后的路径、结构名称和固定错误码，不保存 token、完整邮箱或对话正文。

## 降级规则

- 首次无可信数据：字段显示 `--`。
- 已有可信数据后短暂读取失败：保留上次值并标记 stale。
- 数据目录或结构变化：点击“刷新”重新发现；其他可用字段继续正常显示。
- 窗口隐藏：前端时钟与主动刷新暂停或降频。

## 合规与免责声明

本工具只读取当前用户、本机可访问的数据，不修改、删除或写入任何被监控客户端文件和数据库；不上传数据，不包含遥测。严禁将本项目用于篡改、重放或伪造请求与身份，亦不得用于绕过付费、额度校验或鉴权。

MITM、TLS hook、反调试对抗和进程内存读取不在首版实现范围。用户须自行确认其使用方式符合账户协议、服务条款与当地法律，并自行承担相关风险。

## 当前平台状态

Windows 为首要运行目标。路径发现使用跨平台系统 API，macOS/Linux 代码保持可编译设计，但需在对应系统上完成实际窗口、托盘与侦察验证后才能视为正式支持。
