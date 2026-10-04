import type { EntityEdit } from '../../contracts/generated/EntityEdit';
import type { EntityGeometry as EditGeometry } from '../../contracts/generated/EntityGeometry';
import type { CadDocument } from '../../model/document';
import type { Entity, LineEntity, PolylineEntity } from '../../model/entities';
import type { Elevated } from '../../model/ops/elevation';
import { vertexTableInsert, vertexTableMove, vertexTableRadius, vertexTableRemove, vertexTableRows, vertexTableZ, type VertexAnswer, type VertexKind, type VertexRefusal, type VertexRow } from '../../model/ops/vertexTable';
import { UNIT_PER_METRE } from '../../model/projectSettings';
import { elevatedPaths, withoutElevations } from '../../product/elevation';
import { entitiesEdit } from '../../product/entitiesEdit';
import { parseNumber } from '../../tools/coordinateInput';
import { neighbourLines, type Neighbours } from '../../tools/neighbours';
import { inStep } from './pointEdit';

/**
 * Köşe tablosu's writes (docs/adr/0172 §4, §5), apart from the DOM: a vertex's cell given a value, Satır ekle's draft
 * written after a vertex, and vertices removed. The paths come from the core (`ops::vertex_table`); every write goes
 * through `cad.entities.edit`. A cell is one undo step, “Köşe düzenle”; a draft “Köşe ekle”, a removal “Köşe sil”.
 * fixtures/vertex-table/v1/edits.json holds both platforms to the same drawing, messages and steps; the desktop's is
 * `apps/desktop/src/vertices/edit.rs`.
 */

/** The cells that are edited, in the order Tab takes them. */
export type VertexColumn = 'east' | 'north' | 'z' | 'radius';
export const VERTEX_COLUMNS: readonly VertexColumn[] = ['east', 'north', 'z', 'radius'];

/** The table's own messages start so; the command's refusals are said as they are. */
export const PREFIX = 'Köşe tablosu: ';
export const STEP = 'Köşe düzenle';
export const ADD_STEP = 'Köşe ekle';
export const REMOVE_STEP = 'Köşe sil';

/** What the table edits: one line, polyline or area. */
export type Editable = LineEntity | PolylineEntity;
export const isEditable = (e: Entity | undefined): e is Editable => !!e && (e.kind === 'line' || e.kind === 'polyline' || e.kind === 'polygon');

/** A vertex: its path (`elevatedPaths`' order) and its place in it. */
export interface At {
  path: number;
  index: number;
}

/**
 * What came of a write: what to warn of, the undo step written (null: none), whether the cell stays open, and what to
 * tell (the neighbours that changed with it).
 */
export interface Outcome {
  said: string[];
  step: string | null;
  stay: boolean;
  told?: string[];
}

/**
 * Topological editing while the mode is on (docs/adr/0160, 0172 §6): the neighbours the object becoming `after`
 * (from `before`) puts right, or null. The table gives `tools/neighbours.ts`'s; the cases run without it.
 */
export type NeighboursOf = (before: Entity, after: Entity) => Neighbours | null;

/** A draft written: where the next one goes (after the new vertex), null when not written. */
export interface DraftOutcome extends Outcome {
  next: At | null;
}

/** Satır ekle's row: its cells as typed. */
export interface VertexDraft {
  east: string;
  north: string;
  z: string;
}

export const emptyVertexDraft = (): VertexDraft => ({ east: '', north: '', z: '' });

/** How a length is shown (the project's format: `ctx.format.length`); the least radius is said with it. */
export type LengthText = (metres: number) => string;

const nothing: Outcome = { said: [], step: null, stay: false };

/** What a column is called in a message: east and north as the project's type names them (docs/adr/0165 §4). */
function word(doc: CadDocument, col: VertexColumn): string {
  if (col === 'z') return 'Z';
  if (col === 'radius') return 'Yarıçap';
  const cad = doc.settings.workspace.value === 'cad';
  return col === 'east' ? (cad ? 'X' : 'Y') : cad ? 'Y' : 'X';
}

/** The table's words for a refusal (`adding`: a new vertex's). */
export function refusalText(r: VertexRefusal, length: LengthText, adding = false): string {
  switch (r.why) {
    case 'ontoNeighbour':
      return PREFIX + (adding ? 'Yeni köşe komşu köşesinin yerinde olamaz.' : 'Köşe komşu köşesinin yerine taşınamaz; köşeyi kaldırmak için satırı silin.');
    case 'noEdge':
      return `${PREFIX}Bu köşeden çıkan kenar yok.`;
    case 'lineArc':
      return `${PREFIX}Çizginin kenarı yay olamaz; önce köşe ekleyin.`;
    case 'noChord':
      return `${PREFIX}Kenarın iki ucu aynı yerde; yay olamaz.`;
    case 'radiusBelow':
      return `${PREFIX}Yarıçap kirişin yarısından küçük olamaz: en az ${length(r.least)}.`;
    case 'lineEnds':
      return `${PREFIX}Çizginin iki ucu silinemez.`;
    case 'pathMin':
      return `${PREFIX}Çoklu çizgide en az iki köşe kalmalı.`;
    case 'ringMin':
      return `${PREFIX}Her halkada en az üç köşe kalmalı.`;
    case 'missing':
      return `${PREFIX}Köşe bulunamadı.`;
  }
}

