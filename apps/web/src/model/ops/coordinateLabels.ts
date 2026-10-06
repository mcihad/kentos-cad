import { op } from '../../wasm/core';
import type { TextAlign } from '../entities';
import type { Vec2 } from '../geometry';
import type { ListedObject } from './table';

/**
 * Koordinat yaz (docs/adr/0185): the core's `ops::coordinate_labels`, one rule for both platforms. The places a
 * selection's vertices give, named as the coordinate schedule names them (docs/adr/0184 §3); a label's lines from its
 * template; where its leader and lines stand. The independent reference is scripts/fixtures/coordinate_label_cases.py
 * (fixtures/coordinate-labels/v1/cases.json).
 */

/** A place a label is written at: where, its elevation, its name, the middle of its object's box (none: clicked). */
export interface LabelPlace {
  readonly p: Vec2;
  readonly z?: number;
  readonly name?: string;
  readonly centre?: Vec2;
}

/** Where a label goes from its place: away from its object (`auto`), or a corner's way. */
export type LabelDirection = 'auto' | 'ne' | 'nw' | 'sw' | 'se';

/** How labels are written: the template, decimals, height (m), leader, direction and the face widths are measured in. */
export interface LabelOptions {
  readonly template: string;
  readonly decimals: number;
  readonly height: number;
  readonly leader: boolean;
  readonly direction: LabelDirection;
  readonly font: string;
  readonly bold?: boolean;
  readonly widthFactor?: number;
}

/** The project's axes (a CAD project's X east, a CBS project's Y east) and drawing unit. */
export interface LabelUnits {
  readonly axes: 'cad' | 'gis';
  readonly unit: 'm' | 'cm' | 'mm';
}

/** One line of a label: where its alignment puts it, its words, its alignment (none: its baseline's left). */
export interface LabelText {
  readonly p: Vec2;
  readonly text: string;
  readonly align?: TextAlign;
}

/** A label: its leader (the place, the elbow, the bar's end) when it has one, and its lines. */
export interface CoordinateLabel {
  readonly leader?: readonly Vec2[];
  readonly texts: readonly LabelText[];
}

/** The labels of some places, and how many got none (their template left them no line). */
export interface CoordinateLabels {
  readonly labels: readonly CoordinateLabel[];
  readonly skipped: number;
}

export const coordinatePlaces = op<(objects: readonly ListedObject[]) => LabelPlace[]>('coordinatePlaces');
export const coordinateLabels = op<(places: readonly LabelPlace[], options: LabelOptions, units: LabelUnits) => CoordinateLabels>('coordinateLabels');
