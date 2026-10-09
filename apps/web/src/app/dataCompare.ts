import { fixed } from '../core/displayNumber';
import { ENTITY_KIND_LABEL, type Entity } from '../model/entities';
import type { LayerTime } from '../contracts/generated/LayerTime';
import type { LayerNode } from '../model/layers';
import { shownIn, timeValues } from '../model/time';
import { dataCompare, type CompareRow, type CompareSettings, type CompareStatus } from '../model/ops/compare';
import { entitiesCreate } from '../product/entitiesCreate';
import { geometryOf } from '../product/entitiesEdit';
import { newObjectOf } from '../tools/layerMoveTool';
import { treeLocked } from '../ui/layers/treeRights';
import type { AppContext } from './context';

/**
 * Veri karşılaştır (docs/adr/0179; the desktop's `data_compare.rs`): two sides, each a layer, a group or a whole
 * drawing (the old one may come from another drawing file), paired and compared by the core (`model/ops/compare.ts`);
 * the rows' words, the report and Farkları çizime yaz, which writes the differences' copies into a “Karşılaştırma” group
 * in one undo step “Veri karşılaştır”.
 */

export const STATUS_WORDS: Record<CompareStatus, string> = {
  same: 'Aynı',
  geometry: 'Geometrisi değişti',
  attributes: 'Öznitelikleri değişti',
  both: 'Geometrisi ve öznitelikleri değişti',
  added: 'Eklenen',
  removed: 'Silinen',
  key: 'Anahtar sorunu',
};

/** A side's drawing: the open one, or another read from a file. */
export interface CompareDrawing {
  /** “Bu çizim”, or the file's name. */
  readonly name: string;
  readonly here: boolean;
  readonly tree: readonly LayerNode[];
  readonly entities: readonly Entity[];
  /** The coordinate system: the EPSG code, or a definition's JSON for a project of its own. */
  readonly system: string;
}

/**
 * A side: its drawing and its node (a layer's or a group's id; null, the whole drawing); `at`, a moment its temporal
 * layers are seen at (docs/adr/0210 §8): their objects shown then, the other layers' all.
 */
export interface CompareSide {
  readonly drawing: CompareDrawing;
  readonly node: string | null;
  readonly at?: number;
}

/** The comparison's rows with the two sides' objects they index. */
export interface CompareResult {
  readonly rows: readonly CompareRow[];
  readonly old: readonly Entity[];
  readonly next: readonly Entity[];
  readonly oldSide: CompareSide;
  readonly newSide: CompareSide;
  readonly key: string | null;
}

/** The open drawing as a side's drawing. */
export function thisDrawing(ctx: AppContext): CompareDrawing {
  const s = ctx.doc.settings;
  return {
    name: 'Bu çizim',
    here: true,
    tree: ctx.doc.layers.tree,
    entities: [...ctx.doc.all()],
    system: s.customCrs.value ? JSON.stringify(s.customCrs.value) : String(s.crs.value.srid),
  };
}

function* walk(nodes: readonly LayerNode[], above: readonly string[] = []): Generator<[LayerNode, string]> {
  for (const n of nodes) {
    const path = [...above, n.name];
    yield [n, path.join(' / ')];
    yield* walk(n.children, path);
  }
}

/** A side's choices: the whole drawing, then every group and layer by its path, in the tree's order. */
export function nodeChoices(tree: readonly LayerNode[]): { value: string; label: string }[] {
  return [{ value: '', label: 'Bütün çizim' }, ...[...walk(tree)].map(([n, path]) => ({ value: n.id, label: path }))];
}

/** The layers under a node (itself when a layer); every layer for the whole drawing. */
function layersOf(tree: readonly LayerNode[], node: string | null): Set<string> {
  const out = new Set<string>();
  const take = (n: LayerNode): void => {
    if (n.type === 'layer') out.add(n.id);
    n.children.forEach(take);
  };
  if (node === null) tree.forEach(take);
  else for (const [n] of walk(tree)) if (n.id === node) take(n);
  return out;
}

