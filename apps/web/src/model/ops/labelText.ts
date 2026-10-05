import { op } from '../../wasm/core';
import type { TextAlign } from '../../contracts/generated/TextAlign';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';
import type { LabelStyle } from '../layers';

/**
 * Etiketleri yazıya çevir (docs/adr/0175 §1): the core's `ops::label_text`. A layer's label as a text object, as the
 * sheet writes it on the paper at 1:N (app/sheet/mapLabels.ts): the style's size in CSS px on the paper (25.4/96 mm
 * each) is the text's height on the ground, the placement's offsets are paper px, a label along a line turns to read
 * upright; out of the style's scale range or on an object smaller than its smallest feature it is not written; one
 * touching an earlier label's 8 px paper cells (counted from the drawing's origin) is thinned unless every label is
 * wanted. The geometry store places each object's label (`PickIndex.labelTexts`). The independent reference is
 * scripts/fixtures/label_text_cases.py.
 */

/**
 * A label's text: the template's first `{label}` the label, literally (unlike `String.replace`, `$&` and such in the
 * label mean nothing); without a template, or with an empty one, the label. The core's `fill_template`.
 */
export function fillTemplate(template: string | undefined, label: string): string {
  if (!template) return label;
  const i = template.indexOf('{label}');
  return i < 0 ? template : template.slice(0, i) + label + template.slice(i + '{label}'.length);
}

/** A label to write: where its object puts it, how big the object is, its text and its layer's label style. */
export interface LabelItem {
  placement: LabelStyle['placement'];
  /** The anchor (centre, beside), the box's top left (corner) or the first vertex (along). */
  p: Vec2;
  /** Along: the second vertex. */
  q?: Vec2 | null;
  /** The smaller side of the object's box, metres. */
  feature: number;
  /** The text, its template filled. */
  text: string;
  /** The text's width in em in the drawing's typeface. */
  em: number;
  size: number;
  grow?: number | null;
  maxSize?: number | null;
  minScale?: number | null;
  maxScale?: number | null;
  minFeaturePx?: number | null;
}

/** A label written as a text object. */
export interface LabelText {
  /** The label it was made from (its place in the input). */
  item: number;
  text: string;
  p: Vec2;
  /** On the ground, metres. */
  height: number;
  /** Degrees counter-clockwise from east. */
  rotation: number;
  align: TextAlign;
}

/** The texts, and how many labels were passed over and why. */
export interface LabelTexts {
  texts: LabelText[];
  /** Out of their style's scale range. */
  outOfScale: number;
  /** On an object smaller than the style's smallest feature, or of no size. */
  small: number;
  /** Touching an earlier label's cells. */
  overlapping: number;
}

/** A label to write as the store places it: the object, its label and its layer's (or its kind's default) style. */
export interface LabelWanted {
  id: number;
  label: string;
  style: LabelStyle;
}

/** The labels' texts at 1:`scale` in the given order (the first of overlapping labels stays); `thin` passes over a label touching an earlier one's cells. */
export const labelTexts = op<(items: readonly LabelItem[], scale: number, thin: boolean) => LabelTexts>('labelTexts');

/**
 * One object's label as a text at 1:`scale`, as the rule writes it alone (nothing thins it): none when it writes
 * nothing (no label, out of the style's scale range, too small). What keeps a linked text with its object
 * (model/linkedTexts.ts, docs/adr/0175 §4); `font` is the drawing's typeface (`DrawingFont`).
 */
export const labelTextOf = op<(entity: Entity, label: string, style: LabelStyle, scale: number, font: string) => LabelText | undefined>('labelTextOf', true);
