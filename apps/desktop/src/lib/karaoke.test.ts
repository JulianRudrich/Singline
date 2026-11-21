import { describe, expect, it } from "vitest";
import { BREAK, COUNTDOWN, karaokeFontSize, karaokeLines, karaokeScene, karaokeSlots, upcomingCount } from "./karaoke";
import type { LyricLine } from "./types";

/** Ten pixels per character. */
const measure = (text: string) => text.length * 10;

function timed(time: number, text: string, end: number): LyricLine {
  return { time, text, words: [{ start: time, end, text }] };
}

describe("karaokeLines", () => {
  it("turns a long pause after a sung line into a break of its own", () => {
    const lines = [timed(10, "Verse", 12), timed(20, "Chorus", 22)];
    const result = karaokeLines(lines, "word", null);
    expect(result.map((line) => [line.time, line.text])).toEqual([
      [10, "Verse"],
      [12.8, ""],
      [20, "Chorus"],
    ]);
    expect(result[0]).toBe(lines[0]);
  });

  it("keeps short pauses inside the line", () => {
    const lines = [timed(10, "Verse", 12), timed(15.5, "Chorus", 17)];
    expect(karaokeLines(lines, "word", null)).toEqual(lines);
  });

  it("merges a long pause with the break the lyrics already have", () => {
    const lines = [timed(10, "Verse", 12), { time: 16, text: "" }, { time: 18, text: "" }, timed(24, "Chorus", 26)];
    expect(karaokeLines(lines, "word", null).map((line) => line.time)).toEqual([10, 12.8, 24]);
    // A break right after the last word stays where the lyrics put it.
    const close = [timed(10, "Verse", 12), { time: 12.5, text: "" }, timed(20, "Chorus", 22)];
    expect(karaokeLines(close, "word", null).map((line) => line.time)).toEqual([10, 12.5, 20]);
  });

  it("estimates the words of line-synced lyrics once, from the original next line", () => {
    const lines: LyricLine[] = [
      { time: 0, text: "one two" },
      { time: 30, text: "three" },
    ];
    const [first, pause, second] = karaokeLines(lines, "line", null);
    expect(first?.words?.map((word) => word.text)).toEqual(["one ", "two"]);
    // Short lines are sung over at least 0.6 s; the break follows 0.8 s later.
    expect(pause).toEqual({ time: expect.closeTo(0.6 + 0.8, 5), text: "" });
    expect(second?.words).toHaveLength(1);
  });

  it("ends with a break when the song goes on after the last line", () => {
    const lines = [timed(10, "Last", 12)];
    expect(karaokeLines(lines, "word", 60).map((line) => line.time)).toEqual([10, 12.8]);
    expect(karaokeLines(lines, "word", 14)).toHaveLength(1);
    expect(karaokeLines(lines, "word", null)).toHaveLength(1);
  });

  it("leaves plain lyrics as they are", () => {
    const lines: LyricLine[] = [
      { time: 0, text: "one" },
      { time: 40, text: "two" },
    ];
    expect(karaokeLines(lines, "unsynced", 90)).toEqual(lines);
  });
});

describe("karaokeScene", () => {
  const song = karaokeLines([timed(12, "First", 14), timed(16, "Second", 18), timed(30, "Third", 32)], "word", 40);

  it("shows the card through a long intro, then the lines before the countdown", () => {
    expect(karaokeScene(song, 0)).toMatchObject({ current: -1, card: true, dots: 0, countdown: 0 });
    expect(karaokeScene(song, 7.9).card).toBe(true);
    expect(karaokeScene(song, 8.1)).toMatchObject({ card: false, dots: 0 });
  });

  it("counts the last three seconds down, one dot a second", () => {
    const at = (position: number) => karaokeScene(song, position);
    expect(at(12 - COUNTDOWN).dots).toBe(0);
    expect(at(10).dots).toBeCloseTo(1 / 3);
    expect(at(11).countdown).toBeCloseTo(2 / 3);
    expect(at(12)).toMatchObject({ current: 0, dots: null, countdown: 0 });
  });

  it("counts down to a line after a long break, from the break the pause became", () => {
    const pause = song.findIndex((line) => line.text === "" && line.time > 16);
    expect(karaokeScene(song, 20)).toMatchObject({ current: pause, card: false, dots: 0, countdown: 0 });
    expect(karaokeScene(song, 28.5).dots).toBeCloseTo(0.5);
    expect(karaokeScene(song, 30).current).toBe(pause + 1);
  });

  it("fills the dots over a short wait, without a countdown", () => {
    const short = [timed(2, "Soon", 3)];
    expect(karaokeScene(short, 1)).toMatchObject({ card: false, dots: 0.5, countdown: 0 });
    // With the lyrics shown later, the same song has a longer intro.
    expect(karaokeScene(short, -6, -8)).toMatchObject({ card: true, dots: 0 });
  });

  it("brings the card back once the singing is over", () => {
    expect(karaokeScene(song, 32.5)).toMatchObject({ current: song.length - 2, card: false, dots: null });
    expect(karaokeScene(song, 35)).toMatchObject({ current: song.length - 1, card: true, dots: null });
  });

  it("gives plain lyrics no countdown and short intros no card", () => {
    const plain = [{ time: 6, text: "Plain" }];
    expect(karaokeScene(plain, 3, 0, "unsynced")).toMatchObject({ card: false, dots: 0.5, countdown: 0 });
    expect(karaokeScene([timed(5, "Early", 6)], 0).card).toBe(false);
    expect(BREAK).toBeGreaterThan(COUNTDOWN);
  });
});

