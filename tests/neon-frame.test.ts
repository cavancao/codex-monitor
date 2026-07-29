import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import NeonFrame from "../src/components/NeonFrame.vue";

describe("窗口霓虹流光", () => {
  it("使用不拦截交互的 SVG 边框轨迹", () => {
    const wrapper = mount(NeonFrame);
    expect(wrapper.find("svg.neon-frame").attributes("aria-hidden")).toBe("true");
    expect(wrapper.findAll("rect.neon-frame-flow")).toHaveLength(2);
  });
});
