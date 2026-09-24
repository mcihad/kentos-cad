import type { ProjectPatch } from '../../contracts/generated/ProjectPatch';
import type { ExternalMeta } from '../../model/document';
import type { Entity } from '../../model/entities';
import { ApiFailure } from './api';
import type { Draft } from './drafts';
import { readIncoming } from './incoming';
import type { SyncConflict, SyncCore } from './syncCore';
import { metaParts } from './tracker';

export type Restored = { waiting: true } | { waiting: false; conflicts: SyncConflict[]; changed: boolean };

/**
 * Puts a draft saved on this device back into a freshly opened project
 * (CLAUDE.md §21.3). A command that was on its way is sent first, with its
 * own idempotency key: if it had been committed the server answers from its
 * log and those changes are the server's now. A remaining change whose base
 * is still the server's version waits to be sent; one the server moved past
 * is a conflict (the drawing shows the local copy until resolved). Still no
 * answer from the server: the whole draft is kept for the next attempt. An
 * object edited since the project reopened keeps that newer edit.
 */
export async function restoreDraft(core: SyncCore, draft: Draft): Promise<Restored> {
  const { o, tracker } = core;
  const doc = o.doc;
  // Changes the lost command carried; once it is answered they are the server's, not a draft's.
  const carried = new Map<string, string | null>();
  if (draft.inflight) {
    const sent = (draft.inflight.input as { features?: { op: string; id: string; entity?: unknown }[] }).features ?? [];
    try {
      for (const f of sent) carried.set(f.id, f.op === 'delete' ? null : JSON.stringify(f.entity));
      await o.api.command(draft.inflight);
      core.own.add(draft.inflight.requestId);
      await core.takeServerCopies(sent.map((f) => f.id));
      if (core.closed) return { waiting: false, conflicts: [], changed: false };
    } catch (e) {
      // Left meanwhile: the draft stays as it is for the next time the project opens.
      if (core.closed) return { waiting: false, conflicts: [], changed: false };
      if (!(e instanceof ApiFailure) || e.transient) {
        await o.drafts.put(o.draftKey, draft);
        return { waiting: true };
      }
      // Refused for good (a conflict or a rule): its changes stay in the draft and are checked below.
      carried.clear();
    }
  }
  const put: Entity[] = [];
  const remove: number[] = [];
  const conflicts: SyncConflict[] = [];
  const touched: number[] = [];
  const moved: string[] = [];
  for (const [featureId, change] of Object.entries(draft.changes)) {
    if (carried.has(featureId) && carried.get(featureId) === (change.entity ? JSON.stringify(change.entity) : null)) continue;
    const local = tracker.localOf(featureId);
    // An edit made since the project reopened is newer than the draft: it wins.
    if (local !== undefined && core.dirty.has(local)) continue;
    const serverVersion = local === undefined ? null : (tracker.get(local)?.version ?? null);
    const lid = local ?? doc.allocateId();
    if (local === undefined) tracker.set(lid, { featureId, version: null, json: null });
    if (change.entity) put.push({ ...(change.entity as Entity), id: lid });
    else if (local !== undefined) remove.push(lid);
    touched.push(lid);
    if (change.base !== serverVersion) moved.push(featureId);
  }
  if (moved.length) {
    const fresh = await o.api.featuresById(o.tenantId, o.projectId, moved);
    if (core.closed) return { waiting: false, conflicts: [], changed: false };
    const byId = new Map(fresh.features.map((f) => [f.id, f]));
    for (const id of moved) {
      const rec = byId.get(id) ?? null;
      conflicts.push({ featureId: id, localId: tracker.localOf(id) ?? null, reason: rec ? 'changed' : 'deleted', server: rec, actual: rec?.version ?? null });
    }
  }
  let meta: ExternalMeta | undefined;
  if (draft.meta && o.canEditMeta) {
    const p: ProjectPatch = draft.meta.patch;
    const read = readIncoming(
      {
        name: p.name ?? doc.name.value,
        settings: p.settings ?? doc.settings.toJSON(),
        origin: doc.origin,
        layers: p.layers ?? JSON.parse(metaParts(doc).layers),
        activeLayer: p.activeLayer ?? doc.layers.active.value,
        styles: p.styles ?? doc.styles.value,
      },
      [],
    );
    if (read.ok) {
      const c = read.content;
      meta = { name: c.name, settings: c.settings, layers: c.layers, activeLayer: c.activeLayer, styles: c.styles };
      if (draft.meta.base !== core.metaVersion) conflicts.push({ featureId: '@project', localId: null, reason: 'project', server: null, actual: core.metaVersion });
    } else o.warn(`Cihazdaki proje bilgisi taslağı okunamadı: ${read.error}`);
  }
  // The draft is local work: it goes in without history, then counts as unsent.
  if (meta) doc.applyExternal({ meta });
  const good = core.checked(put.map((e) => ({ key: String(e.id), entity: e })));
  const checkedPut = put.filter((e) => good.has(String(e.id))).map((e) => ({ ...good.get(String(e.id))!, id: e.id }) as Entity);
  doc.applyExternal({ put: checkedPut, remove });
  for (const id of touched) core.dirty.add(id);
  if (meta) core.metaDirty = true;
  return { waiting: false, conflicts, changed: touched.length > 0 || !!meta };
}
