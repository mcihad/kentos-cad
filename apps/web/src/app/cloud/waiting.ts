import type { FeatureRecord } from '../../contracts/generated/FeatureRecord';
import type { SyncCore } from './syncCore';

/**
 * Another editor's objects on a layer this drawing lacks (it was removed
 * here and the removal did not go through, or the tree that brings it has
 * not come yet). Put now, they would sit on no layer; refused, they would be
 * gone until the project is opened again. So they wait, by layer, with no
 * warning, and are fetched at their current versions as soon as a tree
 * brings their layer (live, “take the server's”, or a layer given back).
 * The list lives only while the project is open: opened again, the project
 * comes whole from the server, every object on its layer.
 */

/** Whether the drawing has this layer (a layer, not a group). */
const hasLayer = (core: SyncCore, id: string): boolean => core.o.doc.layers.get(id)?.type === 'layer';

/** Takes an object off the waiting list (deleted, taken back into a conflict, or put). */
export function forget(core: SyncCore, id: string): void {
  for (const [layerId, set] of core.waiting) if (set.delete(id) && !set.size) core.waiting.delete(layerId);
}

/**
 * Takes the records on a layer this drawing lacks out of `records` and puts
 * them on the waiting list. Returns them, by id. The others leave the list:
 * their layer is here, so they are put now (moved there by another editor
 * while waiting) or become a conflict.
 */
export function setAside(core: SyncCore, records: Map<string, FeatureRecord>): Map<string, FeatureRecord> {
  const aside = new Map<string, FeatureRecord>();
  for (const [id, record] of records) {
    const layerId = (record.entity as { layerId?: unknown }).layerId;
    if (typeof layerId !== 'string' || hasLayer(core, layerId)) {
      forget(core, id);
      continue;
    }
    records.delete(id);
    forget(core, id);
    let set = core.waiting.get(layerId);
    if (!set) core.waiting.set(layerId, (set = new Set()));
    set.add(id);
    aside.set(id, record);
  }
  return aside;
}

/** The waiting objects whose layer the drawing has now: off the list, fetched and put. */
export async function fetchArrived(core: SyncCore): Promise<void> {
  const ids: string[] = [];
  for (const [layerId, set] of core.waiting) {
    if (!hasLayer(core, layerId)) continue;
    ids.push(...set);
    core.waiting.delete(layerId);
  }
  if (ids.length) await core.takeServerCopies(ids);
}

/** What the user hears for objects still waiting when the project is left: one line per layer, named as the server's tree last named it. */
export function waitingTexts(core: SyncCore): string[] {
  const names = new Map<string, string>();
  const walk = (nodes: readonly { id?: string; name?: string; children?: unknown[] }[]) => {
    for (const n of nodes) {
      if (n.id && n.name) names.set(n.id, n.name);
      walk((n.children ?? []) as typeof nodes);
    }
  };
  try {
    walk(JSON.parse(core.metaBase.layers) as { id?: string; name?: string; children?: unknown[] }[]);
  } catch {
    // Unreadable: the ids are said instead.
  }
  return [...core.waiting].map(
    ([layerId, set]) => `“${names.get(layerId) ?? layerId}” katmanı bu çizimde olmadığı için başka birinin ${set.size} nesnesi burada gösterilmedi; proje yeniden açılınca görünür.`,
  );
}
