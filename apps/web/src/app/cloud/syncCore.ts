import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { FeatureRecord } from '../../contracts/generated/FeatureRecord';
import type { CadDocument, ExternalMeta } from '../../model/document';
import type { Entity } from '../../model/entities';
import type { CloudApi } from './api';
import type { DraftChange, DraftStore } from './drafts';
import { readEntities, readIncoming } from './incoming';
import { Tracker, entityJson, metaParts, type MetaParts, type Planned } from './tracker';

/**
 * The state one open cloud project's sync shares between its parts: sending
 * (sync.ts), other editors' events (syncRemote.ts) and device drafts
 * (syncRestore.ts). Objects are named by their persistent id everywhere
 * here, which is their id on the server (docs/adr/0026); a slot is asked of
 * the drawing only to put an object in or take it out. What the server has
 * of each object lives in the tracker; `dirty` holds the objects that may
 * differ from it.
 */

/**
 * `deleted`: the project was deleted on the server (moved to the trash);
 * `revoked`: this account lost its access to it; `archived`: it was archived
 * (read-only until it is unarchived, docs/adr/0028). Either way nothing more
 * is sent and edits stay in the device draft (a viewer's never are).
 * `readonly`: this account may only view (from the start, or since its role
 * was lowered: then its edits are kept on the device too).
 */
export type SaveState = 'saved' | 'pending' | 'saving' | 'offline_pending' | 'conflict' | 'error' | 'readonly' | 'deleted' | 'revoked' | 'archived';

/** The states in which nothing more is sent to the project: it is gone, out of reach, or archived. */
export const ENDED: readonly SaveState[] = ['deleted', 'revoked', 'archived'];

/** Event kind of a deleted project (`PROJECT_DELETED` in crates/shared/contracts). */
export const PROJECT_DELETED = 'project.deleted';
/** Event kind of a changed grant (`PROJECT_ACCESS_CHANGED`): what this account may do is asked again. */
export const PROJECT_ACCESS = 'project.access';
/** Event kind of an archived project (`PROJECT_ARCHIVED`): nothing more is sent until it is opened again unarchived. */
export const PROJECT_ARCHIVED = 'project.archived';

export interface SyncConflict {
  /** The object's persistent id (its id on the server), or `@project` for the metadata. */
  featureId: string;
  reason: 'changed' | 'deleted' | 'exists' | 'project' | 'remote';
  /** The server's current copy (null when deleted, or for the project's metadata). */
  server: FeatureRecord | null;
  /** The server's current version (null when deleted). */
  actual: string | null;
}

export interface SyncOptions {
  doc: CadDocument;
  api: CloudApi;
  drafts: DraftStore;
  draftKey: string;
  userId: string;
  tenantId: string;
  projectId: string;
  /** Whether this account may change the project's metadata (layer tree, settings, name, styles). */
  canEditMeta: boolean;
  /** Whether it may change objects at all (a viewer may not: edits stay on screen, unsent). */
  canWrite?: boolean;
  metaVersion: string;
  cursor: string;
  /** What the server sent when the project was opened: each object's id (its `uid` in the drawing) and version. */
  records: readonly { id: string; version: string }[];
  warn: (text: string) => void;
  /** The project was deleted on the server (an event or a refused command); called once. */
  onDeleted?: () => void;
  /** This account lost its access (a command answered 404, or the session found out); called once, with the server's reason when it gave one. */
  onRevoked?: (reason: string) => void;
  /** The project was archived (an event, or a command refused with 409); called once. */
  onArchived?: () => void;
  /** A grant of the project changed, or a command was refused (403): the session asks what this account may do now. */
  onAccessChanged?: () => void;
  debounceMs?: number;
  maxDelayMs?: number;
}

/** The command on its way (or waiting for an answer that was lost). */
export interface Inflight {
  envelope: CommandEnvelope;
  planned: Planned[];
  meta: MetaParts | null;
  revision: number;
}

export const BATCH = 2000;
export const uuid = () => crypto.randomUUID();

export class SyncCore {
  readonly o: SyncOptions;
  readonly tracker = new Tracker();
  /** Persistent ids of the objects that may differ from the server (edited here, not sent yet). */
  readonly dirty = new Set<string>();
  /**
   * Changes from a device draft the drawing could not take (an object on a
   * layer that is gone, say): never sent, but written to the device draft
   * again, so unsent work is not lost (CLAUDE.md §21.3), until an edit of the
   * same object here replaces it.
   */
  readonly held = new Map<string, DraftChange>();
  /** Request ids of our own commits (their events are skipped). */
  readonly own = new Set<string>();
  metaDirty = false;
  /** Whether this account may change the project's metadata now (a manager may lower it while the project is open). */
  canEditMeta: boolean;
  metaVersion: string;
  metaBase: MetaParts;
  inflight: Inflight | null = null;
  /** The newest event cursor applied. */
  cursor: string;
  /**
   * The project was left (the sync disposed): the drawing may already hold
   * another project, so nothing may read or change it on this project's
   * behalf any more. Every step that awaits checks this before going on.
   */
  closed = false;

