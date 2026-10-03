import type { EntityGeometry as EditGeometry } from '../../contracts/generated/EntityGeometry';
import type { CadDocument } from '../../model/document';
import type { Entity, PointEntity } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';
import type { Elevated } from '../../model/ops/elevation';
import { followPoint } from '../../model/ops/pointEditor';
import { textIncrement } from '../../model/textEdit';
import { UNIT_PER_METRE } from '../../model/projectSettings';
import { elevatedPaths } from '../../product/elevation';
import { entitiesEdit } from '../../product/entitiesEdit';
import { entitiesSet } from '../../product/entitiesSet';
import { pointCreate } from '../../product/pointCreate';
import { parseNumber } from '../../tools/coordinateInput';

/**
 * Nokta editörü's writes (docs/adr/0153 §3, §4), apart from the DOM: a cell given a value, with the line work that
 * follows the point, and Satır ekle's draft. Every write goes through the product commands; a cell is one undo step,
 * “Nokta düzenle”. fixtures/point-editor/v1/edits.json holds both platforms to the same drawing, messages and steps;
 * the desktop's is `apps/desktop/src/points/edit.rs`.
 */

/** The cells that are edited, by the core's column keys. */
export type EditColumn = 'name' | 'east' | 'north' | 'z' | 'code';

/** The editor's own messages start so; the commands' refusals are said as they are. */
export const PREFIX = 'Nokta editörü: ';
export const STEP = 'Nokta düzenle';
/** What a cell's value is called in a message: east and north as the project's type names them (docs/adr/0165 §4). */
const word = (doc: CadDocument, col: 'east' | 'north' | 'z') => (col === 'z' ? 'Z' : doc.settings.workspace.value === 'cad' ? { east: 'X', north: 'Y' }[col] : { east: 'Y', north: 'X' }[col]);
export const FOLLOW_LOCKED = `${PREFIX}Bağlı çizgilerden biri kilitli katmanda; katmanın kilidini açın ya da Bağlı çizgiler izler'i kapatın.`;

/** What came of a write: what to say (warnings), the undo step written (null: none), and whether the cell stays open. */
export interface Outcome {
  said: string[];
  step: string | null;
  stay: boolean;
}

const nothing: Outcome = { said: [], step: null, stay: false };

/** Typed coordinates and elevations in metres: typed in a local project's unit (docs/adr/0165 §2). */
const metresOf = (doc: CadDocument) => {
  const perMetre = UNIT_PER_METRE[doc.settings.unit];
  return (typed: number) => typed / perMetre;
};

/** The text a cell's editor opens with: the name and the code as written, a number whole (the shortest that reads back). */
export function cellText(e: PointEntity, col: EditColumn, perMetre = 1): string {
  // In a local project's unit (docs/adr/0165 §2), without the multiplication's last-digit noise.
  const shown = (v: number) => String(perMetre === 1 ? v : Number((v * perMetre).toPrecision(15)));
  switch (col) {
    case 'name':
      return e.label ?? '';
    case 'code':
      return e.attrs.Kod ?? '';
    case 'east':
      return shown(e.p.x);
    case 'north':
      return shown(e.p.y);
    case 'z':
      return e.z === undefined ? '' : shown(e.z);
  }
}

/** Whether another point has the name (trimmed). */
export function named(doc: CadDocument, name: string, except: number | null): boolean {
  for (const e of doc.all()) if (e.kind === 'point' && e.id !== except && e.label?.trim() === name) return true;
  return false;
}

const sameName = (name: string) => `${PREFIX}“${name}” adında başka bir nokta da var.`;

