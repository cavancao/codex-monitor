import { computed, onMounted, onUnmounted, ref } from "vue";
import { createProvider } from "../providers";
import { emptyStatus, mergeStatus } from "../providers/status";

export function useStatus() {
  const provider = createProvider();
  const status = ref(emptyStatus());
  const now = ref(new Date());
  let stop: (() => void) | undefined;
  let clock: number | undefined;
  const apply = (next: ReturnType<typeof emptyStatus>) => { status.value = mergeStatus(status.value, next); };
  const refresh = async () => {
    try { apply(await provider.getSnapshot()); }
    catch (error) { apply({ ...emptyStatus(error instanceof Error ? error.message : "采集失败"), syncState: "disconnected" }); }
  };
  const resetDays = computed(() => {
    const raw = status.value.resetAt.value;
    if (!raw) return null;
    const diff = new Date(raw).getTime() - now.value.getTime();
    if (!Number.isFinite(diff)) return null;
    if (diff <= 0) return 0;
    return Math.floor(diff / 86400000);
  });
  onMounted(() => {
    void refresh(); stop = provider.subscribe(apply);
    clock = window.setInterval(() => { if (!document.hidden) now.value = new Date(); }, 1000);
    document.addEventListener("visibilitychange", refresh);
  });
  onUnmounted(() => { stop?.(); if (clock) clearInterval(clock); document.removeEventListener("visibilitychange", refresh); });
  return { status, now, resetDays, refresh };
}
