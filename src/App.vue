<script setup lang="ts">
import { computed, onErrorCaptured, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Activity, RefreshCw } from "lucide-vue-next";
import TitleBar from "./components/TitleBar.vue";
import RingProgress from "./components/RingProgress.vue";
import InfoCard from "./components/InfoCard.vue";
import NeonFrame from "./components/NeonFrame.vue";
import { useStatus } from "./composables/useStatus";
import { isTauriRuntime } from "./lib/runtime";
import { formatDateTime, formatHeaderDateTime, isFastSpeed, isHighReasoning, isResetSoon, reasoningLabel, speedLabel } from "./lib/format";
import { isFailureSyncState } from "./providers/status";
import { readCollapsed, resizeHudWindow, writeCollapsed } from "./lib/collapse";

const { status, now, resetDays, refresh } = useStatus();
const motion = ref(true), error = ref<string | null>(null);
const collapsed = ref(readCollapsed());
watch(collapsed, value => {
  writeCollapsed(value);
  void resizeHudWindow(value).catch(() => undefined);
}, { immediate: true });
onErrorCaptured(e => { error.value = e instanceof Error ? e.message : "界面发生异常"; return false; });
const text = (value: string | number | null, suffix = "") => value === null ? "--" : `${value}${suffix}`;
const dateText = computed(() => formatHeaderDateTime(now.value));
const subscription = computed(() => import.meta.env.VITE_SUBSCRIPTION_OVERRIDE?.trim() || text(status.value.subscription.value));
const runRecon = async () => {
  if (!isTauriRuntime()) { status.value.message = "浏览器预览不能执行本机侦察，请使用 npm run tauri dev"; return; }
  status.value.syncState = "syncing";
  try { await invoke("run_recon"); await refresh(); } catch(e) { error.value = e instanceof Error ? e.message : String(e); }
};
</script>
<template>
  <main class="shell" :class="{ 'motion-off': !motion, collapsed }">
    <NeonFrame/>
    <div class="grid-bg"/><div class="scanline"/>
    <TitleBar
      :date-text="dateText"
      :state="status.syncState"
      :collapsed="collapsed"
      @toggle-collapse="collapsed = !collapsed"
    />
    <div v-if="error" class="error glass">{{ error }} <button @click="error=null">关闭</button></div>
    <div v-if="status.message" class="notice" :class="{ 'notice-success': status.syncState === 'connected', 'notice-failure': isFailureSyncState(status.syncState) }"><Activity :size="13"/>{{ status.message }}</div>
    <RingProgress :value="status.remainingPercent.value" :motion="motion"/>
    <Transition name="details">
      <div v-if="!collapsed" class="details">
        <section class="info-grid">
          <InfoCard label="当前模型" :value="text(status.model.value)" accent/>
          <InfoCard label="订阅套餐" :value="subscription" success/>
          <InfoCard label="推理强度" :value="reasoningLabel(status.reasoningEffort.value)" :danger="isHighReasoning(status.reasoningEffort.value)"/>
          <InfoCard label="速度" :value="speedLabel(status.speedMode.value)" :danger="isFastSpeed(status.speedMode.value)"/>
          <InfoCard label="用户名称" :value="text(status.username.value)"/>
          <InfoCard label="客户端版本" :value="text(status.clientVersion.value)"/>
        </section>
        <section class="reset glass">
          <div><span>距离重置</span><b class="days-value" :class="{ 'reset-soon': isResetSoon(resetDays) }"><strong>{{ resetDays ?? '--' }}</strong><i v-if="resetDays !== null">天</i></b></div>
          <div class="reset-target"><span>下次重置</span><strong>{{ status.resetAt.value ? formatDateTime(new Date(status.resetAt.value), false) : '--' }}</strong></div>
        </section>
        <footer><button @click="runRecon"><RefreshCw :size="14"/>刷新</button><button @click="motion=!motion">动效 {{ motion?'开':'关' }}</button><span>v0.1.0</span></footer>
      </div>
    </Transition>
  </main>
</template>