describe("layout", () => {
  it("sizes the type to the window and the listener's scale", () => {
    expect(karaokeFontSize(1920, 1080)).toBe(96);
    expect(karaokeFontSize(3840, 2160)).toBe(192);
    expect(karaokeFontSize(1920, 1080, 1.5)).toBe(144);
    // Tall and narrow windows follow the width, small ones keep a readable size.
    expect(karaokeFontSize(800, 1600)).toBe(40);
    expect(karaokeFontSize(200, 100)).toBe(20);
  });

  it("shows two upcoming lines where they fit, otherwise one", () => {
    expect(upcomingCount(1080, 96)).toBe(2);
    expect(upcomingCount(400, 96)).toBe(1);
  });

  const lines: LyricLine[] = [
    { time: 0, text: "Before" },
    { time: 4, text: "Now" },
    { time: 8, text: "x".repeat(120) },
    { time: 12, text: "Later" },
  ];
  const area = { width: 1000, height: 800, fontSize: 100, upcoming: 2, measure };

  it("puts the current line large in the middle and the next ones below, smaller and dimmer", () => {
    const [previous, current, next, after] = karaokeSlots(lines, 1, area);
    expect(current).toMatchObject({ index: 1, place: 0, scale: 1, opacity: 1 });
    expect(next).toMatchObject({ index: 2, place: 1, opacity: 0.46 });
    expect(after).toMatchObject({ index: 3, place: 2, scale: 0.72, opacity: 0.22 });
    // The block of current and upcoming lines is centred.
    const bottom = (after?.y ?? 0) + 100 * 1.25 * 0.72;
    expect((current?.y ?? 0) + bottom).toBeCloseTo(800);
    // The line before waits above, invisible, to move up and fade out.
    expect(previous).toMatchObject({ index: 0, place: -1, opacity: 0 });
    expect(previous?.y).toBeLessThan(current?.y ?? 0);
    // Each row knows how far it is to the next, for rows coming in and going.
    expect((current?.y ?? 0) + (current?.pitch ?? 0)).toBeCloseTo(next?.y ?? 0);
    expect((previous?.y ?? 0) + (previous?.pitch ?? 0)).toBeCloseTo(current?.y ?? 0);
  });

  it("shrinks lines that are too wide to fit", () => {
    expect(karaokeSlots(lines, 2, area)[1]?.scale).toBeCloseTo(900 / 1200);
    // Upcoming lines are smaller anyway; this one fits at their size.
    expect(karaokeSlots(lines, 1, area)[2]?.scale).toBe(0.72);
  });

  it("keeps room for rows without a line, so nothing moves at the start", () => {
    const intro = karaokeSlots(lines, -1, area);
    expect(intro.map((slot) => [slot.index, slot.opacity])).toEqual([
      [-2, 0],
      [-1, 1],
      [0, 0.46],
      [1, 0.22],
    ]);
    const end = karaokeSlots(lines, 3, area);
    expect(end.map((slot) => slot.opacity)).toEqual([0, 1, 0, 0]);
    expect(end[1]?.y).toBeCloseTo(karaokeSlots(lines, 1, area)[1]?.y ?? 0);
  });
});
