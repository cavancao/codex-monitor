import { describe, expect, it } from "vitest";
import { emptyStatus, isFailureSyncState, mergeStatus } from "../src/providers/status";

describe("状态降级", () => {
  it("首次无可信来源时返回 null 而不是 mock", () => {
    const status = emptyStatus("未发现可信数据源");
    expect(status.remainingPercent.value).toBeNull();
    expect(status.syncState).toBe("recon-required");
  });

  it("采集失败时保留上次可信值并标记 stale", () => {
    const before = emptyStatus();
    before.model = { value: "gpt-5", source: "file", observedAt: "2026-01-01T00:00:00Z", confidence: 0.9, stale: false };
    const after = mergeStatus(before, emptyStatus("断开"));
    expect(after.model.value).toBe("gpt-5");
    expect(after.model.stale).toBe(true);
  });

  it("将所有失败同步状态识别为红色状态", () => {
    expect(isFailureSyncState("disconnected")).toBe(true);
    expect(isFailureSyncState("stale")).toBe(true);
    expect(isFailureSyncState("recon-required")).toBe(true);
    expect(isFailureSyncState("unsupported")).toBe(true);
    expect(isFailureSyncState("connected")).toBe(false);
    expect(isFailureSyncState("syncing")).toBe(false);
  });
});
