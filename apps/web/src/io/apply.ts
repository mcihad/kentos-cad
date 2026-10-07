import type { BlockDefinition as ContractBlock } from '../contracts/generated/BlockDefinition';
import type { LayerField } from '../contracts/generated/LayerField';
import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import { foldTurkish } from '../core/text';
import { uuidv7 } from '../core/uuid';
import { dimensionStylesProblem, textStylesProblem, type DimensionStyleDef, type TextStyleDef } from '../model/annotationStyles';
import { importNames, type BlockDefinition } from '../model/blocks';
import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import type { LayerNode, LayerStyle } from '../model/layers';
import { readBlockDefinitions, readEntityList } from '../model/snapshot';

/**
 * Puts what a reader produced into the drawing. The objects and block
 * definitions are checked like a `.kcad` file's first, so nothing changes if
 * one is unusable; then the new layers are made, the definitions added and
 * every object goes in, all as ONE undo step with one change event
 * (CLAUDE.md §4.8, §7): undo takes the objects, the blocks and the layers
 * made for them. The desktop's is apps/desktop/src/exchange/apply.rs.
 */

/** Where the objects of one source layer go. */
export type LayerTarget =
  | { kind: 'existing'; id: string }
  /** A new layer; with the fields the file gives (docs/adr/0199 §6). */
  | { kind: 'new'; name: string; style: Partial<LayerStyle>; visible: boolean; locked: boolean; fields?: readonly LayerField[] };

export interface ImportPlan {
  /** The undo step's name ("Koordinat listesi: noktalar.ncn"). */
  label: string;
  /** Source layer name → target; objects of a layer missing here are left out. */
  layers: ReadonlyMap<string, LayerTarget>;
  /** A root group the new layers go into (made if missing); without it they go to the root. */
  group?: string;
}

export type Applied =
  | {
      ok: true;
      ids: number[];
      created: string[];
      /** The block definitions taken in, and the names changed on the way: as the file says, as it went in. */
      blocks: number;
      renamed: [string, string][];
      /** The text and dimension styles added to the project (docs/adr/0183 §7). */
      styles: [number, number];
    }
  | { ok: false; error: string };

/** The text and dimension styles a reader brought (docs/adr/0183 §7): their ids the reader's (`dxf-text-1` …), the objects naming them so. */
export interface Styles {
  readonly text?: readonly TextStyleDef[];
  readonly dimension?: readonly DimensionStyleDef[];
}

/**
 * The styles ready to go in (docs/adr/0183 §7; the desktop's `ImportedStyles`): each the project's style of the same
 * name (case aside) when it has one, the file's values staying the objects' own; else a new style under a new id. One
 * the project may not keep (a name of Standart's, values out of the rules) is none: its objects keep their look
 * without a style.
 */
export interface ImportedStyles {
  /** The reader's id → the project's (null: no style). */
  readonly ids: ReadonlyMap<string, string | null>;
  readonly text: TextStyleDef[];
  readonly dimension: DimensionStyleDef[];
}

const sameName = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();

export function importedStyles(doc: CadDocument, styles: Styles = {}): ImportedStyles {
  const ids = new Map<string, string | null>();
  const text: TextStyleDef[] = [];
  const dimension: DimensionStyleDef[] = [];
  const project = doc.settings;
  for (const s of styles.text ?? []) {
    const same = project.textStyles.value.find((p) => sameName(p.name, s.name));
    if (same) ids.set(s.id, same.id);
    else {
      const fresh = { ...s, id: uuidv7() };
      const ok = textStylesProblem([...project.textStyles.value, ...text, fresh]) === null;
      if (ok) text.push(fresh);
      ids.set(s.id, ok ? fresh.id : null);
    }
  }
  for (const s of styles.dimension ?? []) {
    const same = project.dimensionStyles.value.find((p) => sameName(p.name, s.name));
    if (same) ids.set(s.id, same.id);
    else {
      const fresh = { ...s, id: uuidv7() };
      const ok = dimensionStylesProblem([...project.dimensionStyles.value, ...dimension, fresh]) === null;
      if (ok) dimension.push(fresh);
      ids.set(s.id, ok ? fresh.id : null);
    }
  }
  return { ids, text, dimension };
}

/** An object naming the project's style, or none (`e` as it is when it names none). */
export function restyled<E extends { kind: string }>(e: E, styles: ImportedStyles): E {
  const field = e.kind === 'text' ? 'textStyle' : e.kind === 'dimension' ? 'dimStyle' : null;
  const id = field ? (e as Record<string, unknown>)[field] : undefined;
  if (!field || typeof id !== 'string') return e;
  const { [field]: _read, ...rest } = e as Record<string, unknown>;
  const to = styles.ids.get(id) ?? null;
  return (to === null ? rest : { ...rest, [field]: to }) as unknown as E;
}

