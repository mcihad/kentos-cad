import { op } from '../wasm/core';
import type { TextAlign, TextRun } from './entities';
import type { Vec2 } from './geometry';

/**
 * A multi-line text (docs/adr/0182 §2, §4), computed by the geometry core
 * (crates/shared/geometry-core/src/text/paragraph.rs): where its lines break and stand, the label records a tool's
 * preview draws, and its runs as the editor changes them. The desktop calls the same functions natively;
 * fixtures/text/v1/paragraph.json holds both to the independent reference.
 *
 * The runs count Unicode scalar values; the browser's text fields count UTF-16 units: `letterAt` and `unitAt` turn
 * one into the other.
 */

/** A text as the core lays it out: its place, its multi-line fields, and the drawing's typeface (`font`). */
export interface ParagraphText {
  p: Vec2;
  text: string;
  height: number;
  rotation: number;
  align?: TextAlign;
  widthFactor?: number;
  boxWidth?: number;
  lineSpacing?: number;
  runs?: readonly TextRun[];
  mask?: boolean;
  /** The typeface it is measured in: its own, else the project's. */
  font?: string;
  /** Its face's bold and slant (docs/adr/0183 §2): bold measured in the bold table, the box leaning with its letters. */
  bold?: boolean;
  oblique?: number;
}

/** One line: its letters start..end, its width, where it stands from the box's left and how far under the first. */
export interface ParagraphLine {
  start: number;
  end: number;
  width: number;
  x: number;
  y: number;
}

/** A text's lines, box width and pitch, and where its point stands from the first baseline's left (along, up). */
export interface ParagraphLayout {
  lines: ParagraphLine[];
  width: number;
  pitch: number;
  shares: [number, number];
}

export const textLayout = op<(t: ParagraphText) => ParagraphLayout>('textLayout');

/** The label records the store gives a text (`LABEL_PARAGRAPH_MASK` with `mask`, then a `LABEL_LINE` per line), id 0. */
export const textLines = op<(t: ParagraphText) => number[]>('textLines');

/** A format the editor's buttons toggle over letters, or a colour it sets (null: the text's own). */
export type RunToggle = 'bold' | 'italic' | 'underline' | 'super' | 'sub' | { color: string | null };

/** The runs with `toggle` over the letters start..end of a text of `len` letters. */
export const textRunsToggle = op<(runs: readonly TextRun[], len: number, start: number, end: number, toggle: RunToggle) => TextRun[]>('textRunsToggle');

/** The runs after the editor's text `before` became `after`. */
export const textRunsRetext = op<(runs: readonly TextRun[], before: string, after: string) => TextRun[]>('textRunsRetext');

/**
 * Çok satırlı yazı's box from two corners for a text turned `rotation` degrees (docs/adr/0182 §4): its top left corner
 * (the text's point, its alignment the top's left) and its width along the turn; null: no box (under a micrometre).
 */
export const textCornerBox = op<(a: Vec2, b: Vec2, rotation: number) => { corner: Vec2; width?: number }>('textCornerBox');

/** A typed text without the white space at its ends, its runs following its letters (the desktop's `trimmed_paragraph`). */
export function trimmedParagraph(text: string, runs: readonly TextRun[]): { text: string; runs: TextRun[] } {
  const kept = text.trim();
  return { text: kept, runs: kept === text ? [...runs] : textRunsRetext(runs, text, kept) };
}

/** The letter (Unicode scalar value) a UTF-16 offset of `text` falls at. */
export function letterAt(text: string, unit: number): number {
  let letters = 0;
  for (let i = 0; i < unit && i < text.length; i++) {
    const c = text.charCodeAt(i);
    // A surrogate pair is one letter: its low half adds none.
    if (!(c >= 0xdc00 && c <= 0xdfff && i > 0 && text.charCodeAt(i - 1) >= 0xd800 && text.charCodeAt(i - 1) <= 0xdbff)) letters++;
  }
  return letters;
}

/** The UTF-16 offset of letter `letter` of `text`. */
export function unitAt(text: string, letter: number): number {
  let unit = 0;
  for (let n = 0; n < letter && unit < text.length; n++) {
    const c = text.charCodeAt(unit);
    unit += c >= 0xd800 && c <= 0xdbff && unit + 1 < text.length ? 2 : 1;
  }
  return unit;
}
