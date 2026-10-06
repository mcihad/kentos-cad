import type { AppContext } from '../app/context';
import type { Formatter } from '../app/format';
import type { Entity } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import { kmText, kmValue, pathReading, pathStation } from './constructions';

/**
 * Obje üzerinde nokta and Km ve sapma (docs/adr/0188 §1–§2): the object the calculator walks, picked on the drawing,
 * and what is typed against it. The arithmetic is the shared core's (`tools::point_calc`). The desktop's twin is
 * `kentos_interaction::point_calc::route`.
 */

/** Why a click gave no object to walk. */
export const NO_OBJECT_HERE = 'Tıklanan yerde nesne yok; çizginin, yayın, dairenin, elipsin, eğrinin ya da alanın üzerine tıklayın.';
/** Why the object clicked cannot be walked. */
export const NO_ROUTE = 'Bu nesnenin üzerinde yürünecek tek bir yolu yok: çizgi, çoklu çizgi, yay, daire, elips, eğri ya da tek parçalı alan seçin.';

/** The object walked: its route, which end it is walked from and where that is. */
export interface Route {
  readonly entity: Entity;
  readonly fromEnd: boolean;
  readonly start: Vec2;
  readonly length: number;
}

/**
 * The object under the click that has a route, the most specific first; Obje üzerinde nokta walks it from the end
 * nearer the click (`nearerEnd`), Km from its first vertex. Why none is said.
 */
export function pickRoute(ctx: AppContext, screen: Vec2, at: Vec2, nearerEnd: boolean): Route | null {
  const hits = ctx.view.pickAll(screen);
  if (!hits.length) {
    ctx.log.warn(NO_OBJECT_HERE);
    return null;
  }
  for (const entity of hits) {
    const first = pathStation(entity, false, 0, 0);
    const start = first.point ?? null;
    if (!start) continue;
    const end = pathStation(entity, true, 0, 0).point ?? start;
    // A closed path starts at its first vertex; an open one at the end nearer the click.
    const closed = dist(start, end) < 1e-12;
    const fromEnd = nearerEnd && !closed && dist(at, end) < dist(at, start);
    return { entity, fromEnd, start: fromEnd ? end : start, length: first.length };
  }
  ctx.log.warn(NO_ROUTE);
  return null;
}

/**
 * The point `s` along the route and `offset` square to it (metres); null outside it. The length as written is the
 * end: typing what the length is said to be reaches it.
 */
export function routeAt(route: Route, s: number, offset: number, f: Formatter): { point: Vec2 | null; length: number } {
  const t = s > route.length && f.length(s) === f.length(route.length) ? route.length : s;
  const at = pathStation(route.entity, route.fromEnd, t, offset);
  return { point: at.point ?? null, length: at.length };
}

/** The distance along the route of the point nearest `p` and how far `p` is from it, the right positive. */
export function routeRead(route: Route, p: Vec2): { s: number; offset: number } | null {
  return pathReading(route.entity, route.fromEnd, p);
}

/** `-?\d+(\.\d+)?` alone (the calculator's numbers). */
export function calcNumber(t: string): number | null {
  return /^-?\d+(?:\.\d+)?$/.test(t.trim()) ? +t.trim() : null;
}

/** A km and an offset typed for Km ve sapma: `km`, or `km,sapma` (a `;` or spaces part them too). */
export function kmAndOffset(t: string): { km: number; offset: number } | null {
  const text = t.trim();
  const m = text.match(/^([^,;\s]+)\s*(?:[,;]\s*|\s+)(\S+)$/);
  const km = kmValue(m ? m[1] : text);
  if (km === null) return null;
  if (!m) return { km, offset: 0 };
  const offset = calcNumber(m[2]);
  return offset === null ? null : { km, offset };
}

/** A km as the calculator writes it: the project's length decimals. */
export const km = (value: number, f: Formatter): string => kmText(value, f.lengthDecimals);
