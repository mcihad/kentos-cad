import { Signal } from '../../core/signal';
import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { EventRecord } from '../../contracts/generated/EventRecord';
import type { FeatureConflict } from '../../contracts/generated/FeatureConflict';
import type { ExternalMeta } from '../../model/document';
import type { Entity } from '../../model/entities';
import { ApiFailure } from './api';
import type { Draft } from './drafts';
import { BATCH, PROJECT_DELETED, SyncCore, uuid, type Inflight, type SaveState, type SyncConflict, type SyncOptions } from './syncCore';
import { applyEvents } from './syncRemote';
import { restoreDraft } from './syncRestore';
import { changeOf, entityJson, metaParts, metaPatch, type Planned } from './tracker';

export type { SaveState, SyncConflict, SyncOptions } from './syncCore';

/**
 * Autosave of an open cloud project (CLAUDE.md §21.3, §15 "Edit commit
 * protokolü"). Edits are diffed against what the server acknowledged
 * (tracker.ts), kept on this device as a draft while they wait, and sent a
 * second after the last edit (at most five seconds after the first) as one
 * `project.changes` command at a time. "Kaydedildi" is shown only after the
 * server's answer, with nothing left to send.
 *
 * - A lost answer or a dead network: the same command goes again with the
 *   same idempotency key (the server answers from its log, never twice).
 * - A 409: nothing more is sent until the user chooses the server's copy or
 *   theirs; the local drawing is kept either way until then.
 * - Other editors' changes (syncRemote.ts): applied at once, unless the
 *   object has unsent local changes, which makes it a conflict too.
 * - A device draft from an earlier session (syncRestore.ts) goes back in
 *   when the project is opened again.
 * - The project deleted on the server (its event, or a 410): nothing more is
 *   sent, and edits keep going to the device draft (a restored project
 *   brings them back when opened).
 */

const DRAFT_MS = 300;
const RESTORE_RETRY_MS = 5000;

export class ProjectSync {
  readonly state = new Signal<SaveState>('saved');
  /** Objects (and the metadata) waiting to be sent. */
  readonly pending = new Signal(0);
  readonly conflicts = new Signal<readonly SyncConflict[]>([]);
  readonly lastSaved = new Signal<number | null>(null);
  readonly error = new Signal('');
  private readonly core: SyncCore;
  private readonly o: SyncOptions;
  private flushing: Promise<boolean> | null = null;
  private again = false;
  private firstPending = 0;
  private timer = 0;
  private draftTimer = 0;
  private restoreTimer = 0;
  private retries = 0;
  private remoteQueue: Promise<void> = Promise.resolve();
  private readonly unsubscribe: (() => void)[] = [];
  private noticeShown = false;
  private disposed = false;
  private deleted = false;

  constructor(o: SyncOptions) {
    this.o = o;
    this.core = new SyncCore(o);
    const doc = o.doc;
    this.unsubscribe.push(
      doc.events.on('touched', (e) => {
        if (e.external) return;
        for (const id of e.ids) this.core.dirty.add(id);
        if (e.layerStyles) this.core.metaDirty = true;
        this.changed();
      }),
      doc.name.subscribe(() => this.metaChanged()),
      doc.settings.changed.subscribe(() => this.metaChanged()),
      doc.styles.subscribe(() => this.metaChanged()),
      doc.layers.events.on('structure', () => this.metaChanged()),
      doc.layers.events.on('state', () => this.metaChanged()),
    );
  }

  /** The newest event cursor applied (the socket subscribes after it). */
  get cursor(): string {
    return this.core.cursor;
  }

  /**
   * Stops for good (the project is left). Whatever is still on its way is
   * abandoned, not finished: the drawing may already hold another project,
   * and the device draft (saved before every send) keeps the command with
   * its idempotency key for the next time this project opens.
   */
  dispose(): void {
    this.disposed = true;
    this.core.closed = true;
    clearTimeout(this.timer);
    clearTimeout(this.draftTimer);
    clearTimeout(this.restoreTimer);
    for (const u of this.unsubscribe) u();
  }

  /** The server id of a local object (for tests and the conflict list). */
  featureOf(localId: number): string | undefined {
    return this.core.tracker.get(localId)?.featureId;
  }

