import { crsBySrid } from '../geo/crs';
import { crsTransformIn, systemOf, type DatumChoice, type System } from './geom/crsTransform';
import type { Vec2 } from './geometry';
import { datumChoices, ownSystem, type CrsSettings } from './projectCrs';

/**
 * Between the project's system and a map service's (docs/adr/0208 §4; the desktop's `services/systems.rs`): the same
 * system takes a point as it is; another goes through the core's transformation with the project's datum choices
 * (the one Koordinat dönüştür uses). A project without a system (a local one) reaches no service: nothing is guessed
 * (CLAUDE.md §5). Servisten veri al and Servis bilgisi ask with it; the tiles' meshes are the services worker's.
 */

export type ServicePair = { same: true } | { same: false; ours: System; theirs: System; choices: DatumChoice[] };

/** The ways between the project and the service system `srid`, or why there are none. */
export function servicePair(s: CrsSettings, srid: number): ServicePair | { error: string } {
  const crs = crsBySrid(srid);
  const theirs = crs ? systemOf(crs) : null;
  if (!theirs) return { error: `Servisin koordinat sistemi (EPSG:${srid}) KentOS'un kaydında yok; servisi başka bir sistemde isteyin.` };
  if (s.srid === srid && !s.customCrs) return { same: true };
  const ours = ownSystem(s)?.system ?? null;
  if (!ours) return { error: 'Projenin koordinat sistemi yok: harita servisi kullanılamaz. Proje ayarlarında projeye bir koordinat sistemi verin.' };
  return { same: false, ours, theirs, choices: datumChoices(s) };
}

/** A point of the project's in the service's system; none where it has no place. */
export function toService(pair: ServicePair, p: Vec2): Vec2 | null {
  if (pair.same) return p;
  const t = crsTransformIn(pair.ours, pair.theirs, p, pair.choices);
  return 'point' in t ? t.point : null;
}

/** A box of the project's system in another's: its edges, five points each, moved and boxed (the desktop's `box_in`). */
export function boxIn(b: readonly [number, number, number, number], move: (p: Vec2) => Vec2 | null): [number, number, number, number] | null {
  const out: [number, number, number, number] = [Infinity, Infinity, -Infinity, -Infinity];
  for (let i = 0; i <= 4; i++) {
    const t = i / 4;
    for (const [x, y] of [
      [b[0] + (b[2] - b[0]) * t, b[1]],
      [b[0] + (b[2] - b[0]) * t, b[3]],
      [b[0], b[1] + (b[3] - b[1]) * t],
      [b[2], b[1] + (b[3] - b[1]) * t],
    ]) {
      const p = move({ x, y });
      if (!p) return null;
      out[0] = Math.min(out[0], p.x);
      out[1] = Math.min(out[1], p.y);
      out[2] = Math.max(out[2], p.x);
      out[3] = Math.max(out[3], p.y);
    }
  }
  return out.every(Number.isFinite) ? out : null;
}

/** A point of the service's system in the project's; none where it has no place. */
export function fromService(pair: ServicePair, p: Vec2): Vec2 | null {
  if (pair.same) return p;
  const t = crsTransformIn(pair.theirs, pair.ours, p, pair.choices);
  return 'point' in t ? t.point : null;
}
