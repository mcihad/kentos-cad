import type { Entity, EntityKind, NewEntity } from '../../../model/entities';
import type { LayerStyle } from '../../../model/layers';
import type { EntityGeometry } from '../../../model/entities';
import type { Feedback } from '../../types';

/**
 * What the geometry tools share (docs/adr/0201 §1; the desktop's `builtin/geometry/mod.rs`): the kinds they take,
 * their output layers' look, a result as a new object, and the notes every run gives about its inputs.
 */

/** The kinds that are areas: a closed area, a circle, a whole ellipse, a closed curve (an open ellipse or curve is a path). */
export const AREA_KINDS: readonly EntityKind[] = ['polygon', 'circle', 'ellipse', 'spline'];

/** What the geometry tools take: areas, paths (lines, polylines, arcs, an ellipse's arc, an open curve) and points. */
export const GEO_KINDS: readonly EntityKind[] = ['polygon', 'circle', 'ellipse', 'spline', 'line', 'polyline', 'arc', 'point'];

/** What has written vertices to check and to repair: areas, polylines and lines. */
export const VERTEX_KINDS: readonly EntityKind[] = ['polygon', 'polyline', 'line'];

/** What Sadeleştir thins: areas and polylines. */
export const SIMPLIFY_KINDS: readonly EntityKind[] = ['polygon', 'polyline'];

/** The scopes the geometry tools offer, a layer first. */
export const GEO_SCOPES = ['layer', 'selection', 'visible', 'all'] as const;

/** An output layer's look when the tool makes it: its colour, a 0.25 mm line, the colour faint inside areas. */
export const outputStyle = (color: string): Partial<LayerStyle> => ({ color, lineType: 'continuous', lineWeight: 0.25, fill: `${color}26` });

/** A result as a new object on the output layer, with its attributes. */
export const newObject = (shape: EntityGeometry, layerId: string, attrs: Record<string, string>): NewEntity => ({ ...shape, layerId, attrs }) as NewEntity;

const held = (zs: readonly (number | null)[] | undefined): boolean => !!zs?.some((z) => z !== null && z !== undefined);

/** Whether an object has elevations: a point's, a line's ends', a path's or an area's vertices' (docs/adr/0142). */
export function hasElevation(e: Entity): boolean {
  switch (e.kind) {
    case 'point':
      return e.z !== undefined || !!e.parts?.some((q) => q.z !== undefined);
    case 'line':
      return e.za !== undefined || e.zb !== undefined;
    case 'polyline':
    case 'polygon':
      return held(e.zs) || !!e.holes?.some((h) => held(h.zs)) || !!e.parts?.some((q) => held(q.zs) || !!q.holes?.some((h) => held(h.zs)));
    default:
      return false;
  }
}

/**
 * The notes about a run's inputs (§1): ellipses and curves that entered as chords within 0.1 mm (the inputs' and the
 * other side's, each object once), and inputs whose elevations the results do not carry (`points`: a point's
 * elevation is carried, as Koordinat sistemine dönüştür does).
 */
export function inputNotes(feedback: Feedback, inputs: readonly Entity[], others: readonly Entity[] = [], points = false): void {
  const seen = new Set<number>();
  let chorded = 0;
  for (const e of [...inputs, ...others]) {
    if (seen.has(e.id)) continue;
    seen.add(e.id);
    if (e.kind === 'ellipse' || e.kind === 'spline') chorded++;
  }
  if (chorded) feedback.warn(`${chorded} elips ya da eğri 0,1 mm içinde doğru parçalarına çevrildi.`);
  const heights = inputs.filter((e) => hasElevation(e) && !(points && e.kind === 'point')).length;
  if (heights) feedback.warn(`${heights} nesnenin kotları sonuca taşınmadı.`);
}

/** The note about objects whose result is nothing (§1). */
export function emptyNote(count: number, feedback: Feedback): void {
  if (count) feedback.warn(`${count} nesnenin sonucu boş; yazılmadı.`);
}
