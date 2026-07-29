<template>
  <svg class="neon-frame" viewBox="0 0 400 720" preserveAspectRatio="none" aria-hidden="true">
    <defs>
      <linearGradient id="frame-neon" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#22d3ee"/>
        <stop offset=".5" stop-color="#a855f7"/>
        <stop offset="1" stop-color="#ec4899"/>
      </linearGradient>
      <filter id="frame-glow" x="-40%" y="-40%" width="180%" height="180%">
        <feGaussianBlur stdDeviation="2.2" result="blur"/>
        <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
      </filter>
    </defs>
    <rect class="neon-frame-flow flow-primary" x="2" y="2" width="396" height="716" rx="16" pathLength="100"/>
    <rect class="neon-frame-flow flow-secondary" x="2" y="2" width="396" height="716" rx="16" pathLength="100"/>
  </svg>
</template>
