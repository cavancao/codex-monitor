import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it } from "vitest";
import RingProgress from "../src/components/RingProgress.vue";
import InfoCard from "../src/components/InfoCard.vue";
import TitleBar from "../src/components/TitleBar.vue";
import App from "../src/App.vue";

beforeEach(() => localStorage.clear());

describe("HUD 组件降级", () => {
  it("圆环空值显示 -- 且不绘制伪造进度", () => {
    const wrapper = mount(RingProgress, { props: { value: null } });
    expect(wrapper.text()).toContain("--");
    expect(wrapper.find("circle.progress").exists()).toBe(false);
  });

  it("有剩余用量时渲染受遮罩约束的连续彩色灯带", () => {
    const wrapper = mount(RingProgress, { props: { value: 87, motion: true } });
    expect(wrapper.find("mask#progress-mask").exists()).toBe(true);
    expect(wrapper.find("circle.energy-flow").attributes("mask")).toBe("url(#progress-mask)");
    expect(wrapper.find("#neon-flow animateTransform").exists()).toBe(true);
    expect(wrapper.find("circle.energy-arc").exists()).toBe(false);
    expect(wrapper.find("circle.energy-endpoint").exists()).toBe(false);
    expect(wrapper.find("circle.endpoint").exists()).toBe(false);
    expect(wrapper.find("#progress-mask circle").attributes("stroke-linecap")).toBe("round");
  });

  it("关闭动效时不创建圆环渐变动画", () => {
    const wrapper = mount(RingProgress, { props: { value: 87, motion: false } });
    expect(wrapper.find("#neon-flow animateTransform").exists()).toBe(false);
  });

  it("信息卡无值显示 --", () => {
    expect(mount(InfoCard, { props: { label: "当前模型", value: "" } }).text()).toContain("--");
  });

  it("信息卡支持红色数值状态", () => {
    const wrapper = mount(InfoCard, { props: { label: "推理强度", value: "高", danger: true } });
    expect(wrapper.find("strong").classes()).toContain("danger");
  });

  it("信息卡支持绿色数值状态", () => {
    const wrapper = mount(InfoCard, { props: { label: "订阅套餐", value: "Plus", success: true } });
    expect(wrapper.find("strong").classes()).toContain("success");
  });

  it("普通浏览器可以挂载标题栏而不访问 Tauri metadata", () => {
    expect(() => mount(TitleBar, { props: { dateText: "07/29 21:40:00", state: "unsupported" } })).not.toThrow();
  });

  it("标题栏只显示中文运行监测标题", () => {
    const wrapper = mount(TitleBar, { props: { dateText: "2026-07-29 21:40:00", state: "connected" } });
    expect(wrapper.text()).toContain("Codex运行监测");
    expect(wrapper.text()).not.toContain("SYSTEM TELEMETRY");
    expect(wrapper.text()).not.toContain("Codex运行情况");
  });

  it("侦察操作使用简洁的刷新文案", () => {
    const wrapper = mount(App);
    expect(wrapper.text()).toContain("刷新");
    expect(wrapper.text()).not.toContain("重新侦察");
    wrapper.unmount();
  });

  it("套餐位于第一行右侧且推理强度与速度位于第二行", () => {
    const wrapper = mount(App);
    const labels = wrapper.findAll(".info-card > span").map(node => node.text());
    expect(labels).toEqual(["当前模型", "订阅套餐", "推理强度", "速度", "用户名称", "客户端版本"]);
    wrapper.unmount();
  });

  it("折叠后仅保留标题、连接状态和剩余用量圆环", async () => {
    const wrapper = mount(App);
    await wrapper.get('[aria-label="折叠详情"]').trigger("click");

    expect(wrapper.find(".titlebar").exists()).toBe(true);
    expect(wrapper.find(".notice").exists()).toBe(true);
    expect(wrapper.find(".ring-card").exists()).toBe(true);
    expect(wrapper.find(".info-grid").exists()).toBe(false);
    expect(wrapper.find(".reset").exists()).toBe(false);
    expect(wrapper.find("footer").exists()).toBe(false);
    expect(wrapper.find('[aria-label="展开详情"]').exists()).toBe(true);
    wrapper.unmount();
  });

  it("挂载时恢复上次折叠状态", () => {
    localStorage.setItem("codex-monitor:collapsed", "true");
    const wrapper = mount(App);

    expect(wrapper.find(".shell").classes()).toContain("collapsed");
    expect(wrapper.find(".info-grid").exists()).toBe(false);
    wrapper.unmount();
  });
});
