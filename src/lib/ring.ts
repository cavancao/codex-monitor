export interface RingMetrics { percent: number | null; circumference: number; offset: number; }
export function ringMetrics(value: number | null, radius: number): RingMetrics {
  const circumference = 2 * Math.PI * radius;
  const percent = value === null || !Number.isFinite(value) ? null : Math.max(0, Math.min(100, value));
  return { percent, circumference, offset: circumference * (1 - (percent ?? 0) / 100) };
}
