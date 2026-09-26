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
 * (CLAUDE.md §21.3). Each change names its object by persistent id, the
 * server's id (docs/adr/0026): an object the drawing has is changed in its
 * slot, another comes in under that id. A command that was on its way is
 * sent first, with its own idempotency key: if it had been committed the
 * server answers from its log and those changes are the server's now. A
 * remaining change whose base is still the server's version waits to be
 * sent; one the server moved past is a conflict (the drawing shows the
 * local copy until resolved); a deletion the server has too is done. Still
 * no answer from the server: the whole draft is kept for the next attempt.
 * An object edited since the project reopened keeps that newer edit. A
 * change the drawing cannot take (its layer is gone, say) stays in the
 * device draft, unsent, and the user is told (`SyncCore.held`).
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
  // Which changes still wait, and which moved on the server meanwhile.
  const waiting: [string, Draft['changes'][string]][] = [];
  const moved: string[] = [];
  for (const [id, change] of Object.entries(draft.changes)) {
    if (carried.has(id) && carried.get(id) === (change.entity ? JSON.stringify(change.entity) : null)) continue;
    // An edit made since the project reopened is newer than the draft: it wins.
    if (core.dirty.has(id)) continue;
    const serverVersion = tracker.get(id)?.version ?? null;
    // Deleted here and on the server alike: nothing is left to do.
    if (!change.entity && serverVersion === null && doc.slotOf(id) === undefined) continue;
    waiting.push([id, change]);
    if (change.base !== serverVersion) moved.push(id);
  }
  const conflicts: SyncConflict[] = [];
  if (moved.length) {
    const fresh = await o.api.featuresById(o.tenantId, o.projectId, moved);
    if (core.closed) return { waiting: false, conflicts: [], changed: false };
    const byId = new Map(fresh.features.map((f) => [f.id, f]));
    for (const id of moved) {
      const rec = byId.get(id) ?? null;
      conflicts.push({ featureId: id, reason: rec ? 'changed' : 'deleted', server: rec, actual: rec?.version ?? null });
    }
  }
  let meta: ExternalMeta | undefined;
  if (draft.meta && core.canEditMeta) {
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
      if (draft.meta.base !== core.metaVersion) conflicts.push({ featureId: '@project', reason: 'project', server: null, actual: core.metaVersion });
    } else o.warn(`Cihazdaki proje bilgisi taslağı okunamadı: ${read.error}`);
  }
  // The draft is local work: it goes in without history, then counts as unsent.
  await core.whenIdle();
  if (core.closed) return { waiting: false, conflicts: [], changed: false };
  if (meta) doc.applyExternal({ meta });
  // Checked against the layers the drawing has now (the draft's own tree included).
  const good = core.checked(
    waiting.flatMap(([id, c]) => (c.entity ? [{ key: id, entity: c.entity }] : [])),
    (id, error) => {
      core.held.set(id, draft.changes[id]);
      o.warn(`Cihazdaki taslakta bir nesne çizime konamadı (${id}): ${error}. Değişiklik bu cihazda saklanıyor, gönderilmedi.`);
    },
  );
  const put: Entity[] = [];
  const remove: number[] = [];
  const touched: string[] = [];
  for (const [id, change] of waiting) {
    const slot = doc.slotOf(id);
    if (change.entity) {
      const e = good.get(id);
      if (!e) continue;
      put.push({ ...e, id: slot ?? doc.allocateId(), uid: id } as Entity);
    } else if (slot !== undefined) remove.push(slot);
    touched.push(id);
  }
  doc.applyExternal({ put, remove });
  for (const id of touched) core.dirty.add(id);
  if (meta) core.metaDirty = true;
  // A change kept aside is not in the drawing: there is nothing to choose between for it yet.
  return { waiting: false, conflicts: conflicts.filter((c) => !core.held.has(c.featureId)), changed: touched.length > 0 || !!meta };
}
