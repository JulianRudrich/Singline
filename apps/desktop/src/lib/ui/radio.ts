/**
 * Keyboard moves in a radio group, after the WAI-ARIA pattern: the arrow keys
 * go to the next or previous option, round the ends, and Home and End to the
 * first and last. Options that cannot be chosen are skipped. Returns the
 * index to choose, or null for any other key or when nothing else can be chosen.
 */
export function radioMove(
  key: string,
  index: number,
  count: number,
  enabled: (index: number) => boolean = () => true,
): number | null {
  if (count <= 0) return null;
  if (key === "Home" || key === "End") {
    const forward = key === "Home";
    for (let step = 0; step < count; step += 1) {
      const candidate = forward ? step : count - 1 - step;
      if (enabled(candidate)) return candidate;
    }
    return null;
  }
  const direction = key === "ArrowRight" || key === "ArrowDown" ? 1 : key === "ArrowLeft" || key === "ArrowUp" ? -1 : 0;
  if (direction === 0) return null;
  for (let step = 1; step < count; step += 1) {
    const candidate = (((index + direction * step) % count) + count) % count;
    if (enabled(candidate)) return candidate;
  }
  return null;
}
