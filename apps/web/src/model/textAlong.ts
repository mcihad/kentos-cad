import { op } from '../wasm/core';
import type { Entity, TextAlign, TextEntity, TextPath } from './entities';
import type { Vec2 } from './geometry';

/**
 * Eğri boyunca yazı (docs/adr/0196): the shared core's rules (`text::along`), as the tools and Okunur yap call them.
 * The letters themselves come from the store and `textLines` (a line record a letter, §2.6).
 */

/** Where a text of `length` metres stands on a curve clicked at `click`, the click its share along (§4); null off any. */
export const textAlongPiece = op<(e: Entity, click: Vec2, length: number, share: number) => { p: Vec2; rotation: number; path: TextPath } | null>('textAlongPiece');

/** A text's letters' length, metres (§2.2), measured as `textBox` measures (`font`: its typeface or the drawing's). */
export const textAlongLength = op<(t: Pick<TextEntity, 'text' | 'height' | 'widthFactor' | 'runs' | 'font' | 'bold'>) => number>('textAlongLength');

/** Okunur yap for a text along a curve (§3): its new point, rotation, curve and alignment; null when it reads. */
export const textAlongReadable = op<(t: TextEntity) => { p: Vec2; rotation: number; path: TextPath; align: TextAlign | null } | null>('textAlongReadable');

/** Düzleştir (§4): the straight text's point (its first letter's baseline start) and rotation; null for a straight text. */
export const textAlongStraight = op<(t: TextEntity) => { p: Vec2; rotation: number } | null>('textAlongStraight');

/** A text's curve's length, metres (Öznitelikler's Eğri row); null for a straight text. */
export const textAlongCurveLength = op<(t: TextEntity) => number | null>('textAlongCurveLength');

/** Doğrultuya döndür (§4): a direction's angle in degrees, half a turn less when it reads upside down. */
export const textAlongTurn = op<(d: Vec2) => number>('textAlongTurn');

/** Where the letters stand against the curve (§4): over it, on it, under it. */
export type AlongSide = 'over' | 'on' | 'under';

/** Hiza's choices: what of the text the click is, its share along, its word typed. */
export const ALONG_SHARES: readonly [number, string, string][] = [
  [0, 'Başı', 'başı'],
  [0.5, 'Ortası', 'ortası'],
  [1, 'Sonu', 'sonu'],
];

/** Konum's choices. */
export const ALONG_SIDES: readonly [AlongSide, string, string][] = [
  ['over', 'Üstünde', 'üstünde'],
  ['on', 'Ortasında', 'ortasında'],
  ['under', 'Altında', 'altında'],
];

/** The alignment a share along and a side make (the desktop's `align_of`). */
export function alongAlign(share: number, side: AlongSide): TextAlign {
  const h = share < 0.25 ? 'Left' : share > 0.75 ? 'Right' : 'Center';
  const v = side === 'over' ? 'bottom' : side === 'on' ? 'middle' : 'top';
  return `${v}${h}` as TextAlign;
}

/** An alignment's share along and side (a baseline one stands over the curve). */
export function alongSplit(a: TextAlign | null): [number, AlongSide] {
  const name = a ?? 'baselineLeft';
  const share = /Center$/.test(name) ? 0.5 : /Right$/.test(name) ? 1 : 0;
  const side: AlongSide = name.startsWith('middle') ? 'on' : name.startsWith('top') ? 'under' : 'over';
  return [share, side];
}
