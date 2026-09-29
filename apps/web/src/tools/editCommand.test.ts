import { describe, expect, it } from 'vitest';
import { entityGeometry, type PolylineEntity } from '../model/entities';
import { reverseEntity, simplifyEntity } from '../model/ops/reshape';
import { entitiesEdit } from '../product/entitiesEdit';
import { editGeometry } from './editCommand';
import { pt, toolHarness } from './toolHarness';

/**
 * What a tool writes of a geometry the core computed (docs/adr/0142, 0143): the core has no elevations, and an
 * object it gives back still carries the old `zs` of its `rest`, which no longer fit its vertices once they were
 * dropped or turned round. `editGeometry` leaves them out, so each vertex takes its own from the objects the edit
 * names, by the command's rules.
 */
describe('editGeometry', () => {
  it('leaves out the elevations of the object the core gave back, and keeps the rest of the geometry', () => {
    const g = editGeometry({ kind: 'polygon', pts: [pt(0, 0), pt(4, 0), pt(4, 4)], zs: [1, 2, 3], holes: [{ pts: [pt(1, 1), pt(2, 1), pt(2, 2)], zs: [4, 5, 6] }], parts: [{ pts: [pt(9, 9), pt(12, 9), pt(12, 12)], zs: [7, 8, 9] }] } as never);
    expect(g).toEqual({ kind: 'polygon', pts: [pt(0, 0), pt(4, 0), pt(4, 4)], holes: [{ pts: [pt(1, 1), pt(2, 1), pt(2, 2)] }], parts: [{ pts: [pt(9, 9), pt(12, 9), pt(12, 12)] }] });
  });

  it('lets Sadeleştir write a polyline that has elevations (the old ones no longer fit the vertices left)', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'polyline', pts: [pt(0, 0), pt(5, 0.0001), pt(10, 0), pt(10, 10)], zs: [1, 2, 3, 4] }) as PolylineEntity;
    const r = simplifyEntity(e, 0.01)!;
    expect((r.entity as PolylineEntity).pts).toHaveLength(3);
    const result = entitiesEdit.execute({ doc: h.doc }, { operation: 'simplify', changes: [{ kind: 'update', uid: h.doc.uidOf(e.id)!, geometry: editGeometry(entityGeometry(r.entity)) }] });
    expect(result.status).toBe('completed');
    // The vertex that went took its elevation with it; the others keep theirs.
    expect((h.doc.get(e.id) as PolylineEntity).zs).toEqual([1, 3, 4]);
  });

  it('lets Yönü çevir turn the elevations with the vertices of a polyline', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'polyline', pts: [pt(0, 0), pt(10, 0), pt(10, 10)], zs: [1, 2, 3] }) as PolylineEntity;
    const turned = reverseEntity(e)!;
    const result = entitiesEdit.execute({ doc: h.doc }, { operation: 'reverse', changes: [{ kind: 'update', uid: h.doc.uidOf(e.id)!, geometry: editGeometry(entityGeometry(turned)) }] });
    expect(result.status).toBe('completed');
    const now = h.doc.get(e.id) as PolylineEntity;
    expect(now.pts).toEqual([pt(10, 10), pt(10, 0), pt(0, 0)]);
    expect(now.zs).toEqual([3, 2, 1]);
  });
});
