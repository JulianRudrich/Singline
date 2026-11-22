import { describe, expect, it } from "vitest";
import { radioMove } from "./radio";

describe("radioMove", () => {
  it("moves with the arrow keys and wraps round the ends", () => {
    expect(radioMove("ArrowDown", 0, 4)).toBe(1);
    expect(radioMove("ArrowRight", 3, 4)).toBe(0);
    expect(radioMove("ArrowUp", 0, 4)).toBe(3);
    expect(radioMove("ArrowLeft", 2, 4)).toBe(1);
  });

  it("goes to the first and last option with Home and End", () => {
    expect(radioMove("Home", 2, 4)).toBe(0);
    expect(radioMove("End", 1, 4)).toBe(3);
  });

  it("skips options that cannot be chosen", () => {
    const enabled = (index: number) => index !== 5;
    expect(radioMove("ArrowRight", 4, 6, enabled)).toBe(0);
    expect(radioMove("End", 0, 6, enabled)).toBe(4);
    expect(radioMove("ArrowLeft", 0, 6, enabled)).toBe(4);
  });

  it("ignores other keys and groups with nothing else to choose", () => {
    expect(radioMove("Enter", 1, 4)).toBeNull();
    expect(radioMove("ArrowDown", 0, 1)).toBeNull();
    expect(radioMove("ArrowDown", 0, 0)).toBeNull();
  });
});
