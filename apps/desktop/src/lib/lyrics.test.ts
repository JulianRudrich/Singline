import { describe, expect, it } from "vitest";
import { currentIndex, estimatedWords, lineFills, lineText, UNSUNG, wordFills, wordOpacity } from "./lyrics";
import type { LyricLine } from "./types";

const lines: LyricLine[] = [
  { time: 1, text: "First" },
  { time: 3.5, text: "Second" },
  { time: 7, text: "Again" },
  { time: 9.1, text: "Again" },
  { time: 12, text: "" },
];

describe("currentIndex", () => {
  it("matches the Rust core", () => {
    expect(currentIndex(lines, 0.99)).toBeNull();
    expect(currentIndex(lines, 1)).toBe(0);
    expect(currentIndex(lines, 8)).toBe(2);
    expect(currentIndex(lines, 12)).toBe(4);
    expect(currentIndex([], 0)).toBeNull();
  });

  it("takes the last of equal timestamps", () => {
    expect(currentIndex([{ time: 1, text: "A" }, { time: 1, text: "B" }], 1)).toBe(1);
  });
});

describe("wordFills", () => {
  const line: LyricLine = {
    time: 10,
    text: "Hello bright world",
    words: [
      { start: 10, end: 10.6, text: "Hello " },
      { start: 10.6, text: "bright " },
      { start: 11.2, text: "world" },
    ],
  };

  it("fills words as they are sung", () => {
    const fills = wordFills(line, 14, 10.9).map((word) => Number(word.fill.toFixed(2)));
    expect(fills).toEqual([1, 0.5, 0]);
  });

  it("ends the last word within a second and a half", () => {
    const [, , last] = wordFills(line, 20, 11.95);
    expect(last?.fill).toBeCloseTo(0.5);
  });

  it("returns nothing for lines without word timing", () => {
    expect(wordFills({ time: 1, text: "Plain" }, null, 2)).toEqual([]);
  });

  it("shows a note for breaks", () => {
    expect(lineText({ time: 1, text: "" })).toBe("♪");
    expect(lineText(undefined)).toBe("");
  });
});

describe("estimatedWords", () => {
  const ends = (line: LyricLine, next: number | null) => estimatedWords(line, next).map((word) => word.end);

  it("gives about 0.07 s to each character, counting one space per word", () => {
    const words = estimatedWords({ time: 10, text: "abcd efgh" }, 20);
    expect(words.map((word) => word.text)).toEqual(["abcd ", "efgh"]);
    expect(words[0]?.start).toBe(10);
    expect(words[0]?.end).toBeCloseTo(10.35);
    expect(words[1]?.start).toBeCloseTo(10.35);
    expect(words[1]?.end).toBeCloseTo(10.7);
  });

  it("takes at least 0.6 s for a line", () => {
    expect(ends({ time: 10, text: "Hi" }, 20)[0]).toBeCloseTo(10.6);
  });

  it("ends within 90 % of the gap to the next line", () => {
    expect(ends({ time: 10, text: "Hi" }, 10.5)[0]).toBeCloseTo(10.45);
    const long = { time: 10, text: "a fairly long line that would take many seconds to sing" };
    expect(ends(long, 12).at(-1)).toBeCloseTo(11.8);
  });

  it("allows the last line six seconds and respects a known line end", () => {
    const text = Array(10).fill("abcdefghi").join(" ");
    expect(ends({ time: 10, text }, null).at(-1)).toBeCloseTo(15.4);
    expect(ends({ time: 10, end: 11, text }, 30).at(-1)).toBeCloseTo(10.9);
  });

  it("shares the time out by word length", () => {
    const [short, long] = estimatedWords({ time: 0, text: "a bbbbbbbb" }, 20);
    const length = (word: { start: number; end?: number } | undefined) => (word?.end ?? 0) - (word?.start ?? 0);
    expect(length(long) / length(short)).toBeCloseTo(9 / 2);
  });

  it("keeps every space of the line", () => {
    const text = " Hello  bright world ";
    expect(estimatedWords({ time: 0, text }, 5).map((word) => word.text).join("")).toBe(text);
  });

  it("has no words for breaks", () => {
    expect(estimatedWords({ time: 0, text: "" }, 5)).toEqual([]);
    expect(estimatedWords({ time: 0, text: "   " }, 5)).toEqual([]);
  });
});

describe("lineFills", () => {
  const line: LyricLine = { time: 10, text: "made up words here" };
  const fills = (position: number, next = 14) => lineFills(line, next, position, "line").map((word) => word.fill);

  it("estimates the words of line-synced lyrics", () => {
    expect(fills(9)).toEqual([0, 0, 0, 0]);
    const middle = fills(10.6);
    expect(middle[0]).toBe(1);
    expect(middle.at(-1)).toBe(0);
    expect(middle.every((fill, index) => index === 0 || fill <= (middle[index - 1] ?? 1))).toBe(true);
    expect(fills(13.9)).toEqual([1, 1, 1, 1]);
    expect(fills(10.46, 10.5)).toEqual([1, 1, 1, 1]);
  });

  it("keeps the real word timing", () => {
    const timed: LyricLine = {
      time: 10,
      text: "one two",
      words: [
        { start: 10, text: "one " },
        { start: 11, text: "two" },
      ],
    };
    expect(lineFills(timed, 20, 10.5, "word")).toEqual(wordFills(timed, 20, 10.5));
    expect(lineFills(timed, 20, 10.5, "line")).toEqual(wordFills(timed, 20, 10.5));
  });

  it("estimates lines without word timing in word-synced lyrics too", () => {
    expect(lineFills(line, 14, 10.6, "word")).toEqual(lineFills(line, 14, 10.6, "line"));
  });

  it("leaves plain lyrics without word highlight", () => {
    expect(lineFills(line, 14, 10.6, "unsynced")).toEqual([]);
  });
});

describe("wordOpacity", () => {
  it("lights words from UNSUNG to white on a fully bright row", () => {
    expect(wordOpacity(0, 1, 0.42)).toBeCloseTo(UNSUNG);
    expect(wordOpacity(0.5, 1, 0.42)).toBeCloseTo(0.65);
    expect(wordOpacity(1, 1, 0.42)).toBe(1);
  });

  it("looks like a neighbour the moment the line becomes active", () => {
    expect(wordOpacity(0, 0.42, 0.42)).toBe(1);
    expect(wordOpacity(0, 0.2, 0.42)).toBe(1);
  });

  it("never dims an unsung word on screen while the row brightens", () => {
    let previous = 0.42;
    for (let step = 0; step <= 100; step += 1) {
      const row = 0.42 + (0.58 * step) / 100;
      const shown = row * wordOpacity(0, row, 0.42);
      expect(shown).toBeGreaterThanOrEqual(UNSUNG - 1e-9);
      expect(shown).toBeLessThanOrEqual(previous + 1e-9);
      previous = shown;
    }
    expect(previous).toBeCloseTo(UNSUNG);
  });
});
