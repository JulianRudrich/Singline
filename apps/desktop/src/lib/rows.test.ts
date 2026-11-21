import { describe, expect, it } from "vitest";
import { RowMotion, type RowTarget } from "./rows";

const PITCH = 30;

/** Rows for lines `from` to `to`, stacked from y = 0, the middle one bright. */
function targets(from: number, to: number, active: number): RowTarget<null>[] {
  const rows = [];
  for (let index = from; index <= to; index += 1) {
    rows.push({ index, y: (index - from) * PITCH, scale: 1, opacity: index === active ? 1 : 0.4, pitch: PITCH, data: null });
  }
  return rows;
}

/** Runs the springs `seconds` on at 60 frames a second; calls `each` after every frame. */
function run(motion: RowMotion<null>, seconds: number, each?: () => void) {
  for (let frame = 0; frame < Math.round(seconds * 60); frame += 1) {
    motion.step(1 / 60);
    each?.();
  }
}

describe("RowMotion", () => {
  it("shows the first rows at once when they appear", () => {
    const motion = new RowMotion<null>();
    motion.update(targets(0, 2, 1), { appear: true });
    expect(motion.list().map((row) => [row.index, row.y.value, row.opacity.value])).toEqual([
      [0, 0, 0.4],
      [1, 30, 1],
      [2, 60, 0.4],
    ]);
  });

  it("scrolls one line on: the old row moves on out, the new one comes in from below", () => {
    const motion = new RowMotion<null>();
    motion.update(targets(0, 2, 1), { appear: true });
    motion.update(targets(1, 3, 2), { moved: 1 });
    const leaving = motion.get(0);
    const coming = motion.get(3);
    expect(leaving?.leaving).toBe(true);
    expect(leaving?.y.target).toBe(-PITCH);
    expect(coming?.y.value).toBe(60 + PITCH);
    expect(coming?.opacity.value).toBe(0);
    run(motion, 0.15);
    // Everything moves up together.
    expect(motion.get(1)?.y.value).toBeLessThan(30);
    expect(motion.get(3)?.y.value).toBeLessThan(90);
    expect(motion.get(0)?.y.value).toBeLessThan(0);
    run(motion, 2);
    expect(motion.get(0)).toBeUndefined();
    expect(motion.list().map((row) => [row.index, row.y.value, row.opacity.value])).toEqual([
      [1, 0, 0.4],
      [2, 30, 1],
      [3, 60, 0.4],
    ]);
  });

  it("never lets two lines overlap in a one-line layout", () => {
    const motion = new RowMotion<null>();
    motion.update(targets(4, 4, 4), { appear: true });
    motion.update(targets(5, 5, 5), { moved: 1 });
    let closest = Infinity;
    run(motion, 1, () => {
      const old = motion.get(4);
      const next = motion.get(5);
      if (old && next && old.opacity.value > 0.02) closest = Math.min(closest, Math.abs(next.y.value - old.y.value));
    });
    expect(closest).toBeGreaterThan(PITCH * 0.95);
  });

  it("scrolls the other way when the lyrics go back", () => {
    const motion = new RowMotion<null>();
    motion.update(targets(3, 5, 4), { appear: true });
    motion.update(targets(2, 4, 3), { moved: -1 });
    expect(motion.get(2)?.y.value).toBe(-PITCH);
    expect(motion.get(5)?.y.target).toBe(60 + PITCH);
  });

  it("scrolls a few lines as one while some rows stay", () => {
    const motion = new RowMotion<null>();
    motion.update(targets(0, 2, 1), { appear: true });
    motion.update(targets(2, 4, 3), { moved: 2 });
    // The rows that go and the rows that come travel as far as the one that stays.
    expect(motion.get(1)?.y.target).toBe(30 - 2 * PITCH);
    expect(motion.get(4)?.y.value).toBe(60 + 2 * PITCH);
    expect(motion.get(2)?.y.value).toBe(60);
    expect(motion.get(2)?.y.target).toBe(0);
  });

  it("cuts to a jump past every row on show: the old rows go at once, the new ones rise in", () => {
    const motion = new RowMotion<null>();
    motion.update(targets(0, 2, 1), { appear: true });
    motion.update(targets(20, 22, 21), { moved: 20 });
    expect(motion.list().map((row) => [row.index, row.y.value, row.opacity.value])).toEqual([
      [20, PITCH, 0],
      [21, 2 * PITCH, 0],
      [22, 3 * PITCH, 0],
    ]);
    run(motion, 2);
    expect(motion.list().map((row) => [row.index, row.y.value, row.opacity.value])).toEqual([
      [20, 0, 0.4],
      [21, 30, 1],
      [22, 60, 0.4],
    ]);
  });

  it("puts rows in place at once for a new size, and fades out the ones no longer shown", () => {
    const motion = new RowMotion<null>();
    motion.update(targets(0, 4, 2), { appear: true });
    const narrower = targets(1, 3, 2).map((row) => ({ ...row, y: row.y + 7 }));
    motion.update(narrower);
    expect(motion.get(2)?.y.value).toBe(37);
    expect(motion.get(0)?.leaving).toBe(true);
    expect(motion.get(0)?.y.value).toBe(0);
    run(motion, 1);
    expect(motion.list().map((row) => row.index)).toEqual([1, 2, 3]);
  });

  it("only fades with reduced motion, and drops a row that goes at once", () => {
    const motion = new RowMotion<null>();
    motion.update(targets(0, 2, 1), { appear: true });
    motion.update(targets(1, 3, 2), { moved: 1, still: true });
    expect(motion.get(0)).toBeUndefined();
    expect(motion.get(1)?.y.value).toBe(0);
    expect(motion.get(3)?.y.value).toBe(60);
    expect(motion.get(3)?.opacity.value).toBe(0);
    run(motion, 1);
    expect(motion.get(3)?.opacity.value).toBe(0.4);
  });

  it("brings a row back that was still leaving", () => {
    const motion = new RowMotion<null>();
    motion.update(targets(0, 2, 1), { appear: true });
    motion.update(targets(1, 3, 2), { moved: 1 });
    run(motion, 0.1);
    motion.update(targets(0, 2, 1), { moved: -1 });
    expect(motion.get(0)?.leaving).toBe(false);
    run(motion, 2);
    expect(motion.list().map((row) => [row.index, row.y.value, row.opacity.value])).toEqual([
      [0, 0, 0.4],
      [1, 30, 1],
      [2, 60, 0.4],
    ]);
  });
});
