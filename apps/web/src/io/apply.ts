import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import { foldTurkish } from '../core/text';
import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import type { LayerNode, LayerStyle } from '../model/layers';
import { readEntityList } from '../model/snapshot';

/**
 * Puts what a reader produced into the drawing. The objects are checked
 * like a `.kcad` file's first, so nothing changes if one is unusable; then
 * the new layers are made and every object goes in, all as ONE undo step
 * with one change event (CLAUDE.md §4.8, §7): undo takes the objects and
 * the layers made for them.
 */

/** Where the objects of one source layer go. */
export type LayerTarget =
  | { kind: 'existing'; id: string }
  | { kind: 'new'; name: string; style: Partial<LayerStyle>; visible: boolean; locked: boolean };

export interface ImportPlan {
  /** The undo step's name ("Koordinat listesi: noktalar.ncn"). */
  label: string;
  /** Source layer name → target; objects of a layer missing here are left out. */
  layers: ReadonlyMap<string, LayerTarget>;
  /** A root group the new layers go into (made if missing); without it they go to the root. */
  group?: string;
}

export type Applied = { ok: true; ids: number[]; created: string[] } | { ok: false; error: string };

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
    doc.addLayer({ id: prepared.newIds.get(source), name: t.name, style: t.style, visible: t.visible, locked: t.locked }, parent);
    created.push(t.name);
  }
  return created;
}

/** The message for objects a reader produced that the drawing refuses. */
export const unusable = (why: string) => `Dosyadan okunan nesneler çizime uymuyor (${why}). Hiçbir şey eklenmedi; dosyayla birlikte bildirin.`;

export function applyImport(doc: CadDocument, entities: readonly ContractEntity[], plan: ImportPlan): Applied {
  const prepared = prepareImport(doc, plan);
  if ('error' in prepared) return { ok: false, error: prepared.error };
  // Checked with the final layer ids and numbered 1…n; the document gives them their own ids.
  const chosen: unknown[] = [];
  for (const e of entities) {
    const layerId = prepared.targets.get(e.layerId);
    if (layerId) chosen.push({ ...e, layerId, id: chosen.length + 1 });
  }
  const checked = readEntityList(chosen, prepared.valid, 'İçe aktarılan nesne');
  if (!checked.ok) return { ok: false, error: unusable(checked.error) };

  let created: string[] = [];
  const added = doc.transact(plan.label, () => {
    created = makeLayers(doc, plan, prepared);
    return doc.addMany(checked.entities as unknown as NewEntity[], plan.label);
  });
  return { ok: true, ids: added.map((e) => e.id), created };
}
