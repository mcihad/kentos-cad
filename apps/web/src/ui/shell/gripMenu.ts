import type { AppContext } from '../../app/context';
import type { EditOperation } from '../../contracts/generated/EditOperation';
import type { EntityGeometry as EditGeometry } from '../../contracts/generated/EntityGeometry';
import type { EntityGeometry, PolylineEntity } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';
import { bulgeAt, bulgeRingArea, isArcBulge, segmentMid } from '../../model/geom/bulge';
import { gripPart, holeGrip, midGripSegment } from '../../model/ops/grips';
import { replacePart, splitParts } from '../../model/ops/parts';
import { insertVertex, removeVertex } from '../../model/ops/vertex';
import { withoutElevations } from '../../product/elevation';
import { EDIT_LABEL, geometryOf } from '../../product/entitiesEdit';
import { uidOf, writeEdit } from '../../tools/editCommand';
import { neighbours, sayNeighbours } from '../../tools/neighbours';
import type { MenuItem } from '../widgets/PopupMenu';

/**
 * The grip actions of the right-button menu over the drawing: the grip under the cursor of a selected polyline or
 * area (docs/adr/0074). A multi-part area's grip acts on its own part, the other parts kept (docs/adr/0143); the
 * heading counts in that part, as the desktop's does (`crates/native/interaction/src/grip_menu.rs`). With
 * Topolojik düzenleme on, the shared corners and edges of the objects around go with it (docs/adr/0160).
 */

/** Actions for the grip under the cursor of a selected polyline or polygon. */
export function gripItems(ctx: AppContext, screen: Vec2): MenuItem[] {
  const hit = ctx.view.gripAt(screen);
  const e = hit && ctx.doc.get(hit.id);
  // Hole vertices only move by dragging; editing a hole's shape needs Patlat.
  if (!hit || !e || (e.kind !== 'polyline' && e.kind !== 'polygon') || holeGrip(e, hit.index)) return [];
  // The grip's own part (0: the area's own first) and its place there; the part is edited as an object of its own.
  const multi = !!e.parts?.length;
  const at = multi ? gripPart(e, hit.index) : { part: 0, index: hit.index };
  if (!at) return [];
  const own = multi ? splitParts(e)[at.part] : e;
  // The mid grip's edge counts within the grip's own part too.
  const seg = midGripSegment(e, hit.index);
  // Through the product command cad.entities.edit, the step named after the operation. The part's geometry is kept
  // but for what the edit gives (its holes stay: the core's answer for a ring has none, and says so with an
  // undefined `holes`, which must not take them away); its bulges only when the edit gives them. The area's other
  // parts stay: the edited part goes back in its place. No elevations go with it: each vertex takes its own by the
  // command's rules (docs/adr/0142).
  const apply = (operation: EditOperation, r: { geometry: EntityGeometry } | { error: string }) => {
    if ('error' in r) return ctx.log.warn(r.error);
    const given = Object.fromEntries(Object.entries(r.geometry).filter(([, value]) => value !== undefined));
    const part = { ...geometryOf(own as unknown as EditGeometry), bulges: undefined, ...given } as unknown as EditGeometry;
    let geometry = part;
    if (multi) {
      const whole = replacePart(e, at.part, { ...own, ...part } as unknown as typeof e);
      if (!whole) return ctx.log.warn('Parça kapalı alan olarak kalmalı.');
      geometry = geometryOf(whole as unknown as EditGeometry) as unknown as EditGeometry;
    }
    // The neighbours sharing what changed go in the same step (docs/adr/0160 §3).
    const follow = neighbours(ctx, e, { ...e, ...(geometry as unknown as EntityGeometry) } as typeof e);
    if (writeEdit(ctx, operation, [{ kind: 'update', uid: uidOf(ctx, e), geometry: withoutElevations(geometry) }, ...(follow?.changes ?? [])])) {
      ctx.log.success(`${EDIT_LABEL[operation]}: tamam.`);
      sayNeighbours(ctx, follow);
    }
  };
  if (seg === null) {
    return [
      { kind: 'header', label: `Köşe ${at.index + 1}` },
      { label: 'Köşeyi sil', icon: 'erase', run: () => apply('vertexRemove', removeVertex(own, at.index)) },
      { kind: 'separator' },
    ];
  }
  const n = own.pts.length;
  const a = own.pts[seg];
  const b = own.pts[(seg + 1) % n];
  const arc = isArcBulge(bulgeAt(own.bulges, seg));
  return [
    { kind: 'header', label: `Kenar ${seg + 1}` },
    { label: 'Ortasına köşe ekle', icon: 'vertex', run: () => apply('vertexAdd', insertVertex(own, seg, segmentMid(a, b, bulgeAt(own.bulges, seg)))) },
    arc
      ? { label: 'Düz kenar yap', icon: 'line', run: () => apply('straightEdge', withBulge(own, seg, 0)) }
      : { label: 'Yaya dönüştür', icon: 'arc', run: () => apply('arcEdge', withBulge(own, seg, outwardBulge(own))) },
    { kind: 'separator' },
  ];
}

/**
 * A gentle arc (sagitta = a quarter of the chord) bowing out of a ring, or
 * to the right of an open path; its mid grip then shapes it further.
 */
function outwardBulge(e: PolylineEntity): number {
  if (e.kind !== 'polygon') return 0.5;
  // Outside of a counter-clockwise ring is right of travel, where a positive bulge bows.
  return bulgeRingArea(e.pts, e.bulges) > 0 ? 0.5 : -0.5;
}

function withBulge(e: PolylineEntity, seg: number, bulge: number): { geometry: EntityGeometry } {
  const bulges = e.pts.map((_, i) => (i === seg ? bulge : bulgeAt(e.bulges, i)));
  return { geometry: { kind: e.kind, pts: e.pts, ...(bulges.some(isArcBulge) && { bulges }) } };
}