  private metaChanged(): void {
    // The quiet external meta changes also arrive here; they leave the drawing equal to the base.
    if (metaPatch(this.o.doc, this.core.metaBase) === null) return;
    this.core.metaDirty = true;
    this.changed();
  }

  private changed(): void {
    if (this.disposed) return;
    if (this.o.canWrite === false) {
      if (!this.noticeShown) {
        this.noticeShown = true;
        this.o.warn('Bu projeyi yalnız görüntüleyebilirsiniz; değişiklikleriniz buluta kaydedilmez.');
      }
      this.state.set('readonly');
      return;
    }
    this.pending.set(this.core.pendingCount());
    if (this.deleted) {
      // Nothing goes out any more; the edit is kept on this device.
      clearTimeout(this.draftTimer);
      this.draftTimer = setTimeout(() => void this.saveDraft(), DRAFT_MS) as unknown as number;
      return;
    }
    if (this.core.metaDirty && !this.o.canEditMeta && !this.noticeShown) {
      this.noticeShown = true;
      this.o.warn('Katman ve proje bilgisi değişiklikleriniz yalnız bu cihazda kalıyor: projede bunları değiştirme yetkiniz yok.');
    }
    if (!this.firstPending) this.firstPending = Date.now();
    if (this.state.value === 'saved' || this.state.value === 'error') this.state.set('pending');
    clearTimeout(this.draftTimer);
    this.draftTimer = setTimeout(() => void this.saveDraft(), DRAFT_MS) as unknown as number;
    this.schedule();
  }

  private schedule(delay?: number): void {
    clearTimeout(this.timer);
    if (this.conflicts.value.length) return;
    const debounce = this.o.debounceMs ?? 1000;
    const latest = this.firstPending + (this.o.maxDelayMs ?? 5000) - Date.now();
    this.timer = setTimeout(() => void this.flush(), delay ?? Math.max(0, Math.min(debounce, latest))) as unknown as number;
  }

  // ── Drafts ─────────────────────────────────────────────────────────────

  private draft(): Draft | null {
    const { core } = this;
    const changes: Draft['changes'] = {};
    for (const id of core.dirty) {
      const p = core.tracker.plan(this.o.doc, id);
      if (!p) continue;
      const base = p.op === 'create' ? null : p.expected;
      changes[p.featureId] = { base, entity: p.op === 'delete' ? null : structuredClone(p.entity) };
    }
    const patch = core.sendsMeta() ? metaPatch(this.o.doc, core.metaBase) : null;
    if (!Object.keys(changes).length && !patch && !core.inflight) return null;
    return {
      userId: this.o.userId,
      changes,
      meta: patch ? { base: core.metaVersion, patch } : undefined,
      inflight: core.inflight?.envelope,
      updated: Date.now(),
    };
  }

  /** Writes what is still unsent to this device now (before the project is left). A viewer's edits are never kept. */
  async keepDraft(): Promise<void> {
    clearTimeout(this.draftTimer);
    if (this.o.canWrite === false) return;
    await this.saveDraft();
  }

  private async saveDraft(): Promise<void> {
    clearTimeout(this.draftTimer);
    // After leaving, the drawing is not this project's any more: its draft stays as last saved.
    if (this.disposed) return;
    try {
      const d = this.draft();
      if (d) await this.o.drafts.put(this.o.draftKey, d);
      else await this.o.drafts.delete(this.o.draftKey);
    } catch (e) {
      // The changes are still in memory and still being sent; the user must know a crash would lose them.
      this.error.set(`Değişiklikler bu cihaza yedeklenemedi (${(e as Error).message}); sekmeyi kapatmayın.`);
    }
  }

  // ── Sending ────────────────────────────────────────────────────────────

  /** Sends everything waiting now; true when the server has it all. */
  flush(): Promise<boolean> {
    if (this.flushing) {
      this.again = true;
      return this.flushing;
    }
    this.flushing = this.run().finally(() => {
      this.flushing = null;
      if (this.again) {
        this.again = false;
        this.schedule(0);
      }
    });
    return this.flushing;
  }

