import type { AppContext } from '../app/context';
import type { AreaPart } from '../contracts/generated/AreaPart';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import type { PointPart } from '../contracts/generated/PointPart';
import { entityLength, type Entity, type PointEntity, type PolylineEntity } from '../model/entities';
import { areasOfEntity } from '../model/ops/areas';
import { uidOf, writeEdit } from './editCommand';

/**
 * Parçaları birleştir and Parçalara ayır for lines and points (docs/adr/0174 §4), beside the areas' (areaTools.ts):
 * lines and polylines become one multi-part polyline in the first one's place (a line's ends' elevations its part's),
 * points one multi-point object; a multi-part polyline or a multi-point object comes apart into its parts, the first
 * keeping its place and persistent id, the others new with its data. Touching lines are not chained: that is
 * Birleştir's. The desktop's is `kentos_interaction::line_parts`.
 */

/** What Parçaları birleştir says of a selection of more than one kind. */
export const MIXED = 'Parçaları birleştir aynı türden nesneleri birleştirir: alanları, çizgileri ya da noktaları.';
/** What it says of a single line or polyline, and of a single point. */
export const ONE_LINE = 'Parçaları birleştirmek için en az iki çizgi ya da çoklu çizgi seçin.';
export const ONE_POINT = 'Parçaları birleştirmek için en az iki nokta seçin.';

/** The selection by what Parçaları birleştir joins: how many objects enclose an area, the lines and the points. */
export interface Kinds {
  areas: number;
  lines: (PolylineEntity | Extract<Entity, { kind: 'line' }>)[];
  points: PointEntity[];
}

/** What the objects are to Parçaları birleştir; other kinds (text, arcs …) are none of them. */
export function kindsOf(list: readonly Entity[]): Kinds {
  const k: Kinds = { areas: 0, lines: [], points: [] };
  for (const e of list) {
    if (e.kind === 'point') k.points.push(e);
    else if (e.kind === 'line') k.lines.push(e);
    else if (e.kind === 'polyline' && (e.parts !== undefined || !areasOfEntity(e).length)) k.lines.push(e);
    else if (areasOfEntity(e).length) k.areas++;
  }
  return k;
}

/** How many of the three kinds the selection holds. */
export const present = (k: Kinds): number => (k.areas > 0 ? 1 : 0) + (k.lines.length ? 1 : 0) + (k.points.length ? 1 : 0);

/** The lines as one multi-part polyline in the first one's place: their parts in the selection's order, each object's own first. */
export function joinLines(ctx: AppContext, lines: Kinds['lines']): void {
  const parts: AreaPart[] = lines.flatMap((e): AreaPart[] => {
    if (e.kind === 'line') return [{ pts: [e.a, e.b], ...((e.za !== undefined || e.zb !== undefined) && { zs: [e.za ?? null, e.zb ?? null] }) }];
    const own: AreaPart = { pts: e.pts, ...(e.bulges && { bulges: e.bulges }), ...(e.zs && { zs: e.zs }) };
    return [own, ...(e.parts ?? []).map(({ holes: _h, ...q }) => q)];
  });
  const [first, ...rest] = parts;
  if (!first) return;
  const geometry: EditGeometry = { kind: 'polyline', pts: first.pts, ...(first.bulges && { bulges: first.bulges }), ...(first.zs && { zs: first.zs }), parts: rest };
  const head = lines[0];
  const uid = uidOf(ctx, head);
  const out = writeEdit(ctx, 'partsJoin', [
    head.kind === 'polyline' ? { kind: 'update', uid, geometry } : { kind: 'replace', uid, geometry, keepData: true },
    ...lines.slice(1).map((e): EntityEdit => ({ kind: 'remove', uid: uidOf(ctx, e) })),
  ]);
  if (!out) return;
  ctx.selection.set([head.id]);
  const now = ctx.doc.get(head.id);
  const total = ctx.format.length(now ? (entityLength(now) ?? 0) : 0);
  ctx.log.success(`${lines.length} çizgi tek çoklu çizgide birleşti: ${parts.length} parça, toplam ${total}.`);
}

/** The points as one multi-point object in the first one's place, each with its elevation, in the selection's order. */
export function joinPoints(ctx: AppContext, points: readonly PointEntity[]): void {
  const all: PointPart[] = points.flatMap((e) => [{ p: e.p, ...(e.z !== undefined && { z: e.z }) }, ...(e.parts ?? [])]);
  const [first, ...rest] = all;
  if (!first) return;
  const head = points[0];
  const geometry: EditGeometry = { kind: 'point', p: first.p, ...(first.z != null && { z: first.z }), parts: rest };
  const out = writeEdit(ctx, 'partsJoin', [
    { kind: 'update', uid: uidOf(ctx, head), geometry },
    ...points.slice(1).map((e): EntityEdit => ({ kind: 'remove', uid: uidOf(ctx, e) })),
  ]);
  if (!out) return;
  ctx.selection.set([head.id]);
  ctx.log.success(`${points.length} nokta tek nesnede birleşti: ${all.length} nokta.`);
}

/**
 * A multi-part polyline's or multi-point object's changes for Parçalara ayır: its own first part in its place, each
 * other one new with its data (`add`, keepData); with how many are new. Null for anything else.
 */
export function splitOf(ctx: AppContext, e: Entity): { changes: EntityEdit[]; made: number } | null {
  const uid = uidOf(ctx, e);
  const add = (geometry: EditGeometry): EntityEdit => ({ kind: 'add', from: uid, geometry, keepData: true });
  if (e.kind === 'polyline' && e.parts?.length) {
    const own: EditGeometry = { kind: 'polyline', pts: e.pts, ...(e.bulges && { bulges: e.bulges }), ...(e.zs && { zs: e.zs }) };
    const parts = e.parts.map((q) => add({ kind: 'polyline', pts: q.pts, ...(q.bulges && { bulges: q.bulges }), ...(q.zs && { zs: q.zs }) }));
    return { changes: [{ kind: 'update', uid, geometry: own }, ...parts], made: parts.length };
  }
  if (e.kind === 'point' && e.parts?.length) {
    const own: EditGeometry = { kind: 'point', p: e.p, ...(e.z !== undefined && { z: e.z }) };
    const parts = e.parts.map((q) => add({ kind: 'point', p: q.p, ...(q.z != null && { z: q.z }) }));
    return { changes: [{ kind: 'update', uid, geometry: own }, ...parts], made: parts.length };
  }
  return null;
}
