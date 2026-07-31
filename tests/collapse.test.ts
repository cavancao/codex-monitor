import { beforeEach, describe, expect, it } from "vitest";
import { hudSize, readCollapsed, writeCollapsed } from "../src/lib/collapse";
import capability from "../src-tauri/capabilities/default.json";

describe("HUD 折叠状态", () => {
  beforeEach(() => localStorage.clear());

  it("没有保存状态时默认展开", () => {
    expect(readCollapsed()).toBe(false);
  });

  it("保存并恢复折叠状态", () => {
    writeCollapsed(true);
    expect(readCollapsed()).toBe(true);

    writeCollapsed(false);
    expect(readCollapsed()).toBe(false);
  });

  it("展开与折叠使用正确的窗口尺寸", () => {
    expect(hudSize(false)).toEqual({ width: 400, height: 680 });
    expect(hudSize(true)).toEqual({ width: 400, height: 410 });
  });

  it("允许主窗口切换尺寸", () => {
    expect(capability.permissions).toContain("core:window:allow-set-size");
  });
});
