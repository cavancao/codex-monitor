import type { CodexStatus, DataProvider, StatusField } from "./types";

const field = <T>(value: T): StatusField<T> => ({ value, source: "mock", observedAt: new Date().toISOString(), confidence: 1, stale: false });

export class MockProvider implements DataProvider {
  private timer: number | undefined;
  private remaining = 72;
  async getSnapshot(): Promise<CodexStatus> {
    this.remaining = Math.max(0, Math.min(100, this.remaining + (Math.random() - 0.58) * 1.4));
    return {
      username: field("演示用户"), model: field("gpt-5.6-sol"), reasoningEffort: field("极高"),
      reasoningSpeed: field(Number((42 + Math.random() * 8).toFixed(1))), subscription: field("Pro"),
      speedMode: field("default"),
      remainingPercent: field(Number(this.remaining.toFixed(1))), resetAt: field(new Date(Date.now() + 2.5 * 86400000).toISOString()),
      clientVersion: field("0.1.0"), monthlyUsage: field(128400), weeklyDurationSeconds: field(64320),
      syncState: "connected", message: "模拟数据"
    };
  }
  subscribe(callback: (status: CodexStatus) => void): () => void {
    void this.getSnapshot().then(callback);
    this.timer = window.setInterval(() => void this.getSnapshot().then(callback), 2000);
    return () => { if (this.timer !== undefined) window.clearInterval(this.timer); };
  }
  capabilities(): readonly string[] { return ["all", "mock"]; }
}
