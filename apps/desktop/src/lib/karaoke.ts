import { clamp, currentIndex, estimatedWords, LONGEST_WORD } from "./lyrics";
import type { Measure } from "./measure";
import type { LyricLine, SyncLevel } from "./types";

/** A wait of this many seconds or more is a break, with a countdown before the next line. */
export const BREAK = 4;
/** The countdown before a line after a break: three dots, one a second. */
export const COUNTDOWN = 3;
/** A sung line keeps its place this long after its last word, before a break takes over. */
const HOLD = 0.8;
/** The intro card leaves this long before the countdown starts, so the first lines can be read. */
const CARD_LEAD = 1;
/** Intros that would leave the card less time than this get none: it would only flash. */
const CARD_SHORTEST = 2;

/** Line height as a share of the font size. */
export const LINE_HEIGHT = 1.25;
/** Lines may take this share of the window's width. */
export const MAX_WIDTH = 0.9;
/** A line too wide shrinks to this share of its size before it is split in two. */
export const SHRINK = 0.8;
/** Upcoming lines are the current line's text at this size. */
const UPCOMING = 0.72;
/** How bright the upcoming lines are, the next one first. */
const UPCOMING_OPACITY = [0.46, 0.22];
/** Space between two rows, as a share of the font size. */
const GAP = 0.32;

/**
 * The lyrics prepared for karaoke. Lines without word timing get the estimate
 * the overlay's highlight uses, and a pause of BREAK seconds or more after a
 * sung line becomes a break of its own, an empty line, so that its countdown
 * can show; so does the end of a song that goes on after its last line.
 * Breaks next to each other merge. Plain lyrics have no timing worth counting
 * down to and stay as they are.
 */
export function karaokeLines(lines: readonly LyricLine[], sync: SyncLevel, duration: number | null): LyricLine[] {
  if (sync === "unsynced") return [...lines];
  const result: LyricLine[] = [];
  lines.forEach((line, index) => {
    const next = lines[index + 1];
    if (line.text.length === 0) {
      if (result.at(-1)?.text !== "") result.push(line);
      return;
    }
    const timed = line.words && line.words.length > 0 ? line : { ...line, words: estimatedWords(line, next?.time ?? null) };
    result.push(timed);
    const until = next?.time ?? (duration && duration > 0 ? duration : null);
    if (until === null) return;
    const end = sungEnd(timed, until);
    const following = lines.slice(index + 1).find((later) => later.text.length > 0);
    const resumes = following?.time ?? until;
    if (resumes - end >= BREAK && end + HOLD < until) result.push({ time: end + HOLD, text: "" });
  });
  return result;
}

/** When the singing of a line with words ends, at the latest at `until`. */
function sungEnd(line: LyricLine, until: number): number {
  const last = line.words?.at(-1);
  if (!last) return Math.min(until, line.end ?? line.time);
  return Math.min(until, last.end ?? Math.min(line.end ?? until, last.start + LONGEST_WORD));
}

/** What karaoke shows at one moment. */
export interface Scene {
  /** Index of the line in the current place, -1 before the first. */
  current: number;
  /** The song's card instead of the lines: during a long intro, and once the singing is over. */
  card: boolean;
  /** How far the dots in the current place have filled, from 0 to 1; null while a line is sung. */
  dots: number | null;
  /** How far the countdown to the next line has come, from 0 to 1; 0 outside of one. */
  countdown: number;
}

/**
 * What karaoke shows at `position`, in the lyrics' time; `start` is where the
 * song begins in that time, which the timing corrections move. Before a line
 * that follows a wait of BREAK seconds or more, the dots stay faint and then
 * count down its last COUNTDOWN seconds, one dot a second; shorter waits fill
 * them over the whole wait, as in the overlay. A long intro shows the card
 * until shortly before its countdown.
 */
