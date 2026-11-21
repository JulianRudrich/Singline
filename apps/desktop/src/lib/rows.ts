import { Spring } from "./motion";

/** Where one row of lyrics should be. */
export interface RowTarget<T> {
  /** The index of the row's line; it names the row. */
  index: number;
  /** Top of the row, in pixels. */
  y: number;
  scale: number;
  opacity: number;
  blur?: number;
  /** How far apart the rows stand here: a row coming in or going travels this far. */
  pitch: number;
  /** Whatever the view needs to draw the row, such as whether it is the active line. */
  data: T;
}

/** A row on its way to its target, or out of the view. */
export interface MovingRow<T> {
  index: number;
  y: Spring;
  scale: Spring;
  opacity: Spring;
  blur: Spring;
  pitch: number;
  data: T;
  /** No longer among the targets: it moves on and fades out, then it is dropped. */
  leaving: boolean;
}

export interface RowChange {
  /** Lines the lyrics moved on since the last change, negative going back; 0 for any other change, such as a new size. */
  moved?: number;
  /** Reduced motion: rows take their places at once and only fade. */
  still?: boolean;
  /** The rows are already there, as when the lyrics appear: no fading in. */
  appear?: boolean;
}

/** Seconds a row takes to fade away when it goes without a line change, as for a new size. */
const QUICK = 0.24;

function springs(y: number, scale: number, opacity: number, blur: number) {
  return {
    y: new Spring(y, { precision: 0.05 }),
    scale: new Spring(scale, { precision: 0.0005 }),
    opacity: new Spring(opacity, { precision: 0.002 }),
    blur: new Spring(blur, { precision: 0.01 }),
  };
}

/**
 * The rows of the lyrics as they move between line changes. A line change
 * moves them like a list that scrolls: the rows that stay glide to their new
 * places, the rows that are done keep going and fade out, and new rows come
 * in from where the lines come from while they fade in. All of them travel
 * the same distance, so no two lines ever sit on top of each other. A jump
 * past every row on show cuts instead: the old rows go at once and the new
 * ones rise in. Every spring keeps its speed when a new change comes, so quick
 * changes flow into each other.
 */
export class RowMotion<T> {
  private readonly rows = new Map<number, MovingRow<T>>();

  /** Sends the rows to `targets`. */
  update(targets: readonly RowTarget<T>[], { moved = 0, still = false, appear = false }: RowChange = {}): void {
    const direction = Math.sign(moved);
    const lines = Math.abs(moved);
    const scrolling = lines > 0 && !still && lines <= targets.length;
    const cut = lines > 0 && !still && !scrolling;
    const wanted = new Set<number>();
    for (const target of targets) {
      wanted.add(target.index);
      const blur = target.blur ?? 0;
      let row = this.rows.get(target.index);
      if (!row) {
        // A new row comes in from the side the lines come from, or fades in where it stands.
        const travel = scrolling ? lines * target.pitch : cut ? target.pitch : 0;
        const from = target.y + direction * travel;
        const opacity = appear ? target.opacity : 0;
        row = { index: target.index, ...springs(from, target.scale, opacity, blur), pitch: target.pitch, data: target.data, leaving: false };
        this.rows.set(target.index, row);
      } else if (row.leaving) {
        // Back before it was gone, as after a quick step back: it fades in again at the usual pace.
        row.leaving = false;
        row.opacity = carry(row.opacity, 0.48);
      }
      row.pitch = target.pitch;
      row.data = target.data;
      row.y.target = target.y;
      row.scale.target = target.scale;
      row.opacity.target = target.opacity;
      row.blur.target = blur;
      // Only a line change moves rows; a new size, font or layout puts them in place at once.
      if (!scrolling && !cut) {
        row.y.settle();
        row.scale.settle();
        row.blur.settle();
      }
    }
    for (const row of this.rows.values()) {
      if (wanted.has(row.index)) continue;
      if (still || cut) {
        // Gone at once, so it never overlaps the rows that come.
        this.rows.delete(row.index);
        continue;
      }
      if (!row.leaving) {
        row.leaving = true;
        if (!scrolling) row.opacity = carry(row.opacity, QUICK);
      }
      row.opacity.target = 0;
      if (scrolling) row.y.target -= direction * lines * row.pitch;
      else row.y.settle();
    }
  }

  /** Moves `dt` seconds on and drops rows that have faded out; returns whether anything still moves. */
  step(dt: number): boolean {
    let moving = false;
    for (const row of this.rows.values()) {
      for (const spring of [row.y, row.scale, row.opacity, row.blur]) {
        if (spring.step(dt)) moving = true;
      }
      if (row.leaving && !row.opacity.moving) this.rows.delete(row.index);
    }
    return moving;
  }

  /** The rows to draw, top line first. */
  list(): MovingRow<T>[] {
    return [...this.rows.values()].sort((a, b) => a.index - b.index);
  }

  get(index: number): MovingRow<T> | undefined {
    return this.rows.get(index);
  }

  /** How many rows there are to draw, the leaving ones included. */
  get size(): number {
    return this.rows.size;
  }
}

/** The same value and speed on a spring with another response. */
function carry(spring: Spring, response: number): Spring {
  const next = new Spring(spring.value, { response, precision: 0.002 });
  next.target = spring.target;
  next.velocity = spring.velocity;
  return next;
}
