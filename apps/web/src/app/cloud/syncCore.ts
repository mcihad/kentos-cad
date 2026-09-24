import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { FeatureRecord } from '../../contracts/generated/FeatureRecord';
import type { CadDocument, ExternalMeta } from '../../model/document';
import type { Entity } from '../../model/entities';
import type { CloudApi } from './api';
import type { DraftStore } from './drafts';
import { readEntities, readIncoming } from './incoming';
import { Tracker, entityJson, metaParts, type MetaParts, type Planned } from './tracker';

/**
 * The state one open cloud project's sync shares between its parts: sending
 * (sync.ts), other editors' events (syncRemote.ts) and device drafts
 * (syncRestore.ts). What the server has of each object lives in the
 * tracker; `dirty` holds the local objects that may differ from it.
 */

/** `deleted`: the project was deleted on the server; nothing more is sent, edits stay in the device draft. */
export type SaveState = 'saved' | 'pending' | 'saving' | 'offline_pending' | 'conflict' | 'error' | 'readonly' | 'deleted';

/** Event kind of a deleted project (`PROJECT_DELETED` in crates/shared/contracts). */
export const PROJECT_DELETED = 'project.deleted';

export interface SyncConflict {
  featureId: string;
  localId: number | null;
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
  /** What the server sent when the project was opened. */
  records: readonly { localId: number; featureId: string; version: string }[];
  warn: (text: string) => void;
  /** The project was deleted on the server (an event or a refused command); called once. */
  onDeleted?: () => void;
  newId?: () => string;
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
  readonly tracker: Tracker;
  readonly dirty = new Set<number>();
  /** Request ids of our own commits (their events are skipped). */
  readonly own = new Set<string>();
  metaDirty = false;
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
    this.metaVersion = o.metaVersion;
    this.tracker = new Tracker(o.newId ?? uuid);
    for (const r of o.records) {
      const e = o.doc.get(r.localId);
      this.tracker.set(r.localId, { featureId: r.featureId, version: r.version, json: e ? entityJson(e) : null });
    }
    this.metaBase = metaParts(o.doc);
  }

  sendsMeta(): boolean {
    return this.metaDirty && this.o.canEditMeta;
  }

  /** Objects and the metadata waiting to be sent. */
  pendingCount(): number {
    return this.dirty.size + (this.sendsMeta() ? 1 : 0);
  }

  /** Whether a local object has unsent edits (or is in the command on its way). */
  busyLocally(localId: number | undefined): boolean {
    if (localId === undefined) return false;
    return this.dirty.has(localId) || !!this.inflight?.planned.some((p) => p.localId === localId);
  }

  /** Resolves when no edit or group is open in the drawing. */
  whenIdle(): Promise<void> {
    return new Promise((resolve) => {
      const check = () => (this.o.doc.busy ? setTimeout(check, 100) : resolve());
      check();
    });
  }

  /** Objects that came from the server or the device, checked like a file; a bad one is reported and left out. */
  checked(list: readonly { key: string; entity: unknown }[]): Map<string, Entity> {
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
   * Server records put into the drawing (keeping local ids), tracked at their
   * versions; missing ones removed. An object with unsent local edits keeps
   * them: only its server version is updated, so the edit goes out on top of it.
   */
  async takeServerCopies(ids: readonly string[]): Promise<void> {
    if (!ids.length) return;
    const fresh = await this.o.api.featuresById(this.o.tenantId, this.o.projectId, ids);
    if (this.closed) return;
    const byId = new Map(fresh.features.map((f) => [f.id, f]));
    const good = this.checked(fresh.features.map((f) => ({ key: f.id, entity: f.entity })));
    const doc = this.o.doc;
    const put: Entity[] = [];
    const remove: number[] = [];
    for (const id of ids) {
      const local = this.tracker.localOf(id);
      const rec = byId.get(id);
      const incoming = good.get(id);
      const keep = local !== undefined && this.dirty.has(local);
      if (rec && incoming) {
        const lid = local ?? doc.allocateId();
        const e = { ...incoming, id: lid } as Entity;
        if (!keep) put.push(e);
        this.tracker.set(lid, { featureId: id, version: rec.version, json: entityJson(e) });
      } else if (!rec && local !== undefined) {
        if (!keep) remove.push(local);
        this.tracker.set(local, { featureId: id, version: null, json: null });
      }
    }
    await this.whenIdle();
    if (this.closed) return;
    doc.applyExternal({ put, remove });
  }
}
