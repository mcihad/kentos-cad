import { op } from '../wasm/core';
import type { Vec2 } from './geometry';
import type { Area, Ring } from './geom/overlay';

/**
 * Plan yolu çizimi (docs/adr/0198): the shared core's rules (`ops::road`), as Plan yolu, Kavşak temizle and Refüj kapat
 * call them. A road is an area; its kind and width are its attributes `Tür` and `Genişlik`.
 */

/** A road's areas from its axis (§2): the road, the carriageway inside its kerbs, the median; null when the widths do not fit. */
export const roadParts = op<(axis: readonly Vec2[], width: number, kerb: number, median: number) => { road: Area; carriageway?: Area; median?: Area } | null>('roadParts');

/** An area's inner corners rounded (§3), with the corners rounded and the inner ones left. */
export const roundInnerCorners = op<(area: Area, radius: number) => { area: Area; done: number; skipped: number }>('roundInnerCorners');

/** Kavşak temizle's work on one kind (§3): the areas joined, then their inner corners rounded. */
export const roadJunctions = op<(areas: readonly Area[], radius: number) => { areas: Area[]; done: number; skipped: number }>('roadJunctions');

/** Two lines closed into a median (§4); null for a line of fewer than two points. */
export const medianRing = op<(first: Ring, second: Ring, round: boolean) => Ring | null>('medianRing');

/** The attribute naming a road's kind, and its width's (metres, as a number's text). */
export const KIND = 'Tür';
export const WIDTH = 'Genişlik';

/** Plan yolu's kinds, their first widths (metres; only the tool's first values, docs/adr/0198 §2) and what they are called. */
export const ROAD_KINDS: readonly { kind: string; width: number }[] = [
  { kind: 'Yol', width: 10 },
  { kind: 'Yaya yolu', width: 5 },
  { kind: 'Bisiklet yolu', width: 2.5 },
];

/** The kinds Kavşak temizle leaves out (§3), and the one whose corners are kerb corners. */
export const NOT_JOINED: readonly string[] = ['Refüj', 'Yol ekseni'];
export const CARRIAGEWAY = 'Taşıt yolu';
