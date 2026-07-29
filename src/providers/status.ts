import type { CodexStatus, StatusField, SyncState } from "./types";

const blank = <T>(): StatusField<T> => ({ value: null, source: null, observedAt: null, confidence: null, stale: false });

export function isFailureSyncState(state: SyncState): boolean {
  return ["disconnected", "stale", "recon-required", "unsupported"].includes(state);
}

export function emptyStatus(message: string | null = null): CodexStatus {
  return {
    username: blank(), model: blank(), reasoningEffort: blank(), reasoningSpeed: blank(), speedMode: blank(),
    subscription: blank(), remainingPercent: blank(), resetAt: blank(), clientVersion: blank(),
    monthlyUsage: blank(), weeklyDurationSeconds: blank(),
    syncState: message ? "recon-required" : "idle", message
  };
}

export function mergeStatus(previous: CodexStatus, incoming: CodexStatus): CodexStatus {
  const keys = ["username", "model", "reasoningEffort", "reasoningSpeed", "speedMode", "subscription", "remainingPercent", "resetAt", "clientVersion", "monthlyUsage", "weeklyDurationSeconds"] as const;
  const merged = { ...incoming };
  for (const key of keys) {
    if (incoming[key].value === null && previous[key].value !== null) merged[key] = { ...previous[key], stale: true } as never;
  }
  return merged;
}