/** A side's objects, in the drawing's order; at a moment, its temporal layers' objects shown then. */
export function sideObjects(side: CompareSide): Entity[] {
  const layers = layersOf(side.drawing.tree, side.node);
  const all = side.drawing.entities.filter((e) => layers.has(e.layerId));
  if (side.at === undefined) return all;
  const rules = new Map<string, LayerTime>();
  for (const [n] of walk(side.drawing.tree)) if (n.type === 'layer' && n.time && layers.has(n.id)) rules.set(n.id, n.time);
  if (!rules.size) return all;
  const byLayer = new Map<string, Entity[]>();
  for (const e of all) if (rules.has(e.layerId)) byLayer.set(e.layerId, [...(byLayer.get(e.layerId) ?? []), e]);
  const keep = new Set<Entity>();
  for (const [id, list] of byLayer) {
    const rule = rules.get(id)!;
    const shown = shownIn(rule.end != null, !!rule.cumulative, timeValues(rule, list.map((e) => e.attrs)), { kind: 'instant', a: side.at });
    list.forEach((e, i) => shown[i] && keep.add(e));
  }
  return all.filter((e) => !rules.has(e.layerId) || keep.has(e));
}

/** The temporal layers under a side's node, by id. */
export function temporalLayersOf(tree: readonly LayerNode[], node: string | null): LayerNode[] {
  const layers = layersOf(tree, node);
  return [...walk(tree)].map(([n]) => n).filter((n) => n.type === 'layer' && n.time && layers.has(n.id));
}

/** The attribute fields of the sides' objects, each once, in the order of their characters' codes (as the core's rows). */
export function fieldsOf(...sides: CompareSide[]): string[] {
  const seen = new Set<string>();
  for (const s of sides) for (const e of sideObjects(s)) for (const k of Object.keys(e.attrs)) seen.add(k);
  return [...seen].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
}

/** A layer's path in a side's drawing. */
function pathOf(tree: readonly LayerNode[], id: string): string {
  for (const [n, path] of walk(tree)) if (n.id === id) return path;
  return id;
}

/** Why the two sides cannot be compared, or null. */
export function sidesRefused(old: CompareSide, next: CompareSide): string | null {
  // The same layers at two different dates are two states of them (docs/adr/0210 §8).
  const dated = old.at !== undefined && next.at !== undefined && old.at !== next.at;
  if (old.drawing.here && next.drawing.here && !dated) {
    const a = layersOf(old.drawing.tree, old.node);
    const b = layersOf(next.drawing.tree, next.node);
    if ([...a].some((id) => b.has(id))) return 'Eski ve Yeni aynı katmanları içeriyor; ayrı katmanlar ya da gruplar seçin ya da iki tarafa farklı tarihler yazın.';
  }
  if (old.drawing.system !== next.drawing.system)
    return `“${old.drawing.name}” başka bir koordinat sisteminde; dönüştürme yapılmaz. Bu çizimle aynı koordinat sistemindeki bir çizim seçin.`;
  return null;
}

/** Compares the sides; the result, or why not (said by the caller). */
export function runCompare(old: CompareSide, next: CompareSide, settings: CompareSettings): CompareResult | string {
  const refused = sidesRefused(old, next);
  if (refused) return refused;
  if (settings.match === 'key' && !settings.key) return 'Anahtar alanı seçin: eşleşen nesnelerde aynı değeri taşıyan öznitelik.';
  if (!(settings.search >= 0) || !(settings.tolerance >= 0)) return 'Arama uzaklığı ve tolerans sıfır ya da artı birer sayı olmalı (metre).';
  const a = sideObjects(old);
  const b = sideObjects(next);
  const member = (e: Entity) => ({ shape: geometryOf(e as never), attrs: { ...e.attrs } });
  const rows = dataCompare(a.map(member), b.map(member), settings);
  return { rows, old: a, next: b, oldSide: old, newSide: next, key: settings.match === 'key' ? (settings.key ?? null) : null };
}

/** Each finding's count. */
export function statusCounts(rows: readonly CompareRow[]): Record<CompareStatus, number> {
  const out: Record<CompareStatus, number> = { same: 0, geometry: 0, attributes: 0, both: 0, added: 0, removed: 0, key: 0 };
  for (const r of rows) out[r.status]++;
  return out;
}

/** “1 eklenen, 1 silinen, 4 değişen, 2 aynı, 1 anahtar sorunu”: the counts that are not naught. */
export function countsText(rows: readonly CompareRow[]): string {
  const c = statusCounts(rows);
  const words: [number, string][] = [
    [c.added, 'eklenen'],
    [c.removed, 'silinen'],
    [c.geometry + c.attributes + c.both, 'değişen'],
    [c.same, 'aynı'],
    [c.key, 'anahtar sorunu'],
  ];
  const said = words.filter(([n]) => n > 0).map(([n, w]) => `${n} ${w}`);
  return said.length ? said.join(', ') : 'karşılaştırılacak nesne yok';
}