/** The object's paths and rows. */
export function vertexRows(e: Editable): VertexRow[] {
  return vertexTableRows(elevatedPaths(e));
}

/** The text a cell's editor opens with: the value whole (the shortest that reads back), in a local project's unit. */
export function cellText(row: VertexRow, col: VertexColumn, perMetre = 1): string {
  // In a local project's unit (docs/adr/0165 §2), without the multiplication's last-digit noise.
  const shown = (v: number) => String(perMetre === 1 ? v : Number((v * perMetre).toPrecision(15)));
  switch (col) {
    case 'east':
      return shown(row.p.x);
    case 'north':
      return shown(row.p.y);
    case 'z':
      return row.z == null ? '' : shown(row.z);
    case 'radius':
      return row.radius == null ? '' : shown(row.radius);
  }
}

/** Whether the row's cell is edited: a line's edge takes no arc, and no edge leaves an open path's last vertex. */
export function cellEditable(kind: VertexKind, row: VertexRow, col: VertexColumn): boolean {
  return col !== 'radius' || (kind !== 'line' && row.chord != null);
}

/**
 * The object's geometry with `paths` put back (in `elevatedPaths`' order), as `kind`: a line given a vertex is a
 * polyline; an area keeps its holes and parts, each ring its own bulges and elevations.
 */
export function geometryOf(e: Editable, kind: VertexKind, paths: readonly Elevated[]): EditGeometry {
  const ring = (p: Elevated) => ({ pts: p.pts, ...(p.bulges && { bulges: p.bulges }), zs: p.zs });
  if (kind === 'line') return { kind: 'line', a: paths[0].pts[0], b: paths[0].pts[1], zs: paths[0].zs };
  if (kind === 'polyline') return { kind: 'polyline', ...ring(paths[0]) };
  let k = 1;
  const next = () => ring(paths[k++]);
  const area = e as PolylineEntity;
  const holes = (area.holes ?? []).map(() => next());
  const parts = (area.parts ?? []).map((part) => {
    const own = next();
    const inner = (part.holes ?? []).map(() => next());
    return { ...own, ...(inner.length && { holes: inner }) };
  });
  return { kind: 'polygon', ...ring(paths[0]), ...(holes.length && { holes }), ...(parts.length && { parts }) };
}

/** Typed coordinates, elevations and radii in metres: typed in a local project's unit (docs/adr/0165 §2). */
const metresOf = (doc: CadDocument) => {
  const perMetre = UNIT_PER_METRE[doc.settings.unit];
  return (typed: number) => typed / perMetre;
};

/**
 * The slack of a radius: half a unit of what the table shows of a length (the project's decimals, in its unit), in
 * metres; a radius that short of half the chord is half a circle (docs/adr/0172 §4).
 */
export function radiusSlack(doc: CadDocument): number {
  // 10⁻ᵈ read from its decimal text: the nearest double, as the desktop's.
  return (0.5 * Number(`1e-${doc.settings.lengthDecimals.value}`)) / UNIT_PER_METRE[doc.settings.unit];
}

/** The object as `geometry` makes it, its data kept, for the neighbours' rule (which reads only its shape). */
function entityAfter(e: Editable, geometry: EditGeometry): Entity {
  const { pts: _pts, bulges: _bulges, holes: _holes, parts: _parts, zs: _zs, ...rest } = e as unknown as Record<string, unknown>;
  const { a: _a, b: _b, za: _za, zb: _zb, ...data } = rest;
  return { ...data, ...withoutElevations(geometry) } as unknown as Entity;
}

/**
 * The core's answer written as one undo step named `step`, the object updated in place (a line that became a polyline
 * replaced, keeping its slot, id and data), with the neighbours `neighbours` puts right; a refusal said in the table's
 * words keeps the cell open, the command's refusal closes it.
 */
function written(
  doc: CadDocument,
  e: Editable,
  answer: VertexAnswer,
  step: string,
  operation: 'properties' | 'vertexAdd' | 'vertexRemove',
  length: LengthText,
  neighbours: NeighboursOf | undefined,
  adding = false,
): Outcome {
  if ('refusal' in answer) return { said: [refusalText(answer.refusal, length, adding)], step: null, stay: true };
  const { kind, paths } = answer.edited;
  const uid = doc.uidOf(e.id) ?? '';
  const geometry = geometryOf(e, kind, paths);
  const change: EntityEdit = kind === e.kind ? { kind: 'update', uid, geometry } : { kind: 'replace', uid, geometry, keepData: true };
  // With the mode on, the neighbours' shared corners and edges go with it (docs/adr/0172 §6).
  const follow = neighbours?.(e, entityAfter(e, geometry)) ?? null;
  const refused = inStep(doc, step, () => {
    const r = entitiesEdit.execute({ doc }, { operation, changes: [change, ...(follow?.changes ?? [])] });
    return r.status === 'completed' || !('error' in r) ? null : r.error.message;
  });
  if (refused) return { said: [refused], step: null, stay: false };
  const lines = follow ? neighbourLines(follow) : [];
  return { said: lines.filter((l) => l.level === 'warn').map((l) => l.text), step, stay: false, told: lines.filter((l) => l.level === 'info').map((l) => l.text) };
}

