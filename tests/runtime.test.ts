import { describe, expect, it } from "vitest";
import { isTauriRuntime } from "../src/lib/runtime";

describe("运行环境边界", () => {
  it("普通浏览器中不误判为 Tauri", () => {
    expect(isTauriRuntime()).toBe(false);
  });
});
