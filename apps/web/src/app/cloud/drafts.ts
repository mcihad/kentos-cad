import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { Entity } from '../../contracts/generated/Entity';
import type { ProjectPatch } from '../../contracts/generated/ProjectPatch';
import { isUuid, uuidv7 } from '../../core/uuid';

/**
 * Unsent changes of a cloud project, kept on this device while they wait
 * (CLAUDE.md §21.3): written as edits happen, not when the tab closes, so a
 * crash or a closed laptop loses nothing. The command being sent is kept
 * too: after a reload it is sent again with the same idempotency key, so a
 * commit whose answer was lost is not made twice. A draft belongs to one
 * account and one project (`key`). Each change is kept under the object's
 * persistent id, which is its id on the server (docs/adr/0026).
 */

/** One object's unsent state: what it should become (null = deleted) and the server version it was based on. */
export interface DraftChange {
  base: string | null;
  entity: Entity | null;
}

/** The draft format this code writes; drafts written before persistent ids were the server's have no `version` (1). */
export const DRAFT_VERSION = 2;

export interface Draft {
  version: typeof DRAFT_VERSION;
  userId: string;
  /** By the object's persistent id: its id on the server, and the `uid` it has in the drawing. */
  changes: Record<string, DraftChange>;
  meta?: { base: string; patch: ProjectPatch };
  inflight?: CommandEnvelope;
  updated: number;
}

/** A stored value is read with `readDraft`: what IndexedDB gives back is checked, not trusted. */
export interface DraftStore {
  get(key: string): Promise<unknown>;
  put(key: string, draft: unknown): Promise<void>;
  delete(key: string): Promise<void>;
}

export const draftKey = (userId: string, tenantId: string, projectId: string) => `${userId}/${tenantId}/${projectId}`;

/** Where a stored draft this code cannot fully read is kept, as it was, before a new one takes its key. */
export const keptDraftKey = (key: string, at: number) => `${key}#unreadable-${at}`;

export interface ReadDraft {
  draft: Draft;
  /** It was written before the current format (it is written anew by the next save). */
  upgraded: boolean;
  /** What could not be read as it was, in Turkish; when any, the stored value is kept aside first. */
  problems: string[];
}

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);

/**
 * A draft as this device stored it, checked and brought to the current
 * format; null when the value is not a draft at all. Drafts written before
 * the objects' persistent ids became the server's ids (no `version`, ADR
 * 0014 slice 3) have the same shape: each change is keyed by the id the
 * object had on the server, or by the id the sync of that time made up for
 * a new object, which the command on its way (`inflight`) used too. Those
 * keys become the objects' persistent ids, so a command committed before
 * its answer was lost names the same objects as the changes after it:
 * nothing is lost and nothing is made twice. A key is made lowercase; one
 * that is not a UUID (no version wrote such keys, and the server refuses
 * them) goes to a new object with a new id, as a new object. A change that
 * cannot be read is left out and named in `problems`.
 */
export function readDraft(raw: unknown, newId: () => string = uuidv7): ReadDraft | null {
  if (!isObj(raw) || typeof raw.userId !== 'string' || !isObj(raw.changes)) return null;
  const upgraded = raw.version === undefined;
  const problems: string[] = [];
  if (!upgraded && raw.version !== DRAFT_VERSION) return null;
  const changes: Record<string, DraftChange> = {};
  for (const [key, value] of Object.entries(raw.changes)) {
    if (!isObj(value) || !(value.base === null || typeof value.base === 'string') || !(value.entity === null || isObj(value.entity))) {
      problems.push(`${key} nesnesinin değişikliği okunamadı`);
      continue;
    }
    let id = key.toLowerCase();
    let base = value.base as string | null;
    if (!isUuid(id)) {
      // Never on the server under this key: the object goes there as a new one.
      problems.push(`${key} bir nesne kimliği değil; nesne yeni kimlikle eklenecek`);
      if (!value.entity) continue;
      id = newId();
      base = null;
    }
    if (changes[id]) {
      problems.push(`${key} nesnesi taslakta iki kez var; ilki kullanıldı`);
      continue;
    }
    changes[id] = { base, entity: value.entity as Entity | null };
  }
  const meta = isObj(raw.meta) && typeof raw.meta.base === 'string' && isObj(raw.meta.patch) ? { base: raw.meta.base, patch: raw.meta.patch as ProjectPatch } : undefined;
  if (raw.meta !== undefined && !meta) problems.push('proje bilgisi değişikliği okunamadı');
  const f = raw.inflight;
  const inflight = isObj(f) && typeof f.idempotencyKey === 'string' && typeof f.requestId === 'string' && isObj(f.input) ? (f as unknown as CommandEnvelope) : undefined;
  if (f !== undefined && !inflight) problems.push('gönderilmekte olan komut okunamadı');
  return {
    draft: { version: DRAFT_VERSION, userId: raw.userId, changes, meta, inflight, updated: typeof raw.updated === 'number' ? raw.updated : 0 },
    upgraded,
    problems,
  };
}

/** Drafts in memory only (tests, and browsers without IndexedDB, where the app says so). */
export class MemoryDraftStore implements DraftStore {
  private readonly map = new Map<string, unknown>();
  async get(key: string) {
    const d = this.map.get(key);
    return d === undefined ? null : structuredClone(d);
  }
  async put(key: string, draft: unknown) {
    this.map.set(key, structuredClone(draft));
  }
  async delete(key: string) {
    this.map.delete(key);
  }
  /** Every key, for tests. */
  keys(): string[] {
    return [...this.map.keys()];
  }
}

const DB = 'kentos.cloud';
const STORE = 'drafts';

function request<T>(r: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error ?? new Error('IndexedDB hatası'));
  });
}

/** Drafts in IndexedDB (`kentos.cloud` / `drafts`). */
export class IdbDraftStore implements DraftStore {
  private db: Promise<IDBDatabase> | null = null;

  private open(): Promise<IDBDatabase> {
    this.db ??= new Promise((resolve, reject) => {
      const r = indexedDB.open(DB, 1);
      r.onupgradeneeded = () => r.result.createObjectStore(STORE);
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error ?? new Error('IndexedDB açılamadı'));
      r.onblocked = () => reject(new Error('IndexedDB başka bir sekmede kilitli'));
    });
    return this.db;
  }

  private async store(mode: IDBTransactionMode): Promise<IDBObjectStore> {
    return (await this.open()).transaction(STORE, mode).objectStore(STORE);
  }

  async get(key: string): Promise<unknown> {
    return (await request((await this.store('readonly')).get(key))) ?? null;
  }

  async put(key: string, draft: unknown) {
    const tx = (await this.open()).transaction(STORE, 'readwrite');
    tx.objectStore(STORE).put(draft, key);
    // Durable once the transaction completes, not when the request succeeds.
    await new Promise<void>((resolve, reject) => {
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error ?? new Error('Taslak yazılamadı'));
      tx.onabort = () => reject(tx.error ?? new Error('Taslak yazılamadı'));
    });
  }

  async delete(key: string) {
    await request((await this.store('readwrite')).delete(key));
  }
}

/** IndexedDB when the browser has it; otherwise memory (the caller warns that drafts do not survive a reload). */
export function browserDraftStore(): { store: DraftStore; durable: boolean } {
  return typeof indexedDB === 'undefined' ? { store: new MemoryDraftStore(), durable: false } : { store: new IdbDraftStore(), durable: true };
}