/** The new styles into the project's tables: a setting, not an undo step (docs/adr/0183 §1); undoing the import leaves them, unused. */
export function addStyles(doc: CadDocument, styles: ImportedStyles): [number, number] {
  const added: [number, number] = [styles.text.length, styles.dimension.length];
  if (added[0] || added[1])
    doc.settings.assign({
      textStyles: [...doc.settings.textStyles.value, ...styles.text],
      dimensionStyles: [...doc.settings.dimensionStyles.value, ...styles.dimension],
    });
  return added;
}

/**
 * The blocks an import brings (docs/adr/0144 §5), ready to go in: each a
 * new id and a name the drawing does not have yet (`importNames`), the
 * inserts among their objects pointing at the new ids, and each object on
 * the drawing layer its source layer goes to, else on none of its own (`''`,
 * the block's: Patlat puts it on the insert's layer).
 */
export interface ImportedBlocks {
  readonly defs: readonly BlockDefinition[];
  /** The reader's block id → the drawing's. */
  readonly ids: ReadonlyMap<string, string>;
  readonly renamed: [string, string][];
}

/** The message for block definitions a reader produced that the drawing refuses. */
const unusableBlocks = (why: string) => `Dosyadan okunan bloklar çizime uymuyor (${why}). Hiçbir şey eklenmedi; dosyayla birlikte bildirin.`;

/** Checks a reader's block definitions and readies them for the drawing (nothing changes); the reason in words when one is unusable. */
export function importedBlocks(
  doc: CadDocument,
  blocks: readonly ContractBlock[] = [],
  targets: ReadonlyMap<string, string> = new Map(),
  styles: ImportedStyles = { ids: new Map(), text: [], dimension: [] },
): ImportedBlocks | { error: string } {
  if (!blocks.length) return { defs: [], ids: new Map(), renamed: [] };
  const read = readBlockDefinitions(blocks);
  if (!read.ok) return { error: unusableBlocks(read.error) };
  const names = importNames(
    doc.blocks.value.map((b) => b.name),
    read.blocks.map((b) => b.name),
  );
  const ids = new Map(read.blocks.map((b) => [b.id, uuidv7()]));
  const renamed: [string, string][] = [];
  const defs = read.blocks.map((b, i): BlockDefinition => {
    if (names[i] !== b.name) renamed.push([b.name, names[i]]);
    return { ...b, id: ids.get(b.id)!, name: names[i], entities: b.entities.map((e) => ({ ...restyled(pointAt(e, ids), styles), layerId: targets.get(e.layerId) ?? '' })) };
  });
  return { defs, ids, renamed };
}

/** Adds the definitions (inside the import's transaction); a refusal in the import's words. */
export function addBlocks(doc: CadDocument, blocks: ImportedBlocks): void {
  for (const b of blocks.defs) {
    try {
      doc.addBlock(b);
    } catch (e) {
      throw new Error(unusableBlocks(e instanceof Error ? e.message : String(e)));
    }
  }
}

/**
 * The first object placing a block the import did not bring (a reader never
 * makes one, and the drawing could not save it), in the import's words;
 * `before` objects were checked in earlier chunks.
 */
export function strayInsert(entities: readonly { kind: string; block?: string }[], known: ReadonlySet<string>, before = 0): string | null {
  const i = entities.findIndex((e) => e.kind === 'insert' && !known.has(e.block ?? ''));
  return i < 0 ? null : unusable(`İçe aktarılan nesne ${before + i + 1} (insert) › blok: ${entities[i].block} çizimde tanımlı değil`);
}

/** An insert placing its block by the id it has in the drawing. */
export function pointAt<E extends { kind: string }>(e: E, ids: ReadonlyMap<string, string>): E {
  if (e.kind !== 'insert') return e;
  const id = ids.get((e as E & { block: string }).block);
  return id ? { ...e, block: id } : e;
}

/** The project layer with this name, ignoring case and Turkish marks (DXF layer names ignore case). */
export function layerNamed(doc: CadDocument, name: string): LayerNode | undefined {
  const key = foldTurkish(name);
  return doc.layers.leaves().find((l) => foldTurkish(l.name) === key);
}

/** An id no node of the tree has, readable from the name ("import-parsel-2"). */
function freshId(name: string, taken: Set<string>): string {
  const base = `import-${foldTurkish(name).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'katman'}`;
  let id = base;
  for (let k = 2; taken.has(id); k++) id = `${base}-${k}`;
  taken.add(id);
  return id;
}

function allNodeIds(nodes: readonly LayerNode[], out = new Set<string>()): Set<string> {
  for (const n of nodes) {
    out.add(n.id);
    allNodeIds(n.children, out);
  }
  return out;
}

