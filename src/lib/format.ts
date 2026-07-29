const pad = (value: number) => String(value).padStart(2, "0");

export function formatDateTime(date: Date, seconds: boolean): string {
  if (!Number.isFinite(date.getTime())) return "--";
  const day = `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
  const time = `${pad(date.getHours())}:${pad(date.getMinutes())}${seconds ? `:${pad(date.getSeconds())}` : ""}`;
  return `${day} ${time}`;
}

const weekdays = ["星期日", "星期一", "星期二", "星期三", "星期四", "星期五", "星期六"] as const;

export function formatHeaderDateTime(date: Date): string {
  if (!Number.isFinite(date.getTime())) return "--";
  const day = `${date.getFullYear()}年${pad(date.getMonth() + 1)}月${pad(date.getDate())}日`;
  const time = `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
  return `${day} ${time} ${weekdays[date.getDay()]}`;
}

const reasoningNames: Record<string, string> = {
  minimal: "轻度", low: "轻度", medium: "中", high: "高", xhigh: "极高", max: "极速", ultra: "极速"
};

export function reasoningLabel(value: string | null): string {
  if (!value) return "--";
  return reasoningNames[value.toLowerCase()] ?? value;
}

export function speedLabel(value: string | null): string {
  if (!value) return "--";
  const normalized = value.toLowerCase();
  if (normalized === "default" || normalized === "standard") return "标准";
  if (normalized === "priority" || normalized === "fast") return "快速";
  return value;
}

export function isHighReasoning(value: string | null): boolean {
  return value !== null && ["high", "xhigh", "max", "ultra"].includes(value.toLowerCase());
}

export function isFastSpeed(value: string | null): boolean {
  return value !== null && ["priority", "fast"].includes(value.toLowerCase());
}

export function isResetSoon(days: number | null): boolean {
  return days !== null && days >= 0 && days <= 3;
}