/**
 * A vertex's cell given `text` (docs/adr/0172 §4): Y and X move it (its arcs keep their bulges), Z is its elevation
 * (empty removes it), Yarıçap its edge's radius (empty or 0: straight). Numbers read as the point input reads them; a
 * value that is no number, or that the core refuses, keeps the cell open; an unchanged one writes nothing.
 */
export function writeVertexCell(doc: CadDocument, e: Editable, at: At, col: VertexColumn, text: string, length: LengthText, neighbours?: NeighboursOf): Outcome {
  const paths = elevatedPaths(e);
  const row = vertexTableRows(paths).find((r) => r.path === at.path && r.index === at.index);
  if (!row) return { said: [`${PREFIX}Köşe bulunamadı.`], step: null, stay: false };
  const blank = text.trim() === '';
  const empty = blank && (col === 'z' || col === 'radius');
  const typed = empty ? null : parseNumber(text);
  if (typed === null && !empty) return { said: [`${PREFIX}${word(doc, col)} bir sayı olmalı.`], step: null, stay: true };
  const v = typed === null ? null : metresOf(doc)(typed);
  const kind = e.kind as VertexKind;
  if (col === 'east' || col === 'north') {
    const to = col === 'east' ? { x: v as number, y: row.p.y } : { x: row.p.x, y: v as number };
    if (to.x === row.p.x && to.y === row.p.y) return nothing;
    return written(doc, e, vertexTableMove(kind, paths, at.path, at.index, to), STEP, 'properties', length, neighbours);
  }
  if (col === 'z') {
    if (v === (row.z ?? null)) return nothing;
    return written(doc, e, vertexTableZ(kind, paths, at.path, at.index, v), STEP, 'properties', length, neighbours);
  }
  // A radius within 1e-9 of the edge's is the edge's: the cell opens with it, its last bit is the platform's.
  const radius = row.radius ?? null;
  const same = v !== null && radius !== null && Math.abs(v - radius) <= 1e-9 * Math.abs(radius);
  if (((v === null || v === 0) && radius === null) || same) return nothing;
  return written(doc, e, vertexTableRadius(kind, paths, at.path, at.index, v === 0 ? null : v, radiusSlack(doc)), STEP, 'properties', length, neighbours);
}

/**
 * Satır ekle's row written after vertex `after` (docs/adr/0172 §5): Y and X are needed, Z is optional; through
 * `cad.entities.edit` as Köşe ekle writes (“Köşe ekle”). The next draft goes after the new vertex.
 */
export function writeVertexDraft(doc: CadDocument, e: Editable, after: At, d: VertexDraft, length: LengthText, neighbours?: NeighboursOf): DraftOutcome {
  const fail = (said: string): DraftOutcome => ({ said: [said], step: null, stay: true, next: null });
  const [eastWord, northWord] = [word(doc, 'east'), word(doc, 'north')];
  const east = d.east.trim();
  const north = d.north.trim();
  if (!east && !north) return fail(`${PREFIX}${eastWord} ve ${northWord} yazılmalı.`);
  if (!east) return fail(`${PREFIX}${eastWord} yazılmalı.`);
  if (!north) return fail(`${PREFIX}${northWord} yazılmalı.`);
  const x = parseNumber(east);
  if (x === null) return fail(`${PREFIX}${eastWord} bir sayı olmalı.`);
  const y = parseNumber(north);
  if (y === null) return fail(`${PREFIX}${northWord} bir sayı olmalı.`);
  let z: number | null = null;
  if (d.z.trim()) {
    const typedZ = parseNumber(d.z);
    if (typedZ === null) return fail(`${PREFIX}Z bir sayı olmalı.`);
    z = typedZ;
  }
  const metres = metresOf(doc);
  const answer = vertexTableInsert(e.kind as VertexKind, elevatedPaths(e), after.path, after.index, { x: metres(x), y: metres(y) }, z === null ? null : metres(z));
  const out = written(doc, e, answer, ADD_STEP, 'vertexAdd', length, neighbours, true);
  return { ...out, next: out.step ? { path: after.path, index: after.index + 1 } : null };
}

/** The vertices `at` removed in one write (docs/adr/0172 §5), as Köşe sil writes (“Köşe sil”). */
export function removeVertices(doc: CadDocument, e: Editable, at: readonly At[], length: LengthText, neighbours?: NeighboursOf): Outcome {
  const answer = vertexTableRemove(e.kind as VertexKind, elevatedPaths(e), at.map((a) => [a.path, a.index] as const));
  const out = written(doc, e, answer, REMOVE_STEP, 'vertexRemove', length, neighbours);
  // Nothing to keep open: a removal has no cell.
  return { ...out, stay: false };
}
