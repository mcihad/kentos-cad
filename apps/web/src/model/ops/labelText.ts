import { op } from '../../wasm/core';
import type { TextAlign } from '../../contracts/generated/TextAlign';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';
import type { LabelStyle } from '../layers';

/**
 * Etiketleri yazıya çevir (docs/adr/0175, 0212 §4): the core's `ops::label_text`. The labels the label engine places
 * at 1:N, as the sheet writes them (app/sheet/mapLabels.ts), written as objects: a label's size in CSS px on the paper
 * (25.4/96 mm each) is its text's height on the ground; a one-line label a text centred on its line, a stacked one a
 * multi-line text, a curved one a text along a curve through its letters' middles, a callout a line. The geometry
 * store places them (`PickIndex.labelTexts`); the placing's independent reference is
 * scripts/fixtures/label_engine_cases.py.
 */

/**
 * A label's text: the template's first `{label}` the label, literally (unlike `String.replace`, `$&` and such in the
 * label mean nothing); without a template, or with an empty one, the label. The core's `fill_template`.
 */
export function fillTemplate(template: string | undefined, label: string): string {
  if (!template) return label;
  const i = template.indexOf('{label}');
  return i < 0 ? template : `${template.slice(0, i)}${label}${template.slice(i + '{label}'.length)}`;
}

/** A label written as a text object (docs/adr/0212 §4). */
export interface LabelText {
  /** The asked object it was made from (its place in the input). */
  item: number;
  /** The label's class (its layer's rule's place; 0 a single label's). */
  class: number;
  text: string;
  p: Vec2;
  /** On the ground, metres. */
  height: number;
  /** Degrees counter-clockwise from east. */
  rotation: number;
  align: TextAlign;
  /** A multi-line text's line spacing (in 5/3 of its height); none for one line. */
  lineSpacing?: number | null;
  /** A curved label's curve: its vertices after `p` in the text's frame (rotation 0), metres. */
  path?: Vec2[] | null;
}

/** A label's callout written as a line: from by the label to its object. */
export interface LabelCallout {
  item: number;
  class: number;
  from: Vec2;
  to: Vec2;
}

/** The texts and callouts, and how many labels were passed over and why. */
export interface LabelTexts {
  texts: LabelText[];
  callouts: LabelCallout[];
  /** Out of their class's scale range. */
  outOfScale: number;
  /** On an object smaller than the class's smallest feature. */
  small: number;
  /** With no free place (written only when every label is wanted). */
  overlapping: number;
}

/**
 * One object's label as a text at 1:`scale`, placed alone by the label engine (nothing else on the page): none when
 * it writes nothing (no text, out of the style's scale range, too small, no place inside an area that wants one). What
 * keeps a linked text with its object (model/linkedTexts.ts, docs/adr/0175 §4); `point` is its layer's point symbol's
 * size (px), `font` the drawing's typeface (`DrawingFont`).
 */
export const labelTextOf = op<(entity: Entity, label: string, style: LabelStyle, scale: number, point: number, font: string) => LabelText | undefined>('labelTextOf', true);
