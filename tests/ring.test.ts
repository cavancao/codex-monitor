import { describe, expect, it } from "vitest";
import { ringMetrics } from "../src/lib/ring";

describe("ringMetrics", () => {
  it("限制百分比并计算圆环偏移", () => {
    expect(ringMetrics(25, 54)).toEqual({
      percent: 25,
      circumference: 2 * Math.PI * 54,
      offset: 2 * Math.PI * 54 * 0.75
    });
    expect(ringMetrics(140, 54).percent).toBe(100);
    expect(ringMetrics(-2, 54).percent).toBe(0);
  });

  it("空值保持空状态", () => {
    expect(ringMetrics(null, 54).percent).toBeNull();
  });
});
