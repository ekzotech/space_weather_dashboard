import type { Sample } from './api';

export const WINDOW_MS = 3 * 60 * 60_000;
export type StormWindow = { start: string; end: string; peak: number };

export function stormWindows(points: Sample[], threshold: number): StormWindow[] {
  const windows: StormWindow[] = [];
  for (const point of points) {
    if (point.value < threshold) continue;
    const end = new Date(Date.parse(point.time) + WINDOW_MS).toISOString();
    const previous = windows.at(-1);
    if (previous && Date.parse(previous.end) === Date.parse(point.time)) {
      previous.end = end;
      previous.peak = Math.max(previous.peak, point.value);
    } else {
      windows.push({ start: point.time, end, peak: point.value });
    }
  }
  return windows;
}

