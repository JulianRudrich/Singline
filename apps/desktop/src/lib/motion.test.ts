import { describe, expect, it } from "vitest";
import styles from "../styles.css?raw";
import {
  cubicBezier,
  duration,
  easeOut,
  easeOutCss,
  dropIn,
  entranceKeyframes,
  panelKeyframes,
  pressScale,
  revealBlur,
  rise,
  slideIn,
  slideOut,
  smoothstep,
  songExit,
  stepIn,
  stepOut,
  Spring,
} from "./motion";

const element = {} as Element;

describe("cubicBezier", () => {
  it("matches CSS timing functions", () => {
    expect(cubicBezier(0, 0, 1, 1)(0.3)).toBeCloseTo(0.3, 6);
    const ease = cubicBezier(0.25, 0.1, 0.25, 1);
    expect(ease(0.5)).toBeCloseTo(0.8024, 3);
    expect(ease(0)).toBe(0);
    expect(ease(1)).toBe(1);
  });

  it("eases out strongly and never goes back", () => {
    expect(easeOut(0.25)).toBeGreaterThan(0.7);
    const samples = Array.from({ length: 101 }, (_, index) => easeOut(index / 100));
    expect(samples.every((value, index) => index === 0 || value >= (samples[index - 1] ?? 0))).toBe(true);
  });
});

describe("smoothstep", () => {
  it("starts and ends flat and stays in range", () => {
    expect([smoothstep(-1), smoothstep(0), smoothstep(0.5), smoothstep(1), smoothstep(2)]).toEqual([0, 0, 0.5, 1, 1]);
    expect(smoothstep(0.1)).toBeCloseTo(0.028);
  });
});

describe("Spring", () => {
  const run = (spring: Spring, seconds: number, fps = 60) => {
    let highest = spring.value;
    for (let frame = 0; frame < Math.round(seconds * fps); frame += 1) {
      spring.step(1 / fps);
      highest = Math.max(highest, spring.value);
    }
    return highest;
  };

  it("arrives quickly and barely overshoots", () => {
    const spring = new Spring(0);
    spring.target = 100;
    run(spring, 0.3);
    expect(spring.value).toBeGreaterThan(90);
    expect(spring.value).toBeLessThan(99);
    const highest = run(spring, 1.5);
    expect(highest).toBeLessThan(100.5);
    expect(spring.value).toBe(100);
    expect(spring.moving).toBe(false);
  });

  it("does not depend on the frame rate", () => {
    const fast = new Spring(0);
    const slow = new Spring(0);
    fast.target = slow.target = 40;
    run(fast, 0.2, 120);
    run(slow, 0.2, 30);
    expect(fast.value).toBeCloseTo(slow.value, 6);
  });

  it("keeps its speed when the target moves", () => {
    const spring = new Spring(0);
    spring.target = 100;
    run(spring, 0.1);
    const { value, velocity } = spring;
    spring.target = 200;
    expect([spring.value, spring.velocity]).toEqual([value, velocity]);
    spring.step(1 / 60);
    expect(spring.value).toBeGreaterThan(value);
  });

  it("stays still at its target and can jump there", () => {
    const spring = new Spring(5);
    expect(spring.step(1 / 60)).toBe(false);
    spring.target = 9;
    spring.settle();
    expect([spring.value, spring.moving]).toEqual([9, false]);
  });
});

