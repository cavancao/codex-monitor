<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ChevronDown, ChevronUp, Minus, X } from "lucide-vue-next";
import { isTauriRuntime } from "../lib/runtime";
withDefaults(defineProps<{ dateText: string; state: string; collapsed?: boolean }>(), {
  collapsed: false,
});
defineEmits<{ (event: "toggle-collapse"): void }>();
// 普通浏览器没有 Tauri IPC，窗口句柄必须在确认运行环境后再获取。
const minimize = () => isTauriRuntime() ? getCurrentWindow().minimize() : Promise.resolve();
const hide = () => isTauriRuntime() ? getCurrentWindow().hide() : Promise.resolve();
</script>
<template>
  <header class="titlebar" data-tauri-drag-region>
    <div data-tauri-drag-region><h1 data-tauri-drag-region>Codex运行监测</h1></div>
    <div class="title-actions">
      <span class="state-dot" :class="state" :title="state"/>
      <button
        :aria-label="collapsed ? '展开详情' : '折叠详情'"
        :title="collapsed ? '展开详情' : '折叠详情'"
        @click="$emit('toggle-collapse')"
      >
        <ChevronDown v-if="collapsed" :size="15"/>
        <ChevronUp v-else :size="15"/>
      </button>
      <button aria-label="最小化" @click="minimize"><Minus :size="15"/></button>
      <button aria-label="隐藏到托盘" @click="hide"><X :size="15"/></button>
    </div>
    <time data-tauri-drag-region>{{ dateText }}</time>
  </header>
</template>
