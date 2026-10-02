import { CoreAdjoinWork, op, writeArgs } from '../../wasm/core';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';
import type { Area } from '../geom/overlay';

/**
 * Bitişik alan and the overlap control (docs/adr/0162): the core's `ops::adjoin`, one for both platforms. `avoid`
 * takes from a new area the neighbours that overlap it (§2); `fill` is the region a drawn path closes with the
 * neighbours (§3). Neighbours are the entities that enclose an area (the area tools' kinds). The independent reference
 * is scripts/fixtures/adjoin_cases.py.
 */

/** What the overlap control leaves of a new area: none when it is covered, more than one when it is cut apart. */
export interface Avoided {
  areas: Area[];
  /** The neighbours whose inside meets the area's, by their places in the list given. */
  overlapped: number[];
}

/** The new area less the neighbours among `neighbours` that overlap it. */
export const adjoinAvoid = op<(area: Area, neighbours: readonly Entity[]) => Avoided>('adjoinAvoid');

/** As `adjoinAvoid`, the neighbours given as their areas (each object's parts). */
export const adjoinAvoidAreas = op<(area: Area, neighbours: readonly (readonly Area[])[]) => Avoided>('adjoinAvoidAreas');

/** The region the open path (`bulges` one per segment, or null) closes with the neighbours; none when it closes none. */
export const adjoinFill = op<(pts: readonly Vec2[], bulges: readonly number[] | null, neighbours: readonly Entity[]) => Area[]>('adjoinFill');

/** As `adjoinFill`, the neighbours given as their areas. */
export const adjoinFillAreas = op<(pts: readonly Vec2[], bulges: readonly number[] | null, neighbours: readonly (readonly Area[])[]) => Area[]>('adjoinFillAreas');

/** The neighbours kept in the core: taken once per view and drawing, asked on every pointer move. */
export interface AdjoinWork {
  /** How many edges the neighbours have (the preview's budget). */
  readonly edgeCount: number;
  fill(pts: readonly Vec2[], bulges: readonly number[] | null): Area[];
  avoid(area: Area): Avoided;
  free(): void;
}

/** The neighbours among `entities`. */
export function adjoinWork(entities: readonly Entity[]): AdjoinWork {
  const core = CoreAdjoinWork.ofEntities(writeArgs(entities));
  return {
    edgeCount: core.edgeCount(),
    fill: (pts, bulges) => {
      const xy = new Float64Array(pts.length * 2);
      pts.forEach((p, i) => {
        xy[2 * i] = p.x;
        xy[2 * i + 1] = p.y;
      });
      return core.fill(xy, new Float64Array(bulges ?? [])) as Area[];
    },
    avoid: (area) => core.avoid(writeArgs(area)) as Avoided,
    free: () => core.free(),
  };
}
