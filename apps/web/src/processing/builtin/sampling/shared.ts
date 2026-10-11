import type { Entity, NewEntity } from '../../../model/entities';
import type { Vec2 } from '../../../model/geometry';
import type { Feedback } from '../../types';

/**
 * What the Örnekleme tools share (docs/adr/0220; the desktop's `builtin/sampling/mod.rs`): their scopes, the path
 * kinds, the note about curves taken as chords, a point on the output layer.
 */

/** The scopes the tools offer, a layer first. */
export const SAMPLE_SCOPES = ['layer', 'selection', 'visible', 'all'] as const;

/** The kinds that are paths (Rastgele noktalar on lines, Hat boyunca nokta). */
export const LINE_KINDS = ['line', 'polyline', 'arc'] as const;

/** The note about curves taken as chords (an ellipse, a curve). */
export function curvedNote(feedback: Feedback, inputs: readonly Entity[]): void {
  const c = inputs.filter((e) => e.kind === 'ellipse' || e.kind === 'spline').length;
  if (c) feedback.warn(`${c} nesnenin eğrileri 0,1 mm'lik kirişlerle alındı.`);
}

/** A point on the output layer, with its elevation when it has one. */
export function pointOn(layerId: string, attrs: Record<string, string>, p: Vec2, z: number | null | undefined): NewEntity {
  return (z === null || z === undefined ? { kind: 'point', layerId, attrs, p } : { kind: 'point', layerId, attrs, p, z }) as NewEntity;
}

/** A run that wrote these objects. */
export const written = (add: NewEntity[], summary: string) => ({ changes: { add }, outputs: { count: add.length }, summary });
