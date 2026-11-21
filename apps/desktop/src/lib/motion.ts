/**
 * Motion tokens, after the macOS app: a strong ease-out for things that appear,
 * a short press, a quick pane change, a slower reveal and an interruptible spring
 * for lyric lines. styles.css mirrors the CSS tokens as custom properties
 * (`--ease-out`, `--duration-press`, …) for transitions.
 */
import type { TransitionConfig } from "svelte/transition";

/** A timing function like CSS `cubic-bezier()`. */
export function cubicBezier(x1: number, y1: number, x2: number, y2: number): (t: number) => number {
  const cx = 3 * x1;
  const bx = 3 * (x2 - x1) - cx;
  const ax = 1 - cx - bx;
  const cy = 3 * y1;
  const by = 3 * (y2 - y1) - cy;
  const ay = 1 - cy - by;
  const x = (s: number) => ((ax * s + bx) * s + cx) * s;
  const y = (s: number) => ((ay * s + by) * s + cy) * s;
  const slope = (s: number) => (3 * ax * s + 2 * bx) * s + cx;
  return (t) => {
    if (t <= 0) return 0;
    if (t >= 1) return 1;
    let s = t;
    for (let step = 0; step < 8; step += 1) {
      const error = x(s) - t;
      if (Math.abs(error) < 1e-7) return y(s);
      const d = slope(s);
      if (Math.abs(d) < 1e-7) break;
      s -= error / d;
    }
    let low = 0;
    let high = 1;
    s = t;
    while (high - low > 1e-7) {
      if (x(s) < t) low = s;
      else high = s;
      s = (low + high) / 2;
    }
    return y(s);
  };
}

/** The strong ease-out everything that appears uses. */
export const easeOut = cubicBezier(0.23, 1, 0.32, 1);

/** Smoothstep: eases a value driven by a clock rather than by an animation. */
export function smoothstep(x: number): number {
  const t = Math.min(1, Math.max(0, x));
  return t * t * (3 - 2 * t);
}

/**
 * Leaving content when the song changes: it drifts up, blurs and fades. With
 * reduced motion it only fades.
 */
export function songExit(_node: Element, { reduced = false }: { reduced?: boolean } = {}): TransitionConfig {
  return {
    duration: 250,
    easing: easeOut,
    css: reduced
      ? (t) => `opacity: ${t}`
      : (t, u) => `opacity: ${t}; transform: translateY(${-10 * u}px); filter: blur(${6 * u}px)`,
  };
}

/** A section that opens below a row drops 6 px into place. With reduced motion it only fades. */
export function dropIn(_node: Element, { reduced = false }: { reduced?: boolean } = {}): TransitionConfig {
  return {
    duration: duration.pane,
    easing: easeOut,
    css: reduced ? (t) => `opacity: ${t}` : (t, u) => `opacity: ${t}; transform: translateY(${-rise.pane * u}px)`,
  };
}

/** Controls that slide in from the right and fade. With reduced motion they only fade. */
export function slideIn(_node: Element, { reduced = false }: { reduced?: boolean } = {}): TransitionConfig {
  return {
    duration: 200,
    easing: easeOut,
    css: reduced ? (t) => `opacity: ${t}` : (t, u) => `opacity: ${t}; transform: translateX(${6 * u}px)`,
  };
}

/** The way back: quicker, as closing is. With reduced motion it only fades. */
export function slideOut(node: Element, options: { reduced?: boolean } = {}): TransitionConfig {
  return { ...slideIn(node, options), duration: duration.dismiss };
}

/**
 * One number moving towards a target like a damped spring: `response` is the
 * period of the undamped spring in seconds, `damping` the share of critical
 * damping. A new target keeps the current speed, so quick changes flow into
 * each other. Steps are exact for any frame time.
 */
export class Spring {
  value: number;
  target: number;
  velocity = 0;
  private readonly omega: number;
  private readonly zeta: number;
  private readonly precision: number;

