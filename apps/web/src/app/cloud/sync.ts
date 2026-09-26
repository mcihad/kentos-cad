import { Signal } from '../../core/signal';
import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { EventRecord } from '../../contracts/generated/EventRecord';
import type { FeatureConflict } from '../../contracts/generated/FeatureConflict';
import type { ExternalMeta } from '../../model/document';
import type { Entity } from '../../model/entities';
import { ApiFailure } from './api';
import { DRAFT_VERSION, keptDraftKey, readDraft, type Draft } from './drafts';
import { BATCH, PROJECT_ACCESS, PROJECT_ARCHIVED, PROJECT_DELETED, SyncCore, uuid, type Inflight, type SaveState, type SyncConflict, type SyncOptions } from './syncCore';
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
 * server's answer, with nothing left to send. An object's persistent id is
 * its id on the server (docs/adr/0026): a new object is created under its
 * `uid`, and an undone deletion creates it again under the same one.
 *
 * - A lost answer or a dead network: the same command goes again with the
 *   same idempotency key (the server answers from its log, never twice).
 * - A 409: nothing more is sent until the user chooses the server's copy or
 *   theirs; the local drawing is kept either way until then.
 * - Other editors' changes (syncRemote.ts): applied at once, unless the
 *   object has unsent local changes, which makes it a conflict too.
 * - A device draft from an earlier session (syncRestore.ts) goes back in
 *   when the project is opened again.
 * - The project deleted on the server (its event, or a 410), archived (its
 *   event, or a 409 `project_archived`; docs/adr/0028), or this account's
 *   access taken away (a 404; docs/adr/0015, TODOS.md CLOUD-13): nothing
 *   more is sent, and edits keep going to the device draft (a restored,
 *   unarchived or again shared project brings them back when opened).
 * - The account's role changed while the project is open (`setAccess`, after
 *   a `project.access` event or a 403): without the right to write nothing
 *   is sent, and edits are kept on the device until it comes back.
 */

