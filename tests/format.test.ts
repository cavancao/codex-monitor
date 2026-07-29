import { describe, expect, it } from "vitest";
import { formatDateTime, formatHeaderDateTime, isFastSpeed, isHighReasoning, isResetSoon, reasoningLabel, speedLabel } from "../src/lib/format";

describe("显示格式", () => {
  it("日期统一为 yyyy-MM-dd", () => {
    expect(formatDateTime(new Date(2026, 6, 29, 8, 5, 3), true)).toBe("2026-07-29 08:05:03");
    expect(formatDateTime(new Date(2026, 6, 29, 8, 5), false)).toBe("2026-07-29 08:05");
  });

  it("顶部日期使用中文年月日并显示完整星期", () => {
    expect(formatHeaderDateTime(new Date(2026, 6, 29, 8, 5, 3))).toBe("2026年07月29日 08:05:03 星期三");
    expect(formatHeaderDateTime(new Date(Number.NaN))).toBe("--");
  });

  it("Codex 推理强度显示中文", () => {
    expect(reasoningLabel("low")).toBe("轻度");
    expect(reasoningLabel("xhigh")).toBe("极高");
    expect(reasoningLabel("ultra")).toBe("极速");
    expect(reasoningLabel(null)).toBe("--");
  });

  it("Codex 速度档位显示为标准或快速", () => {
    expect(speedLabel("default")).toBe("标准");
    expect(speedLabel("priority")).toBe("快速");
    expect(speedLabel("fast")).toBe("快速");
  });

  it("识别需要红色警示的推理强度和速度", () => {
    expect(isHighReasoning("medium")).toBe(false);
    expect(isHighReasoning("high")).toBe(true);
    expect(isHighReasoning("xhigh")).toBe(true);
    expect(isHighReasoning("ultra")).toBe(true);
    expect(isFastSpeed("default")).toBe(false);
    expect(isFastSpeed("priority")).toBe(true);
    expect(isFastSpeed("fast")).toBe(true);
  });

  it("重置天数不超过三天时使用绿色状态", () => {
    expect(isResetSoon(null)).toBe(false);
    expect(isResetSoon(4)).toBe(false);
    expect(isResetSoon(3)).toBe(true);
    expect(isResetSoon(0)).toBe(true);
  });
});
