export type ProviderId = "mock" | "recon" | "auto" | "file" | "log" | "loopback" | "mitm";
export type SyncState = "idle" | "syncing" | "connected" | "stale" | "disconnected" | "recon-required" | "unsupported";

export interface StatusField<T> {
  value: T | null;
  source: ProviderId | null;
  observedAt: string | null;
  confidence: number | null;
  stale: boolean;
}

export interface CodexStatus {
  username: StatusField<string>;
  model: StatusField<string>;
  reasoningEffort: StatusField<string>;
  reasoningSpeed: StatusField<number>;
  speedMode: StatusField<string>;
  subscription: StatusField<string>;
  remainingPercent: StatusField<number>;
  resetAt: StatusField<string>;
  clientVersion: StatusField<string>;
  monthlyUsage: StatusField<number>;
  weeklyDurationSeconds: StatusField<number>;
  syncState: SyncState;
  message: string | null;
}

export interface DataProvider {
  getSnapshot(): Promise<CodexStatus>;
  subscribe(callback: (status: CodexStatus) => void): () => void;
  capabilities(): readonly string[];
}
