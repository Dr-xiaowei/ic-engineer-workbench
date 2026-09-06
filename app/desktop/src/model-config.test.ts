import { beforeEach, describe, expect, it } from "vitest";
import {
  defaultModelEndpoint,
  loadModelEndpoints,
  modelEndpointsStorageKey,
  validateIntranetEndpoint,
} from "./model-config";

describe("model endpoint configuration", () => {
  beforeEach(() => window.localStorage.clear());

  it("allows local and private-network endpoints", () => {
    expect(validateIntranetEndpoint("http://127.0.0.1:11434/v1")).toBe("");
    expect(validateIntranetEndpoint("https://10.8.0.2/v1")).toBe("");
    expect(validateIntranetEndpoint("http://[fd00::8]:8000/v1")).toBe("");
    expect(validateIntranetEndpoint("https://models.internal/v1")).toBe("");
  });

  it("rejects public endpoints and embedded credentials", () => {
    expect(validateIntranetEndpoint("https://api.example.com/v1")).toMatch(/只允许/);
    expect(validateIntranetEndpoint("http://user:secret@localhost/v1")).toMatch(/用户名或密码/);
  });

  it("falls back when persisted endpoint metadata is malformed", () => {
    window.localStorage.setItem(modelEndpointsStorageKey, JSON.stringify([{ name: "broken" }]));
    expect(loadModelEndpoints()).toEqual([defaultModelEndpoint]);
  });

  it("does not trust a persisted connected state after restart", () => {
    window.localStorage.setItem(
      modelEndpointsStorageKey,
      JSON.stringify([{ ...defaultModelEndpoint, status: "connected" }]),
    );
    expect(loadModelEndpoints()[0].status).toBe("unchecked");
    expect(loadModelEndpoints()[0].statusDetail).toMatch(/重新执行连接检测/);
  });
});