/** What a change of the account's access did to the autosave (`ProjectSync.setAccess`). */
export type AccessOutcome = 'same' | 'held' | 'resumed' | 'reopen';

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
  /** Why nothing is sent any more: the project was deleted or archived, or this account lost its access. */
  private ended: 'deleted' | 'revoked' | 'archived' | null = null;
  /** Whether this account may change objects now. */
  private writable: boolean;
  /**
   * Whether its edits are kept on this device and sent once writing is
   * possible: from the start for one who may write, and still after the role
   * is lowered. A viewer's own edits never are (they were told so).
   */
  private keeps: boolean;

  constructor(o: SyncOptions) {
    this.o = o;
    this.core = new SyncCore(o);
    this.writable = o.canWrite !== false;
    this.keeps = this.writable;
    const doc = o.doc;
    this.unsubscribe.push(
      doc.events.on('touched', (e) => {
        if (e.external) return;
        for (const id of e.uids) {
          this.core.dirty.add(id);
          // Edited here: this edit replaces an older one the drawing could not take from the device draft.
          this.core.held.delete(id);
        }
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

  /** Whether this account's edits are kept on this device while they cannot be sent (not a viewer's own). */
  get keepsEdits(): boolean {
    return this.keeps;
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

  /** The server's version of an object (by persistent id) as this sync last saw it; for tests and diagnostics. */
  versionOf(id: string): string | undefined {
    return this.core.tracker.get(id)?.version;
  }

  private metaChanged(): void {
    // The quiet external meta changes also arrive here; they leave the drawing equal to the base.
    if (metaPatch(this.o.doc, this.core.metaBase) === null) return;
    this.core.metaDirty = true;
    this.changed();
  }

  private changed(): void {
    if (this.disposed) return;
    if (!this.keeps) {
      if (!this.noticeShown) {
        this.noticeShown = true;
        this.o.warn(
          this.ended === 'archived'
            ? 'Bu proje arşivlenmiş, salt okunurdur; değişiklikleriniz buluta kaydedilmez. Düzenlemek için arşivden çıkarılmalı ya da kopyası oluşturulmalı.'
            : 'Bu projeyi yalnız görüntüleyebilirsiniz; değişiklikleriniz buluta kaydedilmez.',
        );
      }
      if (!this.ended) this.state.set('readonly');
      return;
    }
    this.pending.set(this.core.pendingCount());
    if (this.ended || !this.writable) {
      // Nothing goes out now; the edit is kept on this device.
      clearTimeout(this.draftTimer);
      this.draftTimer = setTimeout(() => void this.saveDraft(), DRAFT_MS) as unknown as number;
      return;
    }
    if (this.core.metaDirty && !this.core.canEditMeta && !this.noticeShown) {
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
    // What the drawing could not take from an earlier draft is kept too; an edit made here replaces it.
    const changes: Draft['changes'] = Object.fromEntries(core.held);
    for (const id of core.dirty) {
      const p = core.tracker.plan(this.o.doc, id);
      if (!p) continue;
      const base = p.op === 'create' ? null : p.expected;
      changes[p.id] = { base, entity: p.op === 'delete' ? null : structuredClone(p.entity) };
    }
    const patch = core.sendsMeta() ? metaPatch(this.o.doc, core.metaBase) : null;
    if (!Object.keys(changes).length && !patch && !core.inflight) return null;
    return {
      version: DRAFT_VERSION,
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
    if (!this.keeps) return;
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
    if (this.disposed || this.ended || this.conflicts.value.length || !this.writable) return false;
    const doc = this.o.doc;
    if (doc.busy) {
      this.schedule(200);
      return false;
    }
    clearTimeout(this.timer);
    if (core.inflight && !(await this.send(core.inflight))) return false;
    for (;;) {
      // Left, ended, or the role lowered while the last batch went out: what is left stays on the device.
      if (this.disposed || this.ended || !this.writable) return false;
      const planned: Planned[] = [];
      for (const id of [...core.dirty]) {
        const p = core.tracker.plan(doc, id);
        if (p) planned.push(p);
        else core.dirty.delete(id);
        if (planned.length >= BATCH) break;
      }
      const patch = core.sendsMeta() ? metaPatch(doc, core.metaBase) : null;
      if (!patch) core.metaDirty = core.metaDirty && !core.canEditMeta && metaPatch(doc, core.metaBase) !== null;
      if (!planned.length && !patch) break;
      const expectedVersions: Record<string, string> = {};
      for (const p of planned) if (p.op !== 'create') expectedVersions[p.id] = p.expected;
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
        core.tracker.acknowledge(p, result.versions[p.id]);
        if (core.tracker.settled(this.o.doc, p)) core.dirty.delete(p.id);
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
      } else if (failure.archived) {
        // The same: archived before this command reached it.
        core.inflight = null;
        this.markArchived();
      } else if (failure.notFound) {
        // The project is gone for this account: its access was taken away. The command stays in the
        // device draft with its key: shared again and opened, it goes once more and the server answers it once.
        this.markRevoked();
      } else if (failure.transient) {
        // Same command, same key, a little later (1 s … 30 s).
        this.state.set('offline_pending');
        this.retries++;
        this.schedule(Math.min(30_000, 1000 * 2 ** Math.min(this.retries - 1, 5)));
      } else {
        // The server refused it for good (a locked layer, a missing right): say why and wait for the next edit.
        core.inflight = null;
        if (this.writable) {
          this.state.set('error');
          this.error.set(failure.message);
          this.o.warn(`Bulut kaydı yapılamadı: ${failure.message}`);
        } else this.state.set('readonly');
        // A missing right may mean the role was lowered meanwhile: the session asks again.
        if (failure.code === 'forbidden') this.o.onAccessChanged?.();
      }
      await this.saveDraft();
      return false;
    }
  }

  // ── Conflicts ──────────────────────────────────────────────────────────

  private enterConflicts(list: readonly FeatureConflict[]): void {
    this.addConflicts(list.map((c): SyncConflict => ({ featureId: c.id, reason: c.reason, server: c.current ?? null, actual: c.actual ?? null })));
  }

  private addConflicts(list: readonly SyncConflict[]): void {
    if (!list.length) return;
    const known = new Map(this.conflicts.value.map((c) => [c.featureId, c]));
    for (const c of list) known.set(c.featureId, c);
    this.conflicts.set([...known.values()]);
    clearTimeout(this.timer);
    this.state.set('conflict');
  }

  /**
   * Ends the conflicts: take the server's copies, or keep mine and send them
   * over the server's versions. An object is the same object whoever changed
   * it or brought it back (`exists`: the server already has this id): mine
   * goes as a change over the server's version, or is created again under
   * its id when the server no longer has it.
   */
  async resolve(choice: 'server' | 'mine'): Promise<void> {
    const { core } = this;
    const list = this.conflicts.value;
    const doc = this.o.doc;
    if (choice === 'server') {
      let meta: ExternalMeta | undefined;
      if (list.some((c) => c.reason === 'project')) {
        const m = await core.serverMeta();
        if (this.disposed) return;
        if (m) {
          meta = m.meta;
          core.metaVersion = m.version;
        }
      }
      const objects = list.filter((c) => c.reason !== 'project');
      const good = core.checked(objects.flatMap((c) => (c.server ? [{ key: c.featureId, entity: c.server.entity }] : [])));
      // Decided once no edit is open, and applied at once: no edit slips in between.
      await core.whenIdle();
      if (this.disposed) return;
      const put: Entity[] = [];
      const remove: number[] = [];
      for (const c of objects) {
        const slot = doc.slotOf(c.featureId);
        const incoming = good.get(c.featureId);
        if (c.server && incoming) {
          put.push({ ...incoming, id: slot ?? doc.allocateId(), uid: c.featureId } as Entity);
          core.tracker.set(c.featureId, { version: c.server.version, json: entityJson(incoming) });
          core.dirty.delete(c.featureId);
        } else if (!c.server) {
          if (slot !== undefined) remove.push(slot);
          core.tracker.set(c.featureId, null);
          core.dirty.delete(c.featureId);
        }
      }
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
        // What the server has now is the base mine goes over (nothing: mine is created again).
        core.tracker.set(c.featureId, c.actual === null ? null : { version: c.actual, json: c.server ? entityJson(c.server.entity as Entity) : '' });
        core.dirty.add(c.featureId);
      }
    }
    this.conflicts.set([]);
    this.state.set(this.writable ? 'pending' : 'readonly');
    this.pending.set(core.pendingCount());
    await this.saveDraft();
    await this.flush();
  }

  // ── Deletion and access ────────────────────────────────────────────────

  /** Stops sending for good: what was not sent, and every edit from now on, stays in the device draft. */
  private end(why: 'deleted' | 'revoked' | 'archived'): boolean {
    if (this.ended || this.disposed) return false;
    this.ended = why;
    clearTimeout(this.timer);
    this.state.set(why);
    this.pending.set(this.core.pendingCount());
    if (this.keeps) void this.saveDraft();
    return true;
  }

  /** The project was deleted on the server. */
  markDeleted(): void {
    if (this.end('deleted')) this.o.onDeleted?.();
  }

  /**
   * This account lost its access to the project (a 404, or the session found
   * out); `reason` is the server's when it said more than “not found” (a
   * membership that may not be used now).
   */
  markRevoked(reason = ''): void {
    if (this.end('revoked')) this.o.onRevoked?.(reason);
  }

  /**
   * The project was archived (docs/adr/0028): read-only until it is
   * unarchived and opened again. `quiet`: nothing is announced (it was
   * archived already when it was opened, or this window archived it).
   */
  markArchived(quiet = false): void {
    if (this.end('archived') && !quiet) this.o.onArchived?.();
  }

  /** A request this window sends outside the autosave (a lifecycle command): its event is its own. */
  expect(requestId: string): void {
    this.core.own.add(requestId);
  }

  /**
   * What this account may do changed while the project is open (a manager
   * changed its role). Losing the right to write stops sending: what waits,
   * and every edit from now on, is kept on this device and goes out once
   * writing is possible again (`held`, later `resumed`). Gaining it starts
   * sending, unless edits made while the account could only view are on
   * screen: those were never kept, so they are not sent now either, and
   * opening the project again starts clean (`reopen`).
   */
  setAccess(canWrite: boolean, canEditMeta: boolean): AccessOutcome {
    this.core.canEditMeta = canEditMeta;
    if (this.ended || this.disposed || canWrite === this.writable) return 'same';
    if (!canWrite) {
      this.writable = false;
      clearTimeout(this.timer);
      this.state.set('readonly');
      this.pending.set(this.core.pendingCount());
      if (this.keeps) void this.saveDraft();
      return 'held';
    }
    if (!this.keeps && this.core.dirty.size) return 'reopen';
    this.writable = true;
    this.keeps = true;
    this.pending.set(this.core.pendingCount());
    const waiting = this.pending.value > 0 || !!this.core.inflight;
    this.state.set(waiting ? 'pending' : 'saved');
    if (waiting) this.schedule(0);
    return 'resumed';
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
        if (this.disposed || this.ended) return;
        if (events.some((e) => e.kind === PROJECT_DELETED)) {
          this.core.cursor = events[events.length - 1].seq;
          return this.markDeleted();
        }
        // Archived: the changes of the events before it still come in; nothing is sent after it.
        const archived = events.findIndex((e) => e.kind === PROJECT_ARCHIVED);
        if (archived >= 0) {
          const found = await applyEvents(this.core, events.slice(0, archived + 1));
          if (!this.disposed) this.addConflicts(found);
          const by = events[archived].requestId;
          return this.markArchived(!!by && this.core.own.has(by));
        }
        // Someone changed who may do what here: the session asks what this account may do now.
        if (events.some((e) => e.kind === PROJECT_ACCESS)) this.o.onAccessChanged?.();
        const found = await applyEvents(this.core, events);
        if (!this.disposed) this.addConflicts(found);
      })
      .catch((e) => this.o.warn(`Başka kullanıcıların değişiklikleri alınamadı: ${(e as Error).message}`));
    return this.remoteQueue;
  }

  /**
   * Puts a draft saved on this device back into the drawing (tried again
   * every 5 s while the server does not answer). What the store gave back
   * is read first (`readDraft`): a draft of the earlier format is brought
   * over (docs/adr/0026). One that cannot be fully read, or is not this
   * account's, is first kept aside as it was, under a key of its own, so the
   * next save cannot lose it; what can be read of it still comes back.
   * Returns whether a draft of this account was found.
   */
  async restore(raw: unknown): Promise<boolean> {
    if (this.disposed) return false;
    const read = readDraft(raw);
    const mine = !!read && read.draft.userId === this.o.userId;
    if (!mine || read.problems.length) {
      const kept = await this.o.drafts.put(keptDraftKey(this.o.draftKey, Date.now()), raw).then(
        () => true,
        () => false,
      );
      const what = !read ? 'okunamadı' : !mine ? 'başka bir hesabın' : `bir kısmı okunamadı (${read.problems.join('; ')})`;
      this.o.warn(`Bu projenin cihazdaki taslağı ${what}; taslak olduğu gibi ${kept ? 'ayrıca saklandı' : 'ayrıca saklanamadı'}.`);
    }
    if (!mine || this.disposed) return false;
    await this.restoreRead(read.draft);
    return true;
  }

  private async restoreRead(draft: Draft): Promise<void> {
    if (this.disposed) return;
    const r = await restoreDraft(this.core, draft);
    if (this.disposed) return;
    if (r.waiting) {
      this.state.set('offline_pending');
      clearTimeout(this.restoreTimer);
      this.restoreTimer = setTimeout(() => void this.restoreRead(draft), RESTORE_RETRY_MS) as unknown as number;
      return;
    }
    if (r.changed) {
      this.o.doc.markUnsaved();
      this.changed();
    }
    this.addConflicts(r.conflicts);
  }
}