/** Line work's geometry with its paths (in `elevatedPaths`' order: the outer ring, its holes, then each part's ring and holes) put back. */
export function withPaths(e: Entity, ps: readonly Elevated[]): EditGeometry | null {
  if (e.kind === 'line') return { kind: 'line', a: ps[0].pts[0], b: ps[0].pts[1], zs: ps[0].zs };
  if (e.kind === 'polyline') return { kind: 'polyline', pts: ps[0].pts, ...(e.bulges && { bulges: e.bulges }), zs: ps[0].zs };
  if (e.kind !== 'polygon') return null;
  let k = 1;
  const ring = (r: { bulges?: number[] }) => {
    const p = ps[k++];
    return { pts: p.pts, ...(r.bulges && { bulges: r.bulges }), zs: p.zs };
  };
  const holes = (e.holes ?? []).map((h) => ring(h));
  const parts = (e.parts ?? []).map((part) => {
    const own = ring(part);
    return { ...own, ...(part.holes && { holes: part.holes.map((h) => ring(h)) }) };
  });
  return { kind: 'polygon', pts: ps[0].pts, ...(e.bulges && { bulges: e.bulges }), zs: ps[0].zs, ...(holes.length && { holes }), ...(parts.length && { parts }) };
}

/** Runs `write` as one undo step named `label`; a refusal it returns is rolled back and returned. */
export function inStep(doc: CadDocument, label: string, write: () => string | null): string | null {
  let refused: string | null = null;
  try {
    doc.transact(label, () => {
      refused = write();
      if (refused) throw new Error(refused);
    });
  } catch {
    // Rolled back; the refusal is returned.
  }
  return refused;
}

/**
 * A point's cell given `text` (docs/adr/0153 §3). Ad and Kod are trimmed, empty removes them; Y, X and Z read as the
 * point input reads a number, an empty Z removes the elevation. With `follow`, the line work with a vertex at the
 * point's place (1 µm) follows it: moved with it, or given its new elevation. A value that is no number keeps the cell
 * open; an unchanged one writes nothing.
 */
export function writeCell(doc: CadDocument, e: PointEntity, col: EditColumn, text: string, follow: boolean): Outcome {
  const metres = metresOf(doc);
  const uid = doc.uidOf(e.id) ?? '';
  if (col === 'name' || col === 'code') {
    const v = text.trim() || null;
    const old = col === 'name' ? e.label?.trim() || null : (e.attrs.Kod ?? null);
    if (v === old) return nothing;
    const refused = inStep(doc, STEP, () => {
      const r = entitiesSet.execute({ doc }, col === 'name' ? { uids: [uid], label: v, operation: 'label' } : { uids: [uid], attrs: { Kod: v }, operation: 'attributes' });
      return r.status !== 'completed' && 'error' in r ? r.error.message : null;
    });
    if (refused) return { said: [refused], step: null, stay: false };
    return { said: col === 'name' && v !== null && named(doc, v, e.id) ? [sameName(v)] : [], step: STEP, stay: false };
  }
  const blank = text.trim() === '';
  const typed = col === 'z' && blank ? null : parseNumber(text);
  if (typed === null && !(col === 'z' && blank)) return { said: [`${PREFIX}${word(doc, col)} bir sayı olmalı.`], step: null, stay: true };
  const v = typed === null ? null : metres(typed);
  const from = e.p;
  const to: Vec2 = col === 'east' ? { x: v as number, y: from.y } : col === 'north' ? { x: from.x, y: v as number } : from;
  const unchanged = col === 'z' ? (v ?? undefined) === e.z : to.x === from.x && to.y === from.y;
  if (unchanged) return nothing;
  const z = col === 'z' ? v : (e.z ?? null);
  const changes = [{ kind: 'update' as const, uid, geometry: { kind: 'point', p: to, ...(z !== null && { z }) } as EditGeometry }];
  if (follow) {
    for (const other of doc.all()) {
      if (other.kind !== 'line' && other.kind !== 'polyline' && other.kind !== 'polygon') continue;
      const moved = followPoint(elevatedPaths(other), from, to, col === 'z', col === 'z' ? v : null);
      const geometry = moved && withPaths(other, moved);
      if (geometry) changes.push({ kind: 'update', uid: doc.uidOf(other.id) ?? '', geometry });
    }
  }
  const refused = inStep(doc, STEP, () => {
    const r = entitiesEdit.execute({ doc }, { operation: col === 'z' ? 'elevation' : 'properties', changes });
    if (r.status === 'completed' || !('error' in r)) return null;
    // The point is the first change: a refusal of another is a line work's lock.
    return r.error.code === 'layer_locked' && !(r.error.path ?? '').startsWith('changes[0]') ? FOLLOW_LOCKED : r.error.message;
  });
  return refused ? { said: [refused], step: null, stay: false } : { said: [], step: STEP, stay: false };
}

