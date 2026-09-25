import { Signal } from '../core/signal';
import type { DrawingFileHandle } from './fileIO';

/**
 * Drawings opened or saved lately (Son dosyalar), newest first, for the
 * start screen and the application menu. The file handles are kept in
 * IndexedDB (`kentos.files` / `recent`), so one click opens the same file
 * again; the browser asks once more for permission to read it. A handle the
 * browser cannot store (the smoke test's in-memory files) stays for this
 * session only. Nothing of the drawing itself is kept here.
 */

export interface RecentFile {
  readonly id: string;
  readonly name: string;
  /** When it was last opened or saved (ms since the epoch). */
  readonly at: number;
  /** A line about it as it was then: objects and coordinate system. */
  readonly info: string;
  readonly handle: DrawingFileHandle;
}

/** Files kept. */
const MAX = 10;
const DB = 'kentos.files';
const STORE = 'recent';

/** File System Access handles can say whether two are the same file (after a rename too). */
type SameEntry = DrawingFileHandle & { isSameEntry?(other: unknown): Promise<boolean> };

async function same(a: DrawingFileHandle, b: DrawingFileHandle): Promise<boolean> {
  if (a === b) return true;
  const s = (a as SameEntry).isSameEntry;
  if (!s) return false;
  try {
    return await s.call(a, b);
  } catch {
    return false;
  }
}

function request<T>(r: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error ?? new Error('IndexedDB isteği başarısız'));
  });
}

export class RecentFiles {
  /** Newest first. */
  readonly list = new Signal<readonly RecentFile[]>([]);
  private db: Promise<IDBDatabase | null> | null = null;
  /** Read from storage once; what is added before that is kept in front. */
  readonly ready: Promise<void>;

  constructor() {
    this.ready = this.load();
  }

  /** Puts a file first (or moves it there), with a line about it. */
  async add(handle: DrawingFileHandle, info: string): Promise<void> {
    await this.ready;
    const list = this.list.value;
    let old: RecentFile | undefined;
    for (const e of list) if (e.name === handle.name && (await same(e.handle, handle))) old = e;
    const entry: RecentFile = { id: old?.id ?? `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`, name: handle.name, at: Date.now(), info, handle };
    const next = [entry, ...list.filter((e) => e !== old)];
    const dropped = next.slice(MAX);
    this.list.set(next.slice(0, MAX));
    await this.write((s) => {
      s.put(entry);
      for (const e of dropped) s.delete(e.id);
    });
  }

  /** Forgets a file (it was moved or deleted, or the user asked). */
  async remove(id: string): Promise<void> {
    this.list.set(this.list.value.filter((e) => e.id !== id));
    await this.write((s) => s.delete(id));
  }

  private async load(): Promise<void> {
    const db = await this.open();
    if (!db) return;
    try {
      const all = (await request(db.transaction(STORE, 'readonly').objectStore(STORE).getAll())) as RecentFile[];
      const kept = all.filter((e) => e && typeof e.id === 'string' && typeof e.name === 'string' && e.handle && typeof e.handle.getFile === 'function');
      kept.sort((a, b) => b.at - a.at);
      this.list.set([...this.list.value, ...kept.filter((k) => !this.list.value.some((e) => e.id === k.id))].slice(0, MAX));
    } catch {
      /* unreadable storage: the list starts empty */
    }
  }

  /** Runs a write; a handle the browser cannot store keeps its place in this session's list only. */
  private async write(fn: (s: IDBObjectStore) => void): Promise<void> {
    const db = await this.open();
    if (!db) return;
    try {
      const t = db.transaction(STORE, 'readwrite');
      fn(t.objectStore(STORE));
      await new Promise<void>((resolve, reject) => {
        t.oncomplete = () => resolve();
        t.onerror = () => reject(t.error);
        t.onabort = () => reject(t.error);
      });
    } catch {
      /* DataCloneError (in-memory files), quota: this session's list still has it */
    }
  }

  private open(): Promise<IDBDatabase | null> {
    this.db ??= new Promise((resolve) => {
      if (typeof indexedDB === 'undefined') return resolve(null);
      try {
        const r = indexedDB.open(DB, 1);
        r.onupgradeneeded = () => r.result.createObjectStore(STORE, { keyPath: 'id' });
        r.onsuccess = () => resolve(r.result);
        r.onerror = () => resolve(null);
        r.onblocked = () => resolve(null);
      } catch {
        resolve(null);
      }
    });
    return this.db;
  }
}
