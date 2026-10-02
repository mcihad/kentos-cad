import { METRES_PER_PX } from './symbolScale';

/** The view's screen scale as the status bar shows it (“Ekran 1:N”): the N at 96 dpi, from CSS pixels per metre. */
export const screenScale = (pxPerMetre: number): number => Math.round(1 / pxPerMetre / METRES_PER_PX);

/**
 * Whether snapping works at screen scale 1:`n` (docs/adr/0163 §5): not nearer than `snap.scaleMin`, not farther than
 * `snap.scaleMax`, 0 no limit. Out of it no snap shows, a one-shot snap neither. The desktop's is `Draft::snap_in_range`.
 */
export function snapInRange(n: number, min: number, max: number): boolean {
  return (min <= 0 || n >= min) && (max <= 0 || n <= max);
}
