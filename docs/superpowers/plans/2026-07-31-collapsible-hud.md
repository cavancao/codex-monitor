# HUD 展开/折叠功能 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 HUD 增加可记忆的展开/折叠功能，并在 Tauri 中同步切换窗口高度。

**Architecture:** 使用独立的 `collapse.ts` 管理尺寸常量和 `localStorage` 读写，`App.vue` 只维护界面状态并调用窗口缩放函数。`TitleBar.vue` 提供唯一的折叠入口，详细信息整体通过条件渲染收起。

**Tech Stack:** Vue 3、TypeScript、Tauri 2 Window API、Vitest、Vue Test Utils、CSS

## Global Constraints

- 窗口宽度固定为 `400px`。
- 展开高度固定为 `680px`，折叠高度固定为 `410px`。
- 折叠后仅保留标题区、连接状态和剩余用量圆环。
- 折叠状态保存在本机 `localStorage`，读取失败时默认展开。
- 浏览器预览不得调用 Tauri 窗口接口。
- 不修改数据采集、provider、托盘和自动启动逻辑。

---

### Task 1: 折叠状态与窗口尺寸工具

**Files:**
- Create: `src/lib/collapse.ts`
- Create: `tests/collapse.test.ts`

**Interfaces:**
- Produces: `HUD_WIDTH`、`EXPANDED_HEIGHT`、`COLLAPSED_HEIGHT`
- Produces: `readCollapsed(storage?: Storage): boolean`
- Produces: `writeCollapsed(value: boolean, storage?: Storage): void`
- Produces: `hudSize(collapsed: boolean): { width: number; height: number }`
- Produces: `resizeHudWindow(collapsed: boolean): Promise<void>`

- [ ] **Step 1: 编写失败测试**

```ts
import { beforeEach, describe, expect, it } from "vitest";
import {
  COLLAPSED_HEIGHT,
  EXPANDED_HEIGHT,
  hudSize,
  readCollapsed,
  writeCollapsed,
} from "../src/lib/collapse";

describe("HUD 折叠状态", () => {
  beforeEach(() => localStorage.clear());

  it("没有保存状态时默认展开", () => {
    expect(readCollapsed()).toBe(false);
  });

  it("保存并恢复折叠状态", () => {
    writeCollapsed(true);
    expect(readCollapsed()).toBe(true);
    writeCollapsed(false);
    expect(readCollapsed()).toBe(false);
  });

  it("使用固定宽度和两种窗口高度", () => {
    expect(hudSize(false)).toEqual({ width: 400, height: EXPANDED_HEIGHT });
    expect(hudSize(true)).toEqual({ width: 400, height: COLLAPSED_HEIGHT });
  });
});
```

- [ ] **Step 2: 运行测试并确认因模块不存在而失败**

Run: `npm.cmd test -- --run tests/collapse.test.ts`

Expected: FAIL，提示无法解析 `../src/lib/collapse`。

- [ ] **Step 3: 编写最小实现**

```ts
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { isTauriRuntime } from "./runtime";

const STORAGE_KEY = "codex-monitor:collapsed";
export const HUD_WIDTH = 400;
export const EXPANDED_HEIGHT = 680;
export const COLLAPSED_HEIGHT = 410;

export function readCollapsed(storage: Storage = localStorage): boolean {
  try {
    return storage.getItem(STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

export function writeCollapsed(value: boolean, storage: Storage = localStorage): void {
  try {
    storage.setItem(STORAGE_KEY, String(value));
  } catch {
    // 本地存储不可用不应阻止界面切换。
  }
}

export function hudSize(collapsed: boolean) {
  return {
    width: HUD_WIDTH,
    height: collapsed ? COLLAPSED_HEIGHT : EXPANDED_HEIGHT,
  };
}

export async function resizeHudWindow(collapsed: boolean): Promise<void> {
  if (!isTauriRuntime()) return;
  const size = hudSize(collapsed);
  await getCurrentWindow().setSize(new LogicalSize(size.width, size.height));
}
```

- [ ] **Step 4: 运行测试并确认通过**

Run: `npm.cmd test -- --run tests/collapse.test.ts`

Expected: 3 tests PASS。

- [ ] **Step 5: 提交**

```powershell
git add -- src/lib/collapse.ts tests/collapse.test.ts
git commit -m "Add persisted HUD collapse state"
```

### Task 2: 标题栏开关与详细区域收缩

**Files:**
- Modify: `src/components/TitleBar.vue`
- Modify: `src/App.vue`
- Modify: `src/styles/tech.css`
- Modify: `tests/components.test.ts`

**Interfaces:**
- Consumes: `readCollapsed()`、`writeCollapsed()`、`resizeHudWindow()`
- `TitleBar` consumes prop: `collapsed: boolean`
- `TitleBar` produces event: `toggle-collapse`

- [ ] **Step 1: 编写失败的组件测试**

在 `tests/components.test.ts` 添加：

