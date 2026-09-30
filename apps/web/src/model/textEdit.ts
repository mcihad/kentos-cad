import { op } from '../wasm/core';
import type { TextAlign } from './entities';
import type { Vec2 } from './geometry';

/**
 * A text's editing rules (docs/adr/0145 §3), computed by the geometry core
 * (crates/shared/geometry-core/src/text/edit.rs, `TextPlace::readable`): Artır's
 * next number, Bul ve değiştir's matching and Okunur yap's turn. The desktop
 * calls the same functions natively; fixtures/text/v1 holds both to the
 * independent reference.
 */

/** Artır: the text with the number it ends with one more (`A-009` → `A-010`); null without one. */
export const textIncrement = op<(text: string) => string | null>('textIncrement');

/** How Bul ve değiştir looks: wildcards (`*`, the whole text), case folded (Turkish), whole words. */
export interface FindHow {
  wildcard: boolean;
  caseless: boolean;
  wholeWord: boolean;
}

/** Bul ve değiştir: each text as it becomes, null where nothing matched; one call for many texts. */
export const textReplace = op<(texts: readonly string[], find: string, replace: string, how: FindHow) => (string | null)[]>('textReplace');

/** A text as Okunur yap reads it: its place, and its width (m) or its text and the drawing's typeface. */
export interface ReadableText {
  p: Vec2;
  height: number;
  rotation: number;
  align?: TextAlign;
  widthFactor?: number;
  text?: string;
  font?: string;
  width?: number;
}

/** Okunur yap: a text that reads upside down, turned half round about its box's middle; null when it reads. */
export const textReadable = op<(t: ReadableText) => { p: Vec2; rotation: number } | null>('textReadable');