/** A row's words as the table and the report give them: Durum, Tür, Anahtar, Eski katman, Yeni katman, Konum farkı, Değişen alanlar. */
export function rowWords(r: CompareResult, row: CompareRow, metres: (m: number) => string): string[] {
  const a = row.old === undefined ? undefined : r.old[row.old];
  const b = row.new === undefined ? undefined : r.next[row.new];
  const kind = b ?? a;
  return [
    STATUS_WORDS[row.status],
    kind ? ENTITY_KIND_LABEL[kind.kind] : '',
    r.key ? ((b ?? a)?.attrs[r.key] ?? '') : '',
    a ? pathOf(r.oldSide.drawing.tree, a.layerId) : '',
    b ? pathOf(r.newSide.drawing.tree, b.layerId) : '',
    row.distance === undefined ? '' : metres(row.distance),
    row.fields.join(', '),
  ];
}

export const REPORT_HEADER = ['Durum', 'Tür', 'Anahtar', 'Eski katman', 'Yeni katman', 'Konum farkı (m)', 'Değişen alanlar'] as const;

/** The report's rows: the header, then every row (the decimal comma, as the CSV file takes it). */
export function reportRows(r: CompareResult, onlyDiffs: boolean): string[][] {
  return [
    [...REPORT_HEADER],
    ...r.rows.filter((row) => !onlyDiffs || row.status !== 'same').map((row) => rowWords(r, row, (m) => fixed(m, 3).replace('.', ','))),
  ];
}

/**
 * Farkları çizime yaz: a “Karşılaştırma” group (a name of its own) with a layer for each kind of difference there is —
 * Eklenen (green), Silinen (red), Değişen (yellow) — holding the copies of the new side's added and changed objects and
 * the old side's removed ones, with their own properties; one undo step “Veri karşılaştır”. An insert of a block this
 * drawing lacks (another drawing's) is left out and said. Whether it wrote.
 */
export function writeDifferences(ctx: AppContext, r: CompareResult): boolean {
  const { doc, log } = ctx;
  const locked = treeLocked(ctx);
  if (locked) return (log.warn(locked), false);
  const pick = (rows: readonly CompareRow[], side: 'old' | 'new') =>
    rows.flatMap((row) => {
      const i = side === 'old' ? row.old : row.new;
      return i === undefined ? [] : [(side === 'old' ? r.old : r.next)[i]];
    });
  const sets: [string, string, Entity[]][] = [
    ['Eklenen', '#5FBF77', pick(r.rows.filter((row) => row.status === 'added'), 'new')],
    ['Silinen', '#E5484D', pick(r.rows.filter((row) => row.status === 'removed'), 'old')],
    ['Değişen', '#F2C94C', pick(r.rows.filter((row) => row.status === 'geometry' || row.status === 'attributes' || row.status === 'both'), 'new')],
  ];
  let skipped = 0;
  const kept = sets.map(([name, color, list]) => {
    const fit = list.filter((e) => e.kind !== 'insert' || !!doc.block(e.block));
    skipped += list.length - fit.length;
    return [name, color, fit] as const;
  });
  if (!kept.some(([, , list]) => list.length)) return (log.info('Yazılacak fark yok: bütün nesneler aynı.'), false);
  const group = doc.layers.uniqueName('Karşılaştırma');
  const warnings: string[] = [];
  try {
    doc.transact('Veri karşılaştır', () => {
      const g = doc.addLayer({ name: group, type: 'group', children: [] }, null);
      for (const [name, color, list] of kept) {
        if (!list.length) continue;
        const layer = doc.addLayer({ name, type: 'layer', style: { color } }, g.id);
        const out = entitiesCreate.execute({ doc }, { layerId: layer.id, objects: list.map(newObjectOf) });
        if (out.status !== 'completed') throw new Error('error' in out ? out.error.message : 'Farklar yazılamadı.');
        warnings.push(...out.warnings.map((w) => w.message));
      }
    });
  } catch (e) {
    return (log.warn(e instanceof Error ? e.message : String(e)), false);
  }
  for (const w of warnings) log.warn(w);
  const said = kept.filter(([, , list]) => list.length).map(([name, , list]) => `${list.length} ${name.toLocaleLowerCase('tr-TR')}`);
  log.success(`Farklar çizime yazıldı: “${group}” grubunda ${said.join(', ')} nesne.`);
  if (skipped) log.warn(`${skipped} blok yerleştirmesi yazılmadı: bloğu bu çizimde yok.`);
  return true;
}
