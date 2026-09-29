import type { AppContext } from '../../app/context';
import type { Entity } from '../../model/entities';
import { spaceLength, summarizeElevations, takesElevation, uniformElevation, vertexElevations } from '../../product/elevationValues';
import { parseNumber } from '../../tools/coordinateInput';
import { writeElevations } from '../../tools/editCommand';
import type { PropRow } from '../widgets/PropertyGrid';

/**
 * The elevation rows of Öznitelikler (docs/adr/0142), as data: what the panel says of the vertices' elevations and
 * how a value typed into a row is written. Öznitelikler puts them among the geometry values; the desktop's panel
 * says the same words.
 *
 * - A line has `Kot (başlangıç)` and `Kot (bitiş)`; a polyline or an area has `Kot` for all its vertices, holes
 *   included: `kot yok`, the one value, or the range `1098.500–1105.250 m` with, when some vertices have none, the
 *   remark `(bazı köşeler kotsuz)` under it.
 * - Typing a number sets that end, or every vertex; emptying the row clears it: none, not 0. It writes through
 *   Kot ver's operation, one undo step named Kot ver.
 * - `3B uzunluk` (a line, a polyline) and `3B çevre` (an area) show the length in space, read-only, only when
 *   every vertex has an elevation.
 * - With several objects selected, one `Kot` row among the common values: the value when every object that takes
 *   an elevation has the same at every vertex, `kot yok` when none has any, `Çeşitli` otherwise.
 */

/** What was typed into a Kot row: a number (a trailing “m” is allowed), nothing (none), or undefined when it is neither. */
export function parseElevation(text: string): number | null | undefined {
  const t = text.trim().replace(/\s*m$/, '');
  if (!t) return null;
  return parseNumber(t) ?? undefined;
}

/** The editor of a Kot row: what is typed goes to `write` as a number, or null for none; anything else is not taken. */
function elevationEditor(write?: (z: number | null) => void): PropRow['editor'] {
  return write
    ? {
        type: 'number',
        commit: (text) => {
          const z = parseElevation(text);
          if (z !== undefined) write(z);
        },
      }
    : undefined;
}

/**
 * A Kot row for `zs`, the elevations it stands for. `write` (none when the layer is locked) takes what was typed:
 * a number, or null for none.
 */
export function elevationRow(ctx: AppContext, label: string, zs: readonly (number | null)[], write?: (z: number | null) => void): PropRow {
  const f = ctx.format;
  const editor = elevationEditor(write);
  const s = summarizeElevations(zs);
  const range = (min: number, max: number) => `${f.length(min, false)}–${f.length(max, false)} m`;
  switch (s.kind) {
    case 'none':
      return { label, value: 'kot yok', numeric: true, editor };
    case 'value':
      return { label, value: f.length(s.z, false), numeric: true, unit: 'm', editor };
    case 'range':
      return { label, value: range(s.min, s.max), numeric: true, editor };
    case 'partial': {
      const note = '(bazı köşeler kotsuz)';
      return s.min === s.max ? { label, value: f.length(s.min, false), numeric: true, unit: 'm', note, editor } : { label, value: range(s.min, s.max), numeric: true, note, editor };
    }
  }
}

/** A line's `Kot (başlangıç)` (`end` 0) or `Kot (bitiş)` (1): typing sets that end and leaves the other. */
export function lineEndRow(ctx: AppContext, e: Extract<Entity, { kind: 'line' }>, end: 0 | 1, locked: boolean): PropRow {
  const z = end === 0 ? e.za : e.zb;
  const write = locked ? undefined : (v: number | null) => void writeElevations(ctx, [e], (cur, i) => (i === end ? v : cur));
  return elevationRow(ctx, end === 0 ? 'Kot (başlangıç)' : 'Kot (bitiş)', [z ?? null], write);
}

/** A polyline's or an area's `Kot`: typing sets every vertex, holes included. */
export function pathElevationRow(ctx: AppContext, e: Extract<Entity, { kind: 'polyline' | 'polygon' }>, locked: boolean): PropRow {
  const write = locked ? undefined : (v: number | null) => void writeElevations(ctx, [e], () => v);
  return elevationRow(ctx, 'Kot', vertexElevations(e), write);
}

/** `3B uzunluk` or `3B çevre`, when every vertex has an elevation; none otherwise. */
export function spaceRow(ctx: AppContext, e: Entity): PropRow[] {
  const space = spaceLength(e);
  return space ? [{ label: space.label, value: ctx.format.length(space.value, false), numeric: true, unit: 'm' }] : [];
}

/**
 * The common `Kot` of a selection of several objects, none when no object takes an elevation. The objects that
 * do not (a circle, a text) are left out of the value and of what typing writes, as Kalınlık leaves out the
 * objects without lines. `locked`: an object is on a locked layer, so nothing is editable.
 */
export function commonElevationRow(ctx: AppContext, ents: readonly Entity[], locked: boolean): PropRow[] {
  const takers = ents.filter(takesElevation);
  if (!takers.length) return [];
  const write = locked ? undefined : (v: number | null) => void writeElevations(ctx, takers, () => v);
  // Each object's own answer, compared: a selection of tens of thousands of objects must not be flattened.
  let common: number | null | 'mixed' | undefined;
  for (const e of takers) {
    const u = uniformElevation(e);
    if (common === undefined) common = u;
    else if (u !== common) common = 'mixed';
    if (common === 'mixed') break;
  }
  // The one value, or none; anything else (different values, some vertices without one) has no value to show.
  if (common === 'mixed') return [{ label: 'Kot', value: 'Çeşitli', editor: elevationEditor(write) }];
  return [elevationRow(ctx, 'Kot', [common ?? null], write)];
}