  constructor(value: number, { response = 0.48, damping = 0.88, precision = 0.001 } = {}) {
    this.value = value;
    this.target = value;
    this.omega = (2 * Math.PI) / Math.max(0.01, response);
    this.zeta = Math.min(0.999, Math.max(0.01, damping));
    this.precision = precision;
  }

  get moving(): boolean {
    return this.value !== this.target || this.velocity !== 0;
  }

  /** Moves `dt` seconds on; returns whether the spring is still moving. */
  step(dt: number): boolean {
    if (!this.moving) return false;
    const x0 = this.value - this.target;
    const v0 = this.velocity;
    const decay = this.zeta * this.omega;
    const damped = this.omega * Math.sqrt(1 - this.zeta * this.zeta);
    const fade = Math.exp(-decay * Math.max(0, dt));
    const cos = Math.cos(damped * dt);
    const sin = Math.sin(damped * dt);
    const b = (v0 + decay * x0) / damped;
    const x = fade * (x0 * cos + b * sin);
    const v = fade * (v0 * cos - (decay * b + damped * x0) * sin);
    if (Math.abs(x) < this.precision && Math.abs(v) < this.precision * 10) {
      this.settle();
      return false;
    }
    this.value = this.target + x;
    this.velocity = v;
    return true;
  }

  /** Jumps to the target and stops. */
  settle(): void {
    this.value = this.target;
    this.velocity = 0;
  }
}

/** The same ease-out as a CSS timing function, for CSS and Web Animations. */
export const easeOutCss = "cubic-bezier(0.23, 1, 0.32, 1)";

export const duration = {
  /** Buttons and items while pressed. */
  press: 140,
  /** Switching panes, selections that glide, switches, the tray panel opening. */
  pane: 240,
  /** Closing is quicker than opening: the app answers, the listener does not decide. */
  dismiss: 120,
  /** Content that arrives, such as a new song: rare, so it may take a little longer. */
  reveal: 550,
} as const;

export const pressScale = {
  /** Buttons. */
  regular: 0.94,
  /** Larger targets such as sidebar items. */
  subtle: 0.97,
} as const;

/** How far content rises while it comes in, in pixels. */
export const rise = { pane: 6, reveal: 8 } as const;

/** How blurred revealed content starts, in pixels. */
export const revealBlur = 6;

export function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Keyframes for content coming in. With reduced motion it only fades. */
export function entranceKeyframes(kind: "pane" | "reveal", reduced: boolean): Keyframe[] {
  if (reduced) return [{ opacity: 0 }, { opacity: 1 }];
  if (kind === "pane") {
    return [
      { opacity: 0, transform: `translateY(${rise.pane}px)` },
      { opacity: 1, transform: "none" },
    ];
  }
  return [
    { opacity: 0, transform: `translateY(${rise.reveal}px)`, filter: `blur(${revealBlur}px)` },
    { opacity: 1, transform: "none", filter: "none" },
  ];
}

/** The tray panel opening: it fades in and rises 8 px. With reduced motion it only fades. */
export function panelKeyframes(reduced: boolean): Keyframe[] {
  if (reduced) return [{ opacity: 0 }, { opacity: 1 }];
  return [
    { opacity: 0, transform: `translateY(${rise.reveal}px)` },
    { opacity: 1, transform: "none" },
  ];
}

function enter(node: HTMLElement, kind: "pane" | "reveal", delay: number) {
  if (typeof node.animate !== "function") return;
  node.animate(entranceKeyframes(kind, prefersReducedMotion()), {
    duration: duration[kind],
    delay,
    easing: easeOutCss,
    fill: "backwards",
  });
}

/** Svelte action: a newly shown pane fades in while rising 6 px. Pass `false` to skip it. */
export function paneIn(node: HTMLElement, enabled = true) {
  if (enabled) enter(node, "pane", 0);
}

/**
 * Svelte action: fades, lifts and un-blurs content in. Put it inside a `{#key}`
 * block to replay it, for example once per song.
 */
export function reveal(node: HTMLElement, delay = 0) {
  enter(node, "reveal", delay);
}
