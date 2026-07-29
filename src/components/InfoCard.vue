<script setup lang="ts">defineProps<{ label: string; value: string; accent?: boolean; danger?: boolean; success?: boolean }>();</script>
<template><div class="info-card glass"><span>{{ label }}</span><strong :class="{ accent, danger, success }">{{ value || '--' }}</strong></div></template>
