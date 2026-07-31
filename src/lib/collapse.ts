import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { isTauriRuntime } from "./runtime";

const STORAGE_KEY = "codex-monitor:collapsed";

export const HUD_WIDTH = 400;
export const EXPANDED_HEIGHT = 680;
export const COLLAPSED_HEIGHT = 410;

export function readCollapsed(storage: Storage = localStorage): boolean {
  try {
    return storage.getItem(STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

export function writeCollapsed(value: boolean, storage: Storage = localStorage): void {
  try {
    storage.setItem(STORAGE_KEY, String(value));
  } catch {
    // 本地存储不可用时仍允许当前会话切换布局。
  }
}

export function hudSize(collapsed: boolean): { width: number; height: number } {
  return {
    width: HUD_WIDTH,
    height: collapsed ? COLLAPSED_HEIGHT : EXPANDED_HEIGHT,
  };
}

export async function resizeHudWindow(collapsed: boolean): Promise<void> {
  if (!isTauriRuntime()) return;
  const size = hudSize(collapsed);
  await getCurrentWindow().setSize(new LogicalSize(size.width, size.height));
}
