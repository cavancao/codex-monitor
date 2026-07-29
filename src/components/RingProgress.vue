<script setup lang="ts">
import { computed } from "vue";
import { ringMetrics } from "../lib/ring";
const props = withDefaults(defineProps<{ value: number | null; motion?: boolean }>(), { motion: true });
const radius = 76;
const metrics = computed(() => ringMetrics(props.value, radius));
</script>
<template>
  <section class="ring-card glass">
    <svg viewBox="0 0 200 200" role="img" :aria-label="value === null ? '剩余用量未知' : `剩余用量 ${metrics.percent}%`">
      <defs>
        <linearGradient id="neon" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#22d3ee"/><stop offset=".52" stop-color="#a855f7"/><stop offset="1" stop-color="#ec4899"/></linearGradient>
        <linearGradient id="neon-flow" gradientUnits="userSpaceOnUse" x1="24" y1="24" x2="176" y2="176">
          <stop stop-color="#22d3ee"/><stop offset=".28" stop-color="#60a5fa"/><stop offset=".55" stop-color="#a855f7"/><stop offset=".78" stop-color="#ec4899"/><stop offset="1" stop-color="#22d3ee"/>
          <animateTransform v-if="motion" attributeName="gradientTransform" type="rotate" from="0 100 100" to="360 100 100" dur="2.2s" repeatCount="indefinite"/>
        </linearGradient>
        <filter id="glow"><feGaussianBlur stdDeviation="3" result="blur"/><feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
        <mask id="progress-mask" maskUnits="userSpaceOnUse" x="0" y="0" width="200" height="200">
          <circle cx="100" cy="100" :r="radius" fill="none" stroke="#fff" stroke-width="10" stroke-linecap="round" :stroke-dasharray="metrics.circumference" :stroke-dashoffset="metrics.offset"/>
        </mask>
      </defs>
      <circle class="track" cx="100" cy="100" :r="radius"/>
      <circle v-if="value !== null" class="progress" cx="100" cy="100" :r="radius" :stroke-dasharray="metrics.circumference" :stroke-dashoffset="metrics.offset"/>
      <circle v-if="value !== null" class="energy-flow" cx="100" cy="100" :r="radius" mask="url(#progress-mask)"/>
    </svg>
    <div class="ring-value"><strong>{{ metrics.percent === null ? '--' : Math.round(metrics.percent) }}</strong><span v-if="metrics.percent !== null">%</span><small>剩余用量</small></div>
    <div class="legend"><span><i class="used"/>已使用 {{ metrics.percent === null ? '--' : Math.round(100-metrics.percent)+'%' }}</span><span><i class="remain"/>剩余 {{ metrics.percent === null ? '--' : Math.round(metrics.percent)+'%' }}</span></div>
  </section>
</template>
