import { describe, expect, it } from 'vitest';
import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import { entityGeometry, type PolylineEntity } from '../model/entities';
import { insertVertex, removeVertex } from '../model/ops/vertex';
import { reverseEntity } from '../model/ops/reshape';
import { editGeometry } from '../tools/editCommand';
import { pt, toolHarness } from '../tools/toolHarness';
import { entitiesEdit, geometryOf } from './entitiesEdit';

/**
 * `cad.entities.edit` on a multi-part area (docs/adr/0143), beside the shared cases (fixtures/commands/v1): the
 * command writes the whole area, every part with its holes and its elevations, and what an edit computes carries
 * the elevations of each part by the command's rules.
 */
const square = (x: number, y: number, side: number) => [pt(x, y), pt(x + side, y), pt(x + side, y + side), pt(x, y + side)];

function scene() {
  const h = toolHarness();
  const area = h.add({
    kind: 'polygon',
    pts: square(0, 0, 10),
    zs: [1, 2, 3, 4],
    holes: [{ pts: square(2, 2, 2), zs: [10, 11, 12, 13] }],
    parts: [{ pts: square(20, 0, 10), zs: [5, 6, 7, 8], holes: [{ pts: square(24, 4, 2), zs: [20, 21, 22, 23] }] }],
  }) as PolylineEntity;
  const write = (operation: Parameters<typeof entitiesEdit.execute>[1]['operation'], geometry: EditGeometry) =>
    entitiesEdit.execute({ doc: h.doc }, { operation, changes: [{ kind: 'update', uid: h.doc.uidOf(area.id)!, geometry }] });
  const now = () => h.doc.get(area.id) as PolylineEntity;
  return { h, area, write, now };
}

describe('the geometry of a multi-part area is the whole area', () => {
  it('writes every part with its arcs, holes and elevations as given', () => {
    const { area, write, now } = scene();
    const geometry = { ...geometryOf(area as unknown as EditGeometry), parts: [{ pts: square(20, 0, 12), bulges: [0, 0.3, 0, 0], zs: [50, 51, 52, 53] }] } as unknown as EditGeometry;
    expect(write('properties', geometry).status).toBe('completed');
    expect(now().parts).toEqual([{ pts: square(20, 0, 12), bulges: [0, 0.3, 0, 0], zs: [50, 51, 52, 53] }]);
    expect(now().zs).toEqual([1, 2, 3, 4]);
    expect(now().holes).toEqual(area.holes);
  });

  it('refuses elevations of the wrong count in a hole of a part, and says where', () => {
    const { area, write, now } = scene();
    const geometry = geometryOf(area as unknown as EditGeometry) as unknown as PolylineEntity;
    geometry.parts![0].holes![0].zs = [1, 2];
    const r = write('elevation', geometry as unknown as EditGeometry);
    expect(r).toMatchObject({ status: 'failed', error: { code: 'invalid_elevations', path: 'changes[0].geometry.parts[0].holes[0].zs' } });
    expect(now()).toEqual(area);
  });

  it('holds the elevations of a part all null as none: neither its own nor its holes’ are written', () => {
    const { area, write, now } = scene();
    const geometry = geometryOf(area as unknown as EditGeometry) as unknown as PolylineEntity;
    geometry.parts![0].zs = [null, null, null, null];
    geometry.parts![0].holes![0].zs = [null, null, null, null];
    expect(write('elevation', geometry as unknown as EditGeometry).status).toBe('completed');
    expect('zs' in now().parts![0]).toBe(false);
    expect('zs' in now().parts![0].holes![0]).toBe(false);
    expect(now().zs).toEqual([1, 2, 3, 4]);
  });

  it('shows the parts in its plan', () => {
    const { h, area } = scene();
    const geometry = { ...geometryOf(area as unknown as EditGeometry), parts: [{ pts: square(40, 0, 5) }] } as unknown as EditGeometry;
    const plan = entitiesEdit.plan({ doc: h.doc }, { operation: 'properties', changes: [{ kind: 'update', uid: h.doc.uidOf(area.id)!, geometry }] });
    expect(plan.status).toBe('completed');
    if (plan.status !== 'completed') return;
    expect((plan.output.changed[0] as unknown as PolylineEntity).parts).toEqual([{ pts: square(40, 0, 5) }]);
  });
});

describe('the elevations of an edit’s new vertices, part by part', () => {
  it('a vertex added on an edge of the second part takes the elevation along that edge; the rest keep theirs', () => {
    const { area, write, now } = scene();
    // Edges count part after part: the first part's ring 0-3 and its hole 4-7, then the second part's ring from 8.
    const added = insertVertex(area, 8, pt(25, 0));
    if (!('geometry' in added)) throw new Error(added.error);
    expect(write('vertexAdd', editGeometry(added.geometry)).status).toBe('completed');
    expect(now().parts![0].pts).toEqual([pt(20, 0), pt(25, 0), ...square(20, 0, 10).slice(1)]);
    expect(now().parts![0].zs).toEqual([5, 5.5, 6, 7, 8]);
    expect(now().zs).toEqual([1, 2, 3, 4]);
    expect(now().holes![0].zs).toEqual([10, 11, 12, 13]);
    expect(now().parts![0].holes![0].zs).toEqual([20, 21, 22, 23]);
  });

  it('a vertex removed from the second part leaves the others their elevations', () => {
    const { area, write, now } = scene();
    const removed = removeVertex(area, 5);
    if (!('geometry' in removed)) throw new Error(removed.error);
    expect(write('vertexRemove', editGeometry(removed.geometry)).status).toBe('completed');
    expect(now().parts![0].pts).toEqual([pt(20, 0), pt(30, 10), pt(20, 10)]);
    expect(now().parts![0].zs).toEqual([5, 7, 8]);
    expect(now().zs).toEqual([1, 2, 3, 4]);
  });

  it('turning an area round turns the elevations of every part with their vertices (the geometry carries none)', () => {
    const { h, area, write, now } = scene();
    const turned = reverseEntity(area)!;
    // What the core gives back still carries the old, unturned `zs`; the command must not write them.
    expect((turned as PolylineEntity).zs).toEqual([1, 2, 3, 4]);
    expect(write('reverse', editGeometry(entityGeometry(turned))).status).toBe('completed');
    const at = (pts: { x: number; y: number }[], zs: (number | null)[] | undefined) => pts.map((q, i) => [q.x, q.y, zs?.[i]]);
    const before = new Map([
      ...at(area.pts, area.zs),
      ...at(area.holes![0].pts, area.holes![0].zs),
      ...at(area.parts![0].pts, area.parts![0].zs),
      ...at(area.parts![0].holes![0].pts, area.parts![0].holes![0].zs),
    ].map(([x, y, z]) => [`${x},${y}`, z]));
    const e = now();
    const rings = [[e.pts, e.zs], [e.holes![0].pts, e.holes![0].zs], [e.parts![0].pts, e.parts![0].zs], [e.parts![0].holes![0].pts, e.parts![0].holes![0].zs]] as const;
    for (const [pts, zs] of rings) for (const [x, y, z] of at([...pts], zs ?? undefined)) expect(z, `${x},${y}`).toBe(before.get(`${x},${y}`));
    // And the rings do run the other way round.
    expect(e.pts[0]).not.toEqual(area.pts[0]);
    expect(h.doc.undo()).toBe('Yönü çevir');
  });
});
