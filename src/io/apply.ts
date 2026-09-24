import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import { foldTurkish } from '../core/text';
import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import type { LayerNode, LayerStyle } from '../model/layers';
import { readEntityList } from '../model/snapshot';

/**
 * Puts what a reader produced into the drawing. The objects are checked
 * like a `.kcad` file's first, so nothing changes if one is unusable; then
 * the new layers are made (layer creation is not undoable, as with the
 * processing runner's new target layers) and every object goes in as ONE
 * undo step with one change event (CLAUDE.md §4.8, §7).
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

export function applyImport(doc: CadDocument, entities: readonly ContractEntity[], plan: ImportPlan): Applied {
  const taken = allNodeIds(doc.layers.tree);
  const newIds = new Map<string, string>();
  for (const [source, t] of plan.layers) {
    if (t.kind === 'new') newIds.set(source, freshId(t.name, taken));
    else if (doc.layers.get(t.id)?.type !== 'layer') return { ok: false, error: `Hedef katman (${t.id}) çizimde yok; pencereyi kapatıp yeniden açın.` };
    else if (doc.layers.isLocked(t.id)) return { ok: false, error: `“${doc.layers.get(t.id)!.name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katman seçin.` };
  }
  const target = (source: string) => {
    const t = plan.layers.get(source);
    return !t ? null : t.kind === 'existing' ? t.id : newIds.get(source)!;
  };
  // Checked with the final layer ids and numbered 1…n; the document gives them their own ids.
  const chosen: unknown[] = [];
  for (const e of entities) {
    const layerId = target(e.layerId);
    if (layerId) chosen.push({ ...e, layerId, id: chosen.length + 1 });
  }
  const valid = new Set([...doc.layers.leaves().map((l) => l.id), ...newIds.values()]);
  const checked = readEntityList(chosen, valid, 'İçe aktarılan nesne');
  if (!checked.ok) return { ok: false, error: `Dosyadan okunan nesneler çizime uymuyor (${checked.error}). Hiçbir şey eklenmedi; dosyayla birlikte bildirin.` };

  const created: string[] = [];
  if (newIds.size) {
    let parent: string | null = null;
    if (plan.group) {
      const group = doc.layers.tree.find((n) => n.type === 'group' && foldTurkish(n.name) === foldTurkish(plan.group!));
      parent = group?.id ?? doc.layers.add({ id: freshId(plan.group, taken), name: plan.group, type: 'group', children: [] }, null).id;
    }
    for (const [source, t] of plan.layers) {
      if (t.kind !== 'new') continue;
      doc.layers.add({ id: newIds.get(source), name: t.name, style: t.style, visible: t.visible, locked: t.locked }, parent);
      created.push(t.name);
    }
  }
  const added = doc.transact(plan.label, () => doc.addMany(checked.entities as unknown as NewEntity[], plan.label));
  return { ok: true, ids: added.map((e) => e.id), created };
}
