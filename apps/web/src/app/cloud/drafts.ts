import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { Entity } from '../../contracts/generated/Entity';
import type { ProjectPatch } from '../../contracts/generated/ProjectPatch';

/**
 * Unsent changes of a cloud project, kept on this device while they wait
 * (CLAUDE.md §21.3): written as edits happen, not when the tab closes, so a
 * crash or a closed laptop loses nothing. The command being sent is kept
 * too: after a reload it is sent again with the same idempotency key, so a
 * commit whose answer was lost is not made twice. A draft belongs to one
 * account and one project (`key`).
 */

/** One object's unsent state: what it should become (null = deleted) and the server version it was based on. */
export interface DraftChange {
  base: string | null;
  entity: Entity | null;
}

export interface Draft {
  userId: string;
  changes: Record<string, DraftChange>;
  meta?: { base: string; patch: ProjectPatch };
  inflight?: CommandEnvelope;
  updated: number;
}

export interface DraftStore {
  get(key: string): Promise<Draft | null>;
  put(key: string, draft: Draft): Promise<void>;
  delete(key: string): Promise<void>;
}

export const draftKey = (userId: string, tenantId: string, projectId: string) => `${userId}/${tenantId}/${projectId}`;

/** Drafts in memory only (tests, and browsers without IndexedDB, where the app says so). */
export class MemoryDraftStore implements DraftStore {
  private readonly map = new Map<string, Draft>();
  async get(key: string) {
    const d = this.map.get(key);
    return d ? structuredClone(d) : null;
  }
  async put(key: string, draft: Draft) {
    this.map.set(key, structuredClone(draft));
  }
  async delete(key: string) {
    this.map.delete(key);
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

  async get(key: string) {
    return ((await request((await this.store('readonly')).get(key))) as Draft | undefined) ?? null;
  }

  async put(key: string, draft: Draft) {
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