/** Satır ekle's row: its cells as typed. */
export interface Draft {
  name: string;
  east: string;
  north: string;
  z: string;
  code: string;
}

export const emptyDraft = (name = ''): Draft => ({ name, east: '', north: '', z: '', code: '' });

/** A draft written, and the next draft's name (Artır's; empty when the name does not end with a number); null when not written. */
export interface DraftOutcome extends Outcome {
  next: string | null;
}

/**
 * Satır ekle's row written (docs/adr/0153 §4): Y and X are needed, Z is optional; through `cad.point.create` on
 * `layerId`, in `color` (none: the layer's). The step is the command's, “Ekle”.
 */
export function writeDraft(doc: CadDocument, d: Draft, layerId: string, color: string | null): DraftOutcome {
  const metres = metresOf(doc);
  const fail = (said: string): DraftOutcome => ({ said: [said], step: null, stay: true, next: null });
  const east = d.east.trim();
  const north = d.north.trim();
  if (!east && !north) return fail(`${PREFIX}Y ve X yazılmalı.`);
  if (!east) return fail(`${PREFIX}Y yazılmalı.`);
  if (!north) return fail(`${PREFIX}X yazılmalı.`);
  const typedX = parseNumber(east);
  if (typedX === null) return fail(`${PREFIX}Y bir sayı olmalı.`);
  const typedY = parseNumber(north);
  if (typedY === null) return fail(`${PREFIX}X bir sayı olmalı.`);
  const [x, y] = [metres(typedX), metres(typedY)];
  let z: number | null = null;
  if (d.z.trim()) {
    const typedZ = parseNumber(d.z);
    if (typedZ === null) return fail(`${PREFIX}Z bir sayı olmalı.`);
    z = metres(typedZ);
  }
  const name = d.name.trim();
  const code = d.code.trim();
  const said = name && named(doc, name, null) ? [sameName(name)] : [];
  const r = pointCreate.execute(
    { doc },
    { layerId, p: { x, y }, ...(z !== null && { z }), ...(name && { label: name }), ...(code && { attrs: { Kod: code } }), ...(color !== null && { color }) },
  );
  if (r.status !== 'completed') return 'error' in r ? fail(r.error.message) : fail(`${PREFIX}nokta yazılamadı.`);
  return { said, step: 'Ekle', stay: false, next: name ? (textIncrement(name) ?? '') : '' };
}

/** The cells the editor walks, in the order Tab takes them. */
export const EDIT_COLUMNS: readonly EditColumn[] = ['name', 'east', 'north', 'z', 'code'];

/**
 * Where the editor goes after a cell of the row `id` (docs/adr/0153 §3), in `ids`, the rows' order before the write:
 * Enter down the column, Tab right (Kod to the next row's Ad), Shift+Tab left (Ad to the row above's Kod); null past
 * the ends.
 */
export function nextCell(ids: readonly number[], id: number, col: EditColumn, how: 'down' | 'right' | 'left'): { id: number; col: EditColumn } | null {
  const at = ids.indexOf(id);
  if (at < 0) return null;
  if (how === 'down') return at + 1 < ids.length ? { id: ids[at + 1], col } : null;
  const c = EDIT_COLUMNS.indexOf(col) + (how === 'right' ? 1 : -1);
  if (c >= 0 && c < EDIT_COLUMNS.length) return { id, col: EDIT_COLUMNS[c] };
  const row = at + (how === 'right' ? 1 : -1);
  if (row < 0 || row >= ids.length) return null;
  return { id: ids[row], col: EDIT_COLUMNS[how === 'right' ? 0 : EDIT_COLUMNS.length - 1] };
}