  private async run(): Promise<boolean> {
    const { core } = this;
    if (this.disposed || this.deleted || this.conflicts.value.length || this.o.canWrite === false) return false;
    const doc = this.o.doc;
    if (doc.busy) {
      this.schedule(200);
      return false;
    }
    clearTimeout(this.timer);
    if (core.inflight && !(await this.send(core.inflight))) return false;
    for (;;) {
      if (this.disposed) return false;
      const planned: Planned[] = [];
      for (const id of [...core.dirty]) {
        const p = core.tracker.plan(doc, id);
        if (p) planned.push(p);
        else core.dirty.delete(id);
        if (planned.length >= BATCH) break;
      }
      const patch = core.sendsMeta() ? metaPatch(doc, core.metaBase) : null;
      if (!patch) core.metaDirty = core.metaDirty && !this.o.canEditMeta && metaPatch(doc, core.metaBase) !== null;
      if (!planned.length && !patch) break;
      const expectedVersions: Record<string, string> = {};
      for (const p of planned) if (p.op !== 'create') expectedVersions[p.featureId] = p.expected;
      if (patch) expectedVersions['@project'] = core.metaVersion;
      const envelope: CommandEnvelope = {
        commandName: 'project.changes',
        version: 1,
        tenantId: this.o.tenantId,
        projectId: this.o.projectId,
        requestId: `web-${uuid()}`,
        idempotencyKey: uuid(),
        expectedVersions,
        input: { features: planned.map(changeOf), ...(patch ? { project: patch } : {}) },
      };
      core.inflight = { envelope, planned, meta: patch ? metaParts(doc) : null, revision: doc.revision };
      // Kept on the device before it goes: after a crash the same key is sent again.
      await this.saveDraft();
      if (this.disposed || !(await this.send(core.inflight))) return false;
    }
    this.firstPending = 0;
    this.pending.set(0);
    this.state.set('saved');
    this.error.set('');
    if (!core.dirty.size && !core.sendsMeta()) doc.markSaved(doc.revision);
    await this.saveDraft();
    return true;
  }

  private async send(f: Inflight): Promise<boolean> {
    const { core } = this;
    this.state.set('saving');
    core.own.add(f.envelope.requestId);
    try {
      const result = await this.o.api.command(f.envelope);
      // Left while it was on its way: the answer belongs to a drawing that is gone.
      if (this.disposed) return false;
      for (const p of f.planned) {
        core.tracker.acknowledge(p, result.versions[p.featureId]);
        if (core.tracker.settled(this.o.doc, p)) core.dirty.delete(p.localId);
      }
      if (f.meta) {
        core.metaVersion = result.metaVersion;
        core.metaBase = f.meta;
        core.metaDirty = metaPatch(this.o.doc, core.metaBase) !== null;
      }
      core.inflight = null;
      this.retries = 0;
      this.lastSaved.set(Date.now());
      this.pending.set(core.pendingCount());
      return true;
    } catch (e) {
      if (this.disposed) return false;
      const failure = e instanceof ApiFailure ? e : new ApiFailure(0, {}, String(e));
      if (failure.code === 'conflict') {
        core.inflight = null;
        this.enterConflicts(failure.conflicts);
      } else if (failure.deleted) {
        // Refused, not committed: its changes stay dirty and so in the device draft.
        core.inflight = null;
        this.markDeleted();
      } else if (failure.transient) {
        // Same command, same key, a little later (1 s … 30 s).
        this.state.set('offline_pending');
        this.retries++;
        this.schedule(Math.min(30_000, 1000 * 2 ** Math.min(this.retries - 1, 5)));
      } else {
        // The server refused it for good (a locked layer, a missing right): say why and wait for the next edit.
        core.inflight = null;
        this.state.set('error');
        this.error.set(failure.message);
        this.o.warn(`Bulut kaydı yapılamadı: ${failure.message}`);
      }
      await this.saveDraft();
      return false;
    }
  }

  // ── Conflicts ──────────────────────────────────────────────────────────

  private enterConflicts(list: readonly FeatureConflict[]): void {
    this.addConflicts(
      list.map((c): SyncConflict => ({
        featureId: c.id,
        localId: c.id === '@project' ? null : (this.core.tracker.localOf(c.id) ?? null),
        reason: c.reason,
        server: c.current ?? null,
        actual: c.actual ?? null,
      })),
    );
  }

  private addConflicts(list: readonly SyncConflict[]): void {
    if (!list.length) return;
    const known = new Map(this.conflicts.value.map((c) => [c.featureId, c]));
    for (const c of list) known.set(c.featureId, c);
    this.conflicts.set([...known.values()]);
    clearTimeout(this.timer);
    this.state.set('conflict');
  }

