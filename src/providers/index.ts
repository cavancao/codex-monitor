import { MockProvider } from "./MockProvider";
import { TauriProvider } from "./TauriProvider";
import type { DataProvider, ProviderId } from "./types";

const ids: readonly ProviderId[] = ["mock", "recon", "auto", "file", "log", "loopback", "mitm"];
export function normalizeProviderId(value: string | undefined): ProviderId { return ids.includes(value as ProviderId) ? value as ProviderId : "auto"; }
export function createProvider(value = import.meta.env.VITE_DATA_PROVIDER): DataProvider {
  const id = normalizeProviderId(value);
  return id === "mock" ? new MockProvider() : new TauriProvider(id);
}
export type { CodexStatus, DataProvider, ProviderId, StatusField, SyncState } from "./types";