/** What an import needs before anything changes: where each source layer goes, and the ids of the layers it makes. */
export interface Prepared {
  /** Source layer name → the drawing layer its objects go to. */
  readonly targets: ReadonlyMap<string, string>;
  /** Source layer name → the id of the layer made for it. */
  readonly newIds: ReadonlyMap<string, string>;
  /** Every layer id an object may name once the layers are made. */
  readonly valid: ReadonlySet<string>;
  readonly taken: Set<string>;
}

/**
 * Checks the plan against the drawing (the targets exist and are not locked,
 * nothing else is being edited) and works out where each layer's objects go;
 * nothing changes. A reason in words when the import cannot go in.
 */
export function prepareImport(doc: CadDocument, plan: ImportPlan): Prepared | { error: string } {
  // An open edit or group (a processing model still running) would take the import into its own
  // undo step, and its cancel would take the import back out with it.
  if (doc.busy) return { error: 'Bir işlem aracı ya da model hâlâ çalışıyor. Bitmesini bekleyip İçe aktar düğmesine yeniden basın.' };
  const taken = allNodeIds(doc.layers.tree);
  const newIds = new Map<string, string>();
  const targets = new Map<string, string>();
  for (const [source, t] of plan.layers) {
    if (t.kind === 'new') {
      const id = freshId(t.name, taken);
      newIds.set(source, id);
      targets.set(source, id);
    } else if (doc.layers.get(t.id)?.type !== 'layer') return { error: `Hedef katman (${t.id}) çizimde yok; pencereyi kapatıp yeniden açın.` };
    else if (doc.layers.isLocked(t.id)) return { error: `“${doc.layers.get(t.id)!.name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katman seçin.` };
    else targets.set(source, t.id);
  }
  const valid = new Set([...doc.layers.leaves().map((l) => l.id), ...newIds.values()]);
  return { targets, newIds, valid, taken };
}

/** Makes the plan's new layers (inside the caller's transaction), in the group it names; their names. */
export function makeLayers(doc: CadDocument, plan: ImportPlan, prepared: Prepared): string[] {
  const created: string[] = [];
  if (!prepared.newIds.size) return created;
  let parent: string | null = null;
  if (plan.group) {
    const group = doc.layers.tree.find((n) => n.type === 'group' && foldTurkish(n.name) === foldTurkish(plan.group!));
    parent = group?.id ?? doc.addLayer({ id: freshId(plan.group, prepared.taken), name: plan.group, type: 'group', children: [] }, null).id;
  }
  for (const [source, t] of plan.layers) {
    if (t.kind !== 'new') continue;
    doc.addLayer({ id: prepared.newIds.get(source), name: t.name, style: t.style, visible: t.visible, locked: t.locked, ...(t.fields?.length ? { fields: [...t.fields] } : {}) }, parent);
    created.push(t.name);
  }
  return created;
}

/** The message for objects a reader produced that the drawing refuses. */
export const unusable = (why: string) => `Dosyadan okunan nesneler çizime uymuyor (${why}). Hiçbir şey eklenmedi; dosyayla birlikte bildirin.`;

/** Everything an import chose into the drawing as ONE undo step; the file's text and dimension styles (docs/adr/0183 §7) join the project's tables once the objects are in. */
export function applyImport(doc: CadDocument, entities: readonly ContractEntity[], plan: ImportPlan, blocks: readonly ContractBlock[] = [], fileStyles: Styles = {}): Applied {
  const prepared = prepareImport(doc, plan);
  if ('error' in prepared) return { ok: false, error: prepared.error };
  const styles = importedStyles(doc, fileStyles);
  const imported = importedBlocks(doc, blocks, prepared.targets, styles);
  if ('error' in imported) return { ok: false, error: imported.error };
  // Checked with the final layer ids and numbered 1…n; the document gives them their own ids.
  const chosen: unknown[] = [];
  for (const e of entities) {
    const layerId = prepared.targets.get(e.layerId);
    if (layerId) chosen.push(restyled(pointAt({ ...e, layerId, id: chosen.length + 1 }, imported.ids), styles));
  }
  const checked = readEntityList(chosen, prepared.valid, 'İçe aktarılan nesne');
  if (!checked.ok) return { ok: false, error: unusable(checked.error) };
  const stray = strayInsert(checked.entities, new Set(imported.ids.values()));
  if (stray) return { ok: false, error: stray };

  let created: string[] = [];
  try {
    const added = doc.transact(plan.label, () => {
      created = makeLayers(doc, plan, prepared);
      addBlocks(doc, imported);
      return doc.addMany(checked.entities as unknown as NewEntity[], plan.label);
    });
    return { ok: true, ids: added.map((e) => e.id), created, blocks: imported.defs.length, renamed: imported.renamed, styles: addStyles(doc, styles) };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}