  /** Ends the conflicts: take the server's copies, or keep mine and send them over the server's versions. */
  async resolve(choice: 'server' | 'mine'): Promise<void> {
    const { core } = this;
    const list = this.conflicts.value;
    const doc = this.o.doc;
    if (choice === 'server') {
      const put: Entity[] = [];
      const remove: number[] = [];
      let meta: ExternalMeta | undefined;
      for (const c of list) {
        if (c.reason === 'project') {
          const m = await core.serverMeta();
          if (this.disposed) return;
          if (m) {
            meta = m.meta;
            core.metaVersion = m.version;
          }
          continue;
        }
        if (c.localId === null && !c.server) continue;
        const incoming = c.server ? core.checked([{ key: c.featureId, entity: c.server.entity }]).get(c.featureId) : undefined;
        if (c.server && incoming) {
          const id = c.localId ?? doc.allocateId();
          const e = { ...incoming, id } as Entity;
          put.push(e);
          core.tracker.set(id, { featureId: c.featureId, version: c.server.version, json: entityJson(e) });
          core.dirty.delete(id);
        } else if (!c.server && c.localId !== null) {
          remove.push(c.localId);
          core.tracker.set(c.localId, { featureId: c.featureId, version: null, json: null });
          core.dirty.delete(c.localId);
        }
      }
      await core.whenIdle();
      if (this.disposed) return;
      doc.applyExternal({ put, remove, meta });
      if (meta) {
        core.metaBase = metaParts(doc);
        core.metaDirty = false;
      }
    } else {
      for (const c of list) {
        if (c.reason === 'project') {
          core.metaVersion = c.actual ?? core.metaVersion;
          continue;
        }
        if (c.localId === null) continue;
        const t = core.tracker.get(c.localId);
        if (c.reason === 'exists') core.tracker.set(c.localId, { featureId: (this.o.newId ?? uuid)(), version: null, json: null });
        else if (t) core.tracker.set(c.localId, { featureId: t.featureId, version: c.actual, json: c.server ? entityJson({ ...c.server.entity, id: c.localId } as Entity) : null });
        core.dirty.add(c.localId);
      }
    }
    this.conflicts.set([]);
    this.state.set('pending');
    this.pending.set(core.pendingCount());
    await this.saveDraft();
    await this.flush();
  }

  // ── Deletion ───────────────────────────────────────────────────────────

  /**
   * The project was deleted on the server: stop sending for good. What was
   * not sent, and every edit from now on, stays in the device draft.
   */
  markDeleted(): void {
    if (this.deleted || this.disposed) return;
    this.deleted = true;
    clearTimeout(this.timer);
    this.state.set('deleted');
    this.pending.set(this.core.pendingCount());
    void this.saveDraft();
    this.o.onDeleted?.();
  }

  // ── Other editors and device drafts ────────────────────────────────────

  /**
   * Applies committed events (from the socket, in order). Our own commits
   * are skipped. A deletion ends it: the objects of the events before it
   * cannot be read any more, and nothing after it is sent.
   */
  receive(events: readonly EventRecord[]): Promise<void> {
    this.remoteQueue = this.remoteQueue
      .then(async () => {
        if (this.disposed || this.deleted) return;
        if (events.some((e) => e.kind === PROJECT_DELETED)) {
          this.core.cursor = events[events.length - 1].seq;
          return this.markDeleted();
        }
        const found = await applyEvents(this.core, events);
        if (!this.disposed) this.addConflicts(found);
      })
      .catch((e) => this.o.warn(`Başka kullanıcıların değişiklikleri alınamadı: ${(e as Error).message}`));
    return this.remoteQueue;
  }

  /** Puts a draft saved on this device back into the drawing (tried again every 5 s while the server does not answer). */
  async restore(draft: Draft): Promise<void> {
    if (draft.userId !== this.o.userId || this.disposed) return;
    const r = await restoreDraft(this.core, draft);
    if (this.disposed) return;
    if (r.waiting) {
      this.state.set('offline_pending');
      clearTimeout(this.restoreTimer);
      this.restoreTimer = setTimeout(() => void this.restore(draft), RESTORE_RETRY_MS) as unknown as number;
      return;
    }
    if (r.changed) {
      this.o.doc.markUnsaved();
      this.changed();
    }
    this.addConflicts(r.conflicts);
  }
}