describe("transitions", () => {
  it("lets the old song drift up, blur and fade", () => {
    const exit = songExit(element);
    expect(exit.duration).toBe(250);
    expect(exit.css?.(0, 1)).toBe("opacity: 0; transform: translateY(-10px); filter: blur(6px)");
    expect(songExit(element, { reduced: true }).css?.(0.5, 0.5)).toBe("opacity: 0.5");
  });

  it("drops a section into place below its row", () => {
    expect(dropIn(element).duration).toBe(duration.pane);
    expect(dropIn(element).css?.(0, 1)).toBe("opacity: 0; transform: translateY(-6px)");
    expect(dropIn(element, { reduced: true }).css?.(0, 1)).toBe("opacity: 0");
  });

  it("slides a guide's steps in from the side they lie on and out the other way", () => {
    expect(stepIn(element).css?.(0, 1)).toBe("opacity: 0; transform: translateX(16px)");
    expect(stepIn(element, { direction: -1 }).css?.(0, 1)).toBe("opacity: 0; transform: translateX(-16px)");
    expect(stepOut(element).css?.(0, 1)).toBe("opacity: 0; transform: translateX(-16px)");
    expect(stepOut(element, { direction: -1 }).css?.(0, 1)).toBe("opacity: 0; transform: translateX(16px)");
    expect(stepOut(element).duration).toBe(duration.dismiss);
    // The new step waits until the old one has nearly gone.
    expect(stepIn(element).delay).toBe(duration.dismiss / 2);
    expect(stepIn(element, { reduced: true }).css?.(0, 1)).toBe("opacity: 0");
    expect(stepOut(element, { reduced: true }).css?.(0, 1)).toBe("opacity: 0");
  });

  it("slides controls in from the right, and out again quicker", () => {
    expect(slideIn(element).css?.(0, 1)).toBe("opacity: 0; transform: translateX(6px)");
    expect(slideIn(element, { reduced: true }).css?.(0, 1)).toBe("opacity: 0");
    expect(slideOut(element).duration).toBe(duration.dismiss);
    expect(slideOut(element).css?.(0, 1)).toBe("opacity: 0; transform: translateX(6px)");
    expect(slideOut(element, { reduced: true }).css?.(0, 1)).toBe("opacity: 0");
  });
});

/** The value of a custom property in the first (unconditional) `:root` block. */
function token(name: string): string | undefined {
  const root = styles.match(/:root\s*{([^}]*)}/)?.[1] ?? "";
  return root.match(new RegExp(`${name}:\\s*([^;]+);`))?.[1]?.trim();
}

describe("motion tokens", () => {
  it("match the CSS custom properties", () => {
    expect(token("--ease-out")).toBe(easeOutCss);
    expect(token("--duration-press")).toBe(`${duration.press}ms`);
    expect(token("--duration-pane")).toBe(`${duration.pane}ms`);
    expect(token("--duration-reveal")).toBe(`${duration.reveal}ms`);
    expect(token("--duration-dismiss")).toBe(`${duration.dismiss}ms`);
    expect(token("--press-scale")).toBe(String(pressScale.regular));
    expect(token("--press-scale-subtle")).toBe(String(pressScale.subtle));
    expect(token("--press-scale-icon")).toBe(String(pressScale.icon));
    expect(token("--rise-pane")).toBe(`${rise.pane}px`);
    expect(token("--rise-reveal")).toBe(`${rise.reveal}px`);
    expect(token("--reveal-blur")).toBe(`${revealBlur}px`);
  });

  it("only fade with reduced motion", () => {
    for (const kind of ["pane", "reveal"] as const) {
      expect(entranceKeyframes(kind, true)).toEqual([{ opacity: 0 }, { opacity: 1 }]);
      expect(entranceKeyframes(kind, false)[0]).toHaveProperty("transform");
    }
    expect(entranceKeyframes("reveal", false)[0]).toMatchObject({ filter: "blur(6px)", transform: "translateY(8px)" });
    expect(entranceKeyframes("pane", false)[0]).toEqual({ opacity: 0, transform: "translateY(6px)" });
    // A pane further up the list comes down from above.
    expect(entranceKeyframes("pane", false, -1)[0]).toEqual({ opacity: 0, transform: "translateY(-6px)" });
    expect(entranceKeyframes("pane", true, -1)).toEqual([{ opacity: 0 }, { opacity: 1 }]);
    expect(panelKeyframes(true)).toEqual([{ opacity: 0 }, { opacity: 1 }]);
    expect(panelKeyframes(false)[0]).toEqual({ opacity: 0, transform: "translateY(8px)" });
  });
});
