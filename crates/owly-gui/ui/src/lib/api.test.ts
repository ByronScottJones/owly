import { describe, expect, it } from "vitest";
import { errorText, isAppError } from "./api";

describe("errorText", () => {
  it("shows an app error's message", () => {
    expect(
      errorText({ kind: "busy", message: "another client has the radio" }),
    ).toBe("another client has the radio");
  });

  it("shows an Error's message", () => {
    expect(errorText(new Error("boom"))).toBe("boom");
  });

  it("falls back to a string for anything else", () => {
    expect(errorText(42)).toBe("42");
    expect(errorText(null)).toBe("null");
  });
});

describe("isAppError", () => {
  it("needs both fields", () => {
    expect(isAppError({ kind: "io" })).toBe(false);
    expect(isAppError({ kind: "io", message: "x" })).toBe(true);
  });
});
