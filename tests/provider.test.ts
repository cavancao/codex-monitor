import { describe, expect, it } from "vitest";
import { normalizeProviderId } from "../src/providers";

describe("provider 选择", () => {
  it("只接受白名单且默认 auto", () => {
    expect(normalizeProviderId("mock")).toBe("mock");
    expect(normalizeProviderId("C:\\fixed\\path")).toBe("auto");
    expect(normalizeProviderId(undefined)).toBe("auto");
  });
});
