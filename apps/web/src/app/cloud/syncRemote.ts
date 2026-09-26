import type { EventRecord } from '../../contracts/generated/EventRecord';
import type { FeatureRecord } from '../../contracts/generated/FeatureRecord';
import type { Entity } from '../../model/entities';
import { BATCH, type SyncConflict, type SyncCore } from './syncCore';
import { entityJson, metaParts } from './tracker';

/**
 * Other editors' commits arriving as events (CLAUDE.md §17, §21.1). Our own
 * commits are skipped by request id. Each object is fetched once at its
 * latest version and put into the drawing without an undo step, under the
 * server's id as its persistent id (an object already here keeps its slot);
 * an object with unsent local edits is not overwritten but becomes a
 * conflict. New metadata comes first, since new objects may sit on a layer
 * it brings. Returns the conflicts found; the cursor moves past the last event.
 */
export async function applyEvents(core: SyncCore, events: readonly EventRecord[]): Promise<SyncConflict[]> {
  if (!events.length) return [];
  const { o, tracker } = core;
  const last = events[events.length - 1].seq;
  const ops = new Map<string, 'create' | 'update' | 'delete'>();
  let meta = false;
  for (const e of events) {
    if (e.requestId && core.own.has(e.requestId)) continue;
    for (const f of e.features) ops.set(f.id, f.op);
    meta ||= e.meta;
  }
  const wanted = [...ops].filter(([, op]) => op !== 'delete').map(([id]) => id);
  const fetched = new Map<string, FeatureRecord>();
  for (let i = 0; i < wanted.length; i += BATCH) {
    const page = await o.api.featuresById(o.tenantId, o.projectId, wanted.slice(i, i + BATCH));
    for (const f of page.features) fetched.set(f.id, f);
  }
  if (core.closed) return [];
  const conflicts: SyncConflict[] = [];
  if (meta) {
    const m = await core.serverMeta();
    if (core.closed) return [];
    if (m && core.sendsMeta()) conflicts.push({ featureId: '@project', reason: 'project', server: null, actual: m.version });
    else if (m) {
      core.metaVersion = m.version;
      await core.whenIdle();
      if (core.closed) return [];
      o.doc.applyExternal({ meta: m.meta });
      core.metaBase = metaParts(o.doc);
    }
  }
  const good = core.checked([...fetched.values()].map((f) => ({ key: f.id, entity: f.entity })));
  // Decided once no edit is open, and applied at once: no edit slips in between.
  await core.whenIdle();
  if (core.closed) return [];
  const doc = o.doc;
  const put: Entity[] = [];
  const remove: number[] = [];
  for (const [featureId] of ops) {
    const slot = doc.slotOf(featureId);
    const record = fetched.get(featureId) ?? null;
    if (core.busyLocally(featureId)) {
      conflicts.push({ featureId, reason: record ? 'remote' : 'deleted', server: record, actual: record?.version ?? null });
      continue;
    }
    if (record) {
      const incoming = good.get(featureId);
      if (!incoming) continue;
      put.push({ ...incoming, id: slot ?? doc.allocateId(), uid: featureId } as Entity);
      tracker.set(featureId, { version: record.version, json: entityJson(incoming) });
    } else {
      if (slot !== undefined) remove.push(slot);
      tracker.set(featureId, null);
    }
  }
  if (put.length || remove.length) doc.applyExternal({ put, remove });
  core.cursor = last;
  return conflicts;
}
