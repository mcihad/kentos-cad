import type { AppContext } from '../app/context';
import { ENTITY_KIND_LABEL, type Entity, type EntityKind } from '../model/entities';
import type { Vec2 } from '../model/geometry';

/**
 * Seçim süzgeci (docs/adr/0187 §5): while it is on, only the kinds it holds are selected, by a click and its hover, a
 * window or crossing box, the selection tools, Tümünü seç, Ters çevir and a command's step that selects objects. What it
 * leaves out is said. The desktop's twin is `kentos_interaction::selectable`.
 */

/** The kinds the filter can hold, in its menu's order (`ENTITY_KIND_LABEL`'s). */
export const FILTER_KINDS: readonly EntityKind[] = [
  'point',
  'line',
  'polyline',
  'polygon',
  'circle',
  'arc',
  'ellipse',
  'spline',
  'xline',
  'ray',
  'text',
  'dimension',
  'hatch',
  'insert',
  'leader',
  'table',
];

/** Whether an object of this kind may be selected now. */
export function kindSelectable(ctx: AppContext, kind: EntityKind): boolean {
  const s = ctx.settings;
  return !s.selectFilter.value || s.selectKinds.value.has(kind);
}

/** Says how many objects the filter left out of a selection; nothing when none. */
export function sayFiltered(ctx: AppContext, n: number): void {
  if (n > 0) ctx.log.info(`Seçim süzgeci ${n} nesneyi dışarıda bıraktı.`);
}

/** The ids the filter lets through, in their order; what it leaves out is said. */
export function selectableIds(ctx: AppContext, ids: readonly number[]): number[] {
  if (!ctx.settings.selectFilter.value) return [...ids];
  const out = ids.filter((id) => {
    const e = ctx.doc.get(id);
    return !!e && kindSelectable(ctx, e.kind);
  });
  sayFiltered(ctx, ids.length - out.length);
  return out;
}

/**
 * What a click at `screen` can select, the most specific first (Sıradakini seç's candidates, docs/adr/0187 §1), the
 * filter's leftovers dropped; when it drops every one, said (unless `quiet`: the pointer only rests there).
 */
export function candidatesAt(ctx: AppContext, screen: Vec2, quiet = false): Entity[] {
  const all = ctx.view.pickAll(screen);
  if (!ctx.settings.selectFilter.value) return all;
  const out = all.filter((e) => kindSelectable(ctx, e.kind));
  if (!out.length && all.length && !quiet) ctx.log.info(`Seçim süzgeci tıklanan nesneyi (${ENTITY_KIND_LABEL[all[0].kind]}) dışarıda bıraktı.`);
  return out;
}

/** The object a click at `screen` selects (the pointer's hover: `quiet`): the first candidate, or the plain pick when no filter is on. */
export function pickSelectable(ctx: AppContext, screen: Vec2, quiet = false): Entity | null {
  if (!ctx.settings.selectFilter.value) return ctx.view.pick(screen);
  return candidatesAt(ctx, screen, quiet)[0] ?? null;
}