  constructor(o: SyncOptions) {
    this.o = o;
    this.cursor = o.cursor;
    this.canEditMeta = o.canEditMeta;
    this.metaVersion = o.metaVersion;
    for (const r of o.records) {
      const e = o.doc.byUid(r.id);
      if (e) this.tracker.set(r.id, { version: r.version, json: entityJson(e) });
    }
    this.metaBase = metaParts(o.doc);
  }

  sendsMeta(): boolean {
    return this.metaDirty && this.canEditMeta;
  }

  /** Objects and the metadata waiting to be sent. */
  pendingCount(): number {
    return this.dirty.size + (this.sendsMeta() ? 1 : 0);
  }

  /** Whether an object has unsent edits here (or is in the command on its way). */
  busyLocally(id: string): boolean {
    return this.dirty.has(id) || !!this.inflight?.planned.some((p) => p.id === id);
  }

  /** Resolves when no edit or group is open in the drawing. */
  whenIdle(): Promise<void> {
    return new Promise((resolve) => {
      const check = () => (this.o.doc.busy ? setTimeout(check, 100) : resolve());
      check();
    });
  }

  /**
   * Objects that came from the server or the device, checked like a file; a
   * bad one is left out and reported (`bad`; by default a warning that the
   * server sent it).
   */
  checked(list: readonly { key: string; entity: unknown }[], bad?: (key: string, error: string) => void): Map<string, Entity> {
    const out = new Map<string, Entity>();
    if (!list.length) return out;
    // One check for the batch; only a failing batch is checked object by object, to name the bad one.
    const all = readEntities(this.o.doc, list.map((i) => i.entity));
    if (all.ok) {
      list.forEach((item, i) => out.set(item.key, all.entities[i]));
      return out;
    }
    for (const item of list) {
      const read = readEntities(this.o.doc, [item.entity]);
      if (read.ok) out.set(item.key, read.entities[0]);
      else if (bad) bad(item.key, read.error);
      else this.o.warn(`Buluttan gelen bir nesne okunamadı (${item.key}): ${read.error}`);
    }
    return out;
  }

  /** The project's metadata from the server, checked; null (and a warning) when unreadable. */
  async serverMeta(): Promise<{ meta: ExternalMeta; version: string } | null> {
    const info = await this.o.api.project(this.o.tenantId, this.o.projectId);
    const read = readIncoming(info, []);
    if (!read.ok) {
      this.o.warn(`Proje bilgileri sunucudan okunamadı: ${read.error}`);
      return null;
    }
    const c = read.content;
    return { meta: { name: c.name, settings: c.settings, layers: c.layers, styles: c.styles }, version: info.metaVersion };
  }

  /**
   * Server records put into the drawing under their ids (an object already
   * here keeps its slot), tracked at their versions; missing ones removed.
   * An object with unsent local edits keeps them: only its server version is
   * updated, so the edit goes out on top of it. Decided once no edit is
   * open, and applied at once, so no edit slips in between.
   */
  async takeServerCopies(ids: readonly string[]): Promise<void> {
    if (!ids.length) return;
    const fresh = await this.o.api.featuresById(this.o.tenantId, this.o.projectId, ids);
    if (this.closed) return;
    const byId = new Map(fresh.features.map((f) => [f.id, f]));
    const good = this.checked(fresh.features.map((f) => ({ key: f.id, entity: f.entity })));
    await this.whenIdle();
    if (this.closed) return;
    const doc = this.o.doc;
    const put: Entity[] = [];
    const remove: number[] = [];
    for (const id of ids) {
      const slot = doc.slotOf(id);
      const rec = byId.get(id);
      const incoming = good.get(id);
      const keep = this.dirty.has(id);
      if (rec && incoming) {
        if (!keep) put.push({ ...incoming, id: slot ?? doc.allocateId(), uid: id } as Entity);
        this.tracker.set(id, { version: rec.version, json: entityJson(incoming) });
      } else if (!rec) {
        if (!keep && slot !== undefined) remove.push(slot);
        this.tracker.set(id, null);
      }
    }
    doc.applyExternal({ put, remove });
  }
}
