import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isTauriRuntime } from "../lib/runtime";
import { emptyStatus } from "./status";
import type { CodexStatus, DataProvider, ProviderId } from "./types";

export class TauriProvider implements DataProvider {
  constructor(private readonly id: ProviderId) {}
  getSnapshot(): Promise<CodexStatus> {
    if (!isTauriRuntime()) return Promise.resolve({ ...emptyStatus("浏览器预览模式：真实数据与窗口功能需要在 Tauri 桌面端运行"), syncState: "unsupported" });
    return invoke("get_status", { provider: this.id });
  }
  subscribe(callback: (status: CodexStatus) => void): () => void {
    if (!isTauriRuntime()) return () => undefined;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const poll = () => { if (!disposed && !document.hidden) void this.getSnapshot().then(callback).catch(() => undefined); };
    const timer = window.setInterval(poll, 3000);
    const visibility = () => { if (!document.hidden) poll(); };
    document.addEventListener("visibilitychange", visibility);
    void listen<CodexStatus>("status-changed", event => { if (!disposed) callback(event.payload); }).then(fn => { unlisten = fn; });
    return () => { disposed = true; window.clearInterval(timer); document.removeEventListener("visibilitychange", visibility); unlisten?.(); };
  }
  capabilities(): readonly string[] { return this.id === "mitm" ? [] : [this.id]; }
}