export function karaokeScene(lines: readonly LyricLine[], position: number, start = 0, sync: SyncLevel = "line"): Scene {
  const current = currentIndex(lines, position) ?? -1;
  const first = lines.find((line) => line.text.length > 0);
  if (!first) return { current, card: true, dots: null, countdown: 0 };
  if (position < first.time) {
    const leave = first.time - COUNTDOWN - CARD_LEAD;
    const card = leave - start >= CARD_SHORTEST && position < leave;
    return { current, card, ...waiting(start, first.time, position, sync) };
  }
  const line = lines[current];
  if (line && line.text.length === 0) {
    const next = lines[current + 1];
    if (!next) return { current, card: true, dots: null, countdown: 0 };
    return { current, card: false, ...waiting(line.time, next.time, position, sync) };
  }
  return { current, card: false, dots: null, countdown: 0 };
}

function waiting(from: number, until: number, position: number, sync: SyncLevel): Pick<Scene, "dots" | "countdown"> {
  if (sync !== "unsynced" && until - from >= BREAK) {
    const countdown = clamp((position - (until - COUNTDOWN)) / COUNTDOWN, 0, 1);
    return { dots: countdown, countdown };
  }
  return { dots: clamp((position - from) / Math.max(0.1, until - from), 0, 1), countdown: 0 };
}

/** The current line's font size in a window of this size, times the listener's scale. */
export function karaokeFontSize(width: number, height: number, scale = 1): number {
  const fitting = clamp(Math.min(width * 0.05, height * 0.09), 20, 200);
  return Math.round(fitting * clamp(scale, 0.5, 2));
}

/** Upcoming lines below the current one: two where they fit, otherwise one. */
export function upcomingCount(height: number, fontSize: number): number {
  const row = fontSize * LINE_HEIGHT;
  const block = row + 2 * (row * UPCOMING + fontSize * GAP);
  return block <= height * 0.62 ? 2 : 1;
}

/** Where one row of the lyrics goes. */
export interface Slot {
  /** Index of its line; there may be none, as before the first line. */
  index: number;
  /** -1 for the line leaving above, 0 for the current one, 1 and 2 for the ones coming. */
  place: number;
  /** Top of the row, in pixels from the top of the stage. */
  y: number;
  scale: number;
  opacity: number;
  /** From this row's top to the next one's, in pixels: how far it travels coming in or going. */
  pitch: number;
}

export interface SlotArea {
  width: number;
  height: number;
  fontSize: number;
  upcoming: number;
  /** Width of a text at `fontSize`. */
  measure: Measure;
}

/**
 * Where the rows go: the current line large and the upcoming ones below it,
 * smaller and dimmer, all of them centred. The line before waits above,
 * invisible, so a line that is done moves up and fades out. Lines too wide
 * shrink to fit. Rows without a line keep their room, so nothing moves at the
 * start and end of a song.
 */
export function karaokeSlots(lines: readonly LyricLine[], current: number, area: SlotArea): Slot[] {
  const { width, height, fontSize, upcoming, measure } = area;
  const gap = fontSize * GAP;
  const rows = [];
  for (let place = -1; place <= upcoming; place += 1) {
    const index = current + place;
    const line = lines[index];
    const size = place === 0 ? 1 : UPCOMING;
    const fit = line && line.text.length > 0 ? (width * MAX_WIDTH) / Math.max(1, measure(line.text)) : 1;
    const scale = Math.min(size, fit);
    const opacity = place === 0 ? 1 : place < 0 || !line ? 0 : (UPCOMING_OPACITY[place - 1] ?? 0);
    rows.push({ index, place, scale, opacity, extent: fontSize * LINE_HEIGHT * scale });
  }
  const below = rows.filter((row) => row.place >= 0);
  const block = below.reduce((sum, row) => sum + row.extent, 0) + gap * (below.length - 1);
  const top = (height - block) / 2;
  let y = top;
  return rows.map(({ extent, ...row }) => {
    const pitch = extent + gap;
    if (row.place < 0) return { ...row, y: top - pitch, pitch };
    const slot = { ...row, y, pitch };
    y += pitch;
    return slot;
  });
}
