import type { AppContext } from '../app/context';
import type { AreaPart } from '../contracts/generated/AreaPart';
import type { EntityGeometry } from '../contracts/generated/EntityGeometry';
import type { RingGeometry } from '../contracts/generated/RingGeometry';
import type { Bounds } from '../model/geometry';
import type { Area, Ring } from '../model/geom/overlay';
import { netArea } from '../model/geom/region';
import { adjoinWork } from '../model/ops/adjoin';

/**
 * The overlap control (docs/adr/0162 §1–§2): while it is on, a new area drawn by its outline loses what overlaps the
 * visible areas of the overlap layers (its own layer, or the chosen ones) before it is written. What is left is the
 * core's (`adjoinAvoid`); the tools write it as one object, its holes and parts as they are. The desktop's is
 * `crates/native/interaction/src/overlap.rs`; both play fixtures/interaction/v1/overlap.json.
 */

/** The layers whose areas a new area going on `layerId` must not overlap; none while the mode is Serbest. */
export function overlapLayers(ctx: AppContext, layerId: string): string[] {
  switch (ctx.settings.overlap.value) {
    case 'layer':
      return [layerId];
    case 'layers':
      return [...ctx.settings.overlapLayers.value];
    default:
      return [];
  }
}

/** A ring's box, generous on arcs: an arc stays within twice its radius of its ends. */
function ringBox(r: Ring): Bounds {
  const b = { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity };
  r.pts.forEach((p, i) => {
    const q = r.pts[(i + 1) % r.pts.length];
    const k = Math.abs(r.bulges?.[i] ?? 0);
    const chord = Math.hypot(q.x - p.x, q.y - p.y);
    // Twice the radius of the arc of bulge k on this chord (none for a straight edge).
    const pad = k ? (chord * (1 + k * k)) / (2 * k) : 0;
    b.minX = Math.min(b.minX, p.x - pad);
    b.minY = Math.min(b.minY, p.y - pad);
    b.maxX = Math.max(b.maxX, p.x + pad);
    b.maxY = Math.max(b.maxY, p.y + pad);
  });
  return b;
}

/** What the overlap control leaves of a new area: what to write (none when it was covered) and how many it overlapped. */
export interface Clipped {
  areas: Area[];
  overlapped: number;
}

/**
 * The overlap control on a new area going on `layerId`: null to write it as drawn (the mode is Serbest, or it
 * overlaps nothing); else what is left. The neighbours are the visible objects of the overlap layers whose box meets
 * the area's.
 */
export function clipNewArea(ctx: AppContext, area: Area, layerId: string): Clipped | null {
  return clipNewAreas(ctx, [area], layerId);
}

/**
 * The overlap control on the parts of one new area (Bitişik alan's, §3): as `clipNewArea`, the neighbours those whose
 * box meets the parts' box, each counted once.
 */
export function clipNewAreas(ctx: AppContext, areas: readonly Area[], layerId: string): Clipped | null {
  const layers = new Set(overlapLayers(ctx, layerId));
  if (!layers.size) {
    if (ctx.settings.overlap.value === 'layers') ctx.log.warn('Seçili katmanlarda önle kipinde seçili katman yok: alan olduğu gibi yazıldı. Katmanları Çakışma hücresinin menüsünden seçin.');
    return null;
  }
  const box = areas.map((a) => ringBox(a.outer)).reduce((b, r) => ({ minX: Math.min(b.minX, r.minX), minY: Math.min(b.minY, r.minY), maxX: Math.max(b.maxX, r.maxX), maxY: Math.max(b.maxY, r.maxY) }));
  const neighbours = ctx.view.entitiesIn(box).filter((e) => layers.has(e.layerId));
  if (!neighbours.length) return null;
  const work = adjoinWork(neighbours);
  try {
    const overlapped = new Set<number>();
    const left: Area[] = [];
    for (const area of areas) {
      const got = work.avoid(area);
      got.overlapped.forEach((k) => overlapped.add(k));
      left.push(...got.areas);
    }
    return overlapped.size ? { areas: left, overlapped: overlapped.size } : null;
  } finally {
    work.free();
  }
}

const ring = (r: Ring): RingGeometry => ({ pts: r.pts, ...(r.bulges && { bulges: r.bulges }) });

/** The areas left as one area's geometry: the first its own fields, the others its parts (docs/adr/0143). */
export function clippedGeometry(areas: readonly Area[]): EntityGeometry {
  const [first, ...rest] = areas;
  const parts: AreaPart[] = rest.map((a) => ({ ...ring(a.outer), ...(a.holes.length && { holes: a.holes.map(ring) }) }));
  return {
    kind: 'polygon',
    ...ring(first.outer),
    ...(first.holes.length && { holes: first.holes.map(ring) }),
    ...(parts.length && { parts }),
  } as EntityGeometry;
}

/** The overlap control's report: what it took, or that nothing was left to write. */
export function sayClipped(ctx: AppContext, clipped: Clipped): void {
  if (!clipped.areas.length) ctx.log.warn('Yeni alan komşu alanların içinde kalıyor; alan eklenmedi.');
  else ctx.log.info(`Çakışma önlendi: ${clipped.overlapped} komşu alanla örtüşen kısım çıkarıldı.`);
}

/** The area written: what is left, every part, less its holes. */
export const writtenArea = (areas: readonly Area[]): number => areas.reduce((sum, a) => sum + netArea(a), 0);
