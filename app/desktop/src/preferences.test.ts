import { beforeEach, describe, expect, it } from "vitest";
import {
  defaultPreferences,
  loadPreferences,
  preferencesStorageKey,
} from "./preferences";

describe("preferences", () => {
  beforeEach(() => window.localStorage.clear());

  it("falls back safely when stored data is invalid", () => {
    window.localStorage.setItem(preferencesStorageKey, "not-json");
    expect(loadPreferences()).toEqual(defaultPreferences);
  });

  it("validates and normalizes stored preferences", () => {
    window.localStorage.setItem(
      preferencesStorageKey,
      JSON.stringify({
        avatarDataUrl: 123,
        displayName: "  本地工程师  ",
        theme: "light",
      }),
    );

    expect(loadPreferences()).toEqual({
      avatarDataUrl: "",
      displayName: "本地工程师",
      theme: "light",
    });
  });
});
