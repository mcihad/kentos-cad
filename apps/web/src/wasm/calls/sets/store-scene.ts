import { CadDocument } from '../../../model/document';
import type { NewEntity } from '../../../model/entities';
import type { Bounds, Vec2 } from '../../../model/geometry';
import { LayerStore, type LayerInit } from '../../../model/layers';
import type { SnapKind } from '../../../viewport/picking';
import type { Gen } from '../harness';
import { entity } from './p5-entities';

/**
 * Scenes and cursors for the geometry store (docs/adr/0008, S1): objects of
 * every kind on layers that are hidden, locked, edge-pick only or not in
 * the tree at all; parcels on a grid so edges are shared and hatches fill
 * some of them; and cursors near vertices, edges or anywhere. The fixture
 * recorders (scripts/fixtures/record-store*.test.ts) and the store's own
 * tests (src/viewport/picking.test.ts, src/processing/runs.test.ts) use them.
 */

export const SNAP_KINDS: SnapKind[] = ['endpoint', 'midpoint', 'center', 'node', 'quadrant', 'intersection', 'perpendicular', 'tangent', 'nearest'];
const v = (x: number, y: number): Vec2 => ({ x, y });

/** Layers of every state; some carry label styles of every placement, the rest fall back to the kind's default. */
export const SCENE_LAYERS: LayerInit[] = [
  { id: 'a', name: 'A', style: { label: { placement: 'center', size: 10, minFeaturePx: 20 } } },
  { id: 'b', name: 'Yalnız kenar', style: { pickInterior: false, label: { placement: 'along', size: 9, minScale: 0.5, maxScale: 30 } } },
  { id: 'kilitli', name: 'Kilitli', locked: true, style: { label: { placement: 'corner', size: 9 } } },
  { id: 'gizli', name: 'Gizli', visible: false },
  { id: 'grup', name: 'Grup', children: [{ id: 'g1', name: 'G1', style: { label: { placement: 'beside', size: 9, minScale: 2 } } }, { id: 'g2', name: 'G2', style: { pickInterior: false } }] },
];

/** Screen scales (px per metre) from an overview to a close-up. */
export const SCENE_SCALES = [0.05, 0.3, 1, 3, 10, 40];
/** Layers objects land on; 'yok' is not in the tree at all. */
const ON: string[] = ['a', 'a', 'a', 'b', 'kilitli', 'gizli', 'g1', 'g2', 'yok'];
/** Layer nodes an edit may hide or lock. */
export const SCENE_LAYER_IDS = ['a', 'b', 'kilitli', 'gizli', 'grup', 'g1', 'g2'];

/** A random object on a random layer; parcels on a grid share edges, hatches fill some of them. */
export function sceneEntity(g: Gen): NewEntity {
  const layerId = g.pick(ON);
  const k = g.int(0, 9);
  if (k < 3) {
    const c = g.gridPt(10, 6);
    const w = g.pick([10, 20]);
    const h = g.pick([10, 20]);
    const ring = [v(c.x, c.y), v(c.x + w, c.y), v(c.x + w, c.y + h), v(c.x, c.y + h)];
    if (k === 2) return { kind: 'hatch', layerId, attrs: {}, ring, pattern: { type: g.pick(['solid', 'lines'] as const), angle: 45, spacing: 1 } };
    const holes = g.chance(0.2) ? [{ pts: [v(c.x + 2, c.y + 2), v(c.x + 4, c.y + 2), v(c.x + 4, c.y + 4), v(c.x + 2, c.y + 4)] }] : undefined;
    return { kind: 'polygon', layerId, attrs: {}, pts: ring, ...(holes ? { holes } : {}), ...(g.chance(0.3) ? { label: String(g.int(1, 99)) } : {}) };
  }
  if (k === 3 && g.chance(0.3))
    return g.pick<NewEntity>([
      { kind: 'polyline', layerId, attrs: {}, pts: [] },
      { kind: 'polyline', layerId, attrs: {}, pts: [g.pt()] },
      { kind: 'line', layerId, attrs: {}, a: v(1, 1), b: v(1, 1) },
      { kind: 'circle', layerId, attrs: {}, c: g.pt(), r: 0 },
    ]);
  const { id: _id, ...e } = entity(g);
  return { ...e, layerId } as NewEntity;
}

/** A document of `n` scene objects in one frame (near the origin or in a TM zone). */
export function sceneDocument(g: Gen, n: number): CadDocument {
  g.frame();
  const doc = new CadDocument({ name: 'Depo', layers: new LayerStore(SCENE_LAYERS, 'a'), origin: v(0, 0) });
  doc.load(Array.from({ length: n }, () => sceneEntity(g)));
  return doc;
}

/** A cursor near something: an object's vertex or anchor, jittered, or anywhere in the drawing. */
export function sceneCursor(g: Gen, doc: CadDocument): Vec2 {
  const all = [...doc.all()];
  const e = all.length ? g.pick(all) : null;
  const b = doc.bounds() ?? { minX: -100, minY: -100, maxX: 100, maxY: 100 };
  if (!e || g.chance(0.2)) return v(g.num(b.minX, b.maxX), g.num(b.minY, b.maxY));
  const at = 'pts' in e && e.pts.length ? g.pick(e.pts) : 'ring' in e ? g.pick(e.ring) : 'c' in e && e.c ? e.c : 'a' in e ? e.a : 'p' in e ? e.p : v(0, 0);
  const j = g.pick([0, 0, 0.001, 0.05, 1, 5]);
  return v(at.x + g.num(-j, j), at.y + g.num(-j, j));
}

/** A rectangle around a point, from a tenth of a metre to five kilometres. */
export function sceneRect(g: Gen, p: Vec2, sizes = [0.1, 3, 20, 200, 5000]): Bounds {
  const w = g.pick(sizes);
  const x = p.x - g.num(0, w);
  const y = p.y - g.num(0, w);
  return { minX: x, minY: y, maxX: x + g.num(0, w), maxY: y + g.num(0, w) };
}

/** Pick and snap tolerances in world units (a pick aperture at scales from 1:50 to 1:5000). */
export const SCENE_TOLERANCES = [0.01, 0.2, 1, 5, 25];

/** A random set of snap kinds (often all of them). */
export function sceneKinds(g: Gen): Set<SnapKind> {
  return new Set(g.chance(0.4) ? SNAP_KINDS : SNAP_KINDS.filter(() => g.chance(0.5)));
}
