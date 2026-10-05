import { op } from '../../wasm/core';

/**
 * Kayıtlı ölçüler (docs/adr/0180): the core's `ops::cogo`, one rule for both platforms. A line's and an arc's recorded
 * measurements are four attributes beside the geometry (ArcGIS's COGO fields): measured from the drawing, checked
 * against it, recorded from a typed polar point. The independent reference is scripts/fixtures/cogo_cases.py.
 */

export const COGO_SEMT = 'Kayıtlı semt';
export const COGO_LENGTH = 'Kayıtlı uzunluk';
export const COGO_RADIUS = 'Kayıtlı yarıçap';
export const COGO_ARC = 'Kayıtlı yay uzunluğu';

/** What the drawing gives: semt in grads (0 ≤ semt < 400, north clockwise), lengths in metres; radius and arc an arc's. */
export interface CogoMeasured {
  readonly semt: number;
  readonly length: number;
  readonly radius?: number;
  readonly arc?: number;
}

export type CogoField = 'semt' | 'length' | 'radius' | 'arc';

/** A recorded value against the measured one: the semt's difference in cc, the lengths' in metres; none: no number. */
export interface CogoItem {
  readonly field: CogoField;
  readonly recorded: string;
  readonly measured: number;
  readonly difference?: number;
  readonly over: boolean;
}

export interface CogoFinding {
  readonly status: 'ok' | 'differs' | 'unreadable';
  readonly items: readonly CogoItem[];
}

/** A line's or an arc's measurements (the shape as `geometryOf` gives it); null for another kind. */
export const cogoMeasure = op<(shape: Record<string, unknown>) => CogoMeasured | null>('cogoMeasure');

/** An object's recorded values checked; null when it is no line or arc or has none of them. */
export const cogoCheck = op<
  (shape: Record<string, unknown>, attrs: Readonly<Record<string, string>>, tolerances: { length: number; cc: number }) => CogoFinding | null
>('cogoCheck');

/** The recorded values a typed polar point gives a line; null when the text is no polar point. */
export const cogoRecord = op<(text: string, convention: 'gis' | 'cad', angleUnit: string, lengthUnit: string) => Record<string, string> | null>('cogoRecord');

/** Each field's attribute name. */
export const COGO_ATTRIBUTE: Record<CogoField, string> = { semt: COGO_SEMT, length: COGO_LENGTH, radius: COGO_RADIUS, arc: COGO_ARC };
