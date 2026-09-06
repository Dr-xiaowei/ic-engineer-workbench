import { describe, expect, it } from "vitest";
import { calculateExpression } from "./calculator";

describe("safe scientific calculator", () => {
  it("supports precedence, powers, constants and scientific functions", () => {
    expect(calculateExpression("2 + 3 * 4", "rad")).toBe(14);
    expect(calculateExpression("2^3^2", "rad")).toBe(512);
    expect(calculateExpression("-2^2", "rad")).toBe(-4);
    expect(calculateExpression("2^-2", "rad")).toBe(0.25);
    expect(calculateExpression("sin(30)^2 + cos(30)^2", "deg")).toBeCloseTo(1, 12);
    expect(calculateExpression("ln(e) + sqrt(9)", "rad")).toBeCloseTo(4, 12);
  });

  it("rejects code, division by zero, unknown functions and non-finite results", () => {
    expect(() => calculateExpression("globalThis.alert(1)", "rad")).toThrow();
    expect(() => calculateExpression("1 / 0", "rad")).toThrow(/除以零/);
    expect(() => calculateExpression("unknown(1)", "rad")).toThrow(/未知函数/);
    expect(() => calculateExpression("sqrt(-1)", "rad")).toThrow(/有限数值/);
  });
});