```ts
import { beforeEach } from "vitest";

beforeEach(() => localStorage.clear());

it("折叠后仅保留标题、连接状态和剩余用量圆环", async () => {
  const wrapper = mount(App);
  await wrapper.get('[aria-label="折叠详情"]').trigger("click");

  expect(wrapper.find(".titlebar").exists()).toBe(true);
  expect(wrapper.find(".notice").exists()).toBe(true);
  expect(wrapper.find(".ring-card").exists()).toBe(true);
  expect(wrapper.find(".info-grid").exists()).toBe(false);
  expect(wrapper.find(".reset").exists()).toBe(false);
  expect(wrapper.find("footer").exists()).toBe(false);
  expect(wrapper.get('[aria-label="展开详情"]').exists()).toBe(true);
  wrapper.unmount();
});

it("挂载时恢复上次折叠状态", () => {
  localStorage.setItem("codex-monitor:collapsed", "true");
  const wrapper = mount(App);
  expect(wrapper.find(".shell").classes()).toContain("collapsed");
  expect(wrapper.find(".info-grid").exists()).toBe(false);
  wrapper.unmount();
});
```

- [ ] **Step 2: 运行测试并确认缺少按钮和折叠行为**

Run: `npm.cmd test -- --run tests/components.test.ts`

Expected: FAIL，找不到 `aria-label="折叠详情"`。

- [ ] **Step 3: 实现标题栏开关**

在 `TitleBar.vue`：

```ts
import { ChevronDown, ChevronUp, Minus, X } from "lucide-vue-next";
defineProps<{ dateText: string; state: string; collapsed: boolean }>();
defineEmits<{ (event: "toggle-collapse"): void }>();
```

在窗口按钮前增加：

```vue
<button
  :aria-label="collapsed ? '展开详情' : '折叠详情'"
  :title="collapsed ? '展开详情' : '折叠详情'"
  @click="$emit('toggle-collapse')"
>
  <ChevronDown v-if="collapsed" :size="15"/>
  <ChevronUp v-else :size="15"/>
</button>
```

- [ ] **Step 4: 实现 App 状态、持久化与条件渲染**

在 `App.vue` 初始化：

```ts
import { watch } from "vue";
import { readCollapsed, resizeHudWindow, writeCollapsed } from "./lib/collapse";

const collapsed = ref(readCollapsed());
watch(collapsed, value => {
  writeCollapsed(value);
  void resizeHudWindow(value).catch(() => undefined);
}, { immediate: true });
```

将标题栏改为：

```vue
<TitleBar
  :date-text="dateText"
  :state="status.syncState"
  :collapsed="collapsed"
  @toggle-collapse="collapsed = !collapsed"
/>
```

为 `.shell` 增加 `collapsed` class，并用一个 `Transition` 包住信息卡、重置区和底部工具栏；`v-if="!collapsed"` 时才渲染这些内容。

- [ ] **Step 5: 添加收缩过渡样式**

在 `tech.css` 添加：

```css
.details-enter-active,.details-leave-active{transition:opacity .16s ease,transform .16s ease}
.details-enter-from,.details-leave-to{opacity:0;transform:translateY(-6px)}
.motion-off .details-enter-active,.motion-off .details-leave-active{transition:none}
```

- [ ] **Step 6: 运行组件测试并确认通过**

Run: `npm.cmd test -- --run tests/components.test.ts`

Expected: 全部 PASS。

- [ ] **Step 7: 提交**

```powershell
git add -- src/components/TitleBar.vue src/App.vue src/styles/tech.css tests/components.test.ts
git commit -m "Add collapsible HUD layout"
```

### Task 3: Tauri 权限与完整验证

**Files:**
- Modify: `src-tauri/capabilities/default.json`

**Interfaces:**
- Consumes: Tauri permission `core:window:allow-set-size`

- [ ] **Step 1: 编写失败的权限测试**

在 `tests/collapse.test.ts` 添加：

```ts
import capability from "../src-tauri/capabilities/default.json";

it("允许主窗口切换尺寸", () => {
  expect(capability.permissions).toContain("core:window:allow-set-size");
});
```

- [ ] **Step 2: 运行测试并确认权限缺失**

Run: `npm.cmd test -- --run tests/collapse.test.ts`

Expected: FAIL，权限数组不包含 `core:window:allow-set-size`。

- [ ] **Step 3: 添加最小权限**

在 `src-tauri/capabilities/default.json` 的 `permissions` 中加入：

```json
"core:window:allow-set-size"
```

- [ ] **Step 4: 运行完整验证**

Run:

```powershell
npm.cmd test -- --run
npm.cmd run build
```

Expected: 所有测试通过，TypeScript 检查和 Vite production build 退出码为 0。

- [ ] **Step 5: 提交**

```powershell
git add -- src-tauri/capabilities/default.json tests/collapse.test.ts
git commit -m "Allow HUD window resizing"
```
