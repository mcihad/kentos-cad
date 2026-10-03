import type { AssetMeta } from '../../contracts/generated/sheet/AssetMeta';

/**
 * Where the sheets stay on this device (docs/sheet/design.md §3.4, §10, §12):
 * each project's book, kept apart from the project file until `.kcad`
 * carries it (integration.md §6), the user's own templates saved on this
 * device and the pictures' bytes the books name by their SHA-256. They live
 * in IndexedDB `kentos.sheets.v1`: `books` by the project's key
 * (app/sheet/projectKey.ts), `templates` by the template's id, `assets` by
 * the digest. What is stored is an envelope this code writes and checks
 * (format, version, key, when) round the engine's JSON, which only the
 * engine reads and validates. A stored value this code cannot read is kept
 * aside under its own key before anything takes its place: no path loses
 * data silently.
 */

export const SHEETS_DB = 'kentos.sheets.v1';
/** The database's version: 2 added `assets` (a version-1 database is opened and given it). */
export const SHEETS_DB_VERSION = 2;
export const BOOKS = 'books';
export const TEMPLATES = 'templates';
export const ASSETS = 'assets';

export type StoreName = typeof BOOKS | typeof TEMPLATES | typeof ASSETS;
const STORES: readonly StoreName[] = [BOOKS, TEMPLATES, ASSETS];

/** The few things the stores need of IndexedDB; tests and browsers without it use memory. */
export interface KeyValue {
  get(store: StoreName, key: string): Promise<unknown>;
  put(store: StoreName, key: string, value: unknown): Promise<void>;
  delete(store: StoreName, key: string): Promise<void>;
  keys(store: StoreName): Promise<string[]>;
}

/** Values in memory, copied in and out as IndexedDB copies them (tests; a browser without IndexedDB). */
export class MemoryKeyValue implements KeyValue {
  private readonly stores = new Map<StoreName, Map<string, unknown>>();

  private map(store: StoreName): Map<string, unknown> {
    let m = this.stores.get(store);
    if (!m) this.stores.set(store, (m = new Map()));
    return m;
  }

  async get(store: StoreName, key: string): Promise<unknown> {
    const v = this.map(store).get(key);
    return v === undefined ? null : structuredClone(v);
  }

  async put(store: StoreName, key: string, value: unknown): Promise<void> {
    this.map(store).set(key, structuredClone(value));
  }

  async delete(store: StoreName, key: string): Promise<void> {
    this.map(store).delete(key);
  }

  async keys(store: StoreName): Promise<string[]> {
    return [...this.map(store).keys()].sort();
  }
}

function request<T>(r: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error ?? new Error('IndexedDB hatası'));
  });
}

/** IndexedDB `kentos.sheets.v1` (`books`, `templates`, `assets`). */
export class IdbKeyValue implements KeyValue {
  private db: Promise<IDBDatabase> | null = null;
  private readonly factory: IDBFactory;

  constructor(factory: IDBFactory) {
    this.factory = factory;
  }

  private open(): Promise<IDBDatabase> {
    this.db ??= new Promise((resolve, reject) => {
      const r = this.factory.open(SHEETS_DB, SHEETS_DB_VERSION);
      // A new database gets every store; one of an older version the ones it lacks (what it holds stays).
      r.onupgradeneeded = () => {
        for (const name of STORES) if (!r.result.objectStoreNames.contains(name)) r.result.createObjectStore(name);
      };
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error ?? new Error('IndexedDB açılamadı'));
      r.onblocked = () => reject(new Error('IndexedDB başka bir sekmede kilitli'));
    });
    // A failed open is tried again next time (another tab may have let go of it).
    this.db.catch(() => (this.db = null));
    return this.db;
  }

  async get(store: StoreName, key: string): Promise<unknown> {
    const db = await this.open();
    return (await request(db.transaction(store, 'readonly').objectStore(store).get(key))) ?? null;
  }

  async put(store: StoreName, key: string, value: unknown): Promise<void> {
    const tx = (await this.open()).transaction(store, 'readwrite');
    tx.objectStore(store).put(value, key);
    // Kept once the transaction completes, not when the request succeeds.
    await new Promise<void>((resolve, reject) => {
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error ?? new Error('Yazılamadı'));
      tx.onabort = () => reject(tx.error ?? new Error('Yazılamadı'));
    });
  }

  async delete(store: StoreName, key: string): Promise<void> {
    const db = await this.open();
    await request(db.transaction(store, 'readwrite').objectStore(store).delete(key));
  }

  async keys(store: StoreName): Promise<string[]> {
    const db = await this.open();
    const keys = await request(db.transaction(store, 'readonly').objectStore(store).getAllKeys());
    return keys.map(String).sort();
  }
}

/** IndexedDB when the browser has it; otherwise memory, and `durable` says the stores do not survive a reload. */
export function deviceKeyValue(factory: IDBFactory | undefined = globalThis.indexedDB): { kv: KeyValue; durable: boolean } {
  return factory ? { kv: new IdbKeyValue(factory), durable: true } : { kv: new MemoryKeyValue(), durable: false };
}

/** The key a stored value nobody could read is kept under before a new one takes its place. */
export const keptKey = (key: string, at: number) => `${key}#okunamadi-${at}`;

export const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);

// ── Books ───────────────────────────────────────────────────────────────

export const BOOK_FORMAT = 'kentos.sheets.book';
export const BOOK_VERSION = 1;

/** A project's book as this device keeps it. */
export interface StoredBook {
  readonly format: typeof BOOK_FORMAT;
  readonly version: typeof BOOK_VERSION;
  /** The project's key, as it was found under (a copy under another key is not that project's). */
  readonly key: string;
  /** When it was written, ms since 1970. */
  readonly saved: number;
  /** The engine's `SheetBook` (kentos.sheet/1); read and checked by the engine only. */
  readonly book: unknown;
}

export type ReadBook =
  | { readonly status: 'none' }
  | { readonly status: 'ok'; readonly record: StoredBook }
  /** Something is stored that this code cannot read (another version, damaged): kept aside before anything replaces it. */
  | { readonly status: 'unreadable'; readonly reason: string };

/** A stored value as a book record; what IndexedDB gives back is checked, not trusted. */
export function readStoredBook(raw: unknown, key: string): ReadBook {
  if (raw === null || raw === undefined) return { status: 'none' };
  if (!isObj(raw) || raw.format !== BOOK_FORMAT) return { status: 'unreadable', reason: 'kayıt bir pafta kitabı değil' };
  if (raw.version !== BOOK_VERSION) return { status: 'unreadable', reason: `kayıt bu sürümün bilmediği ${String(raw.version)}. biçimde` };
  if (raw.key !== key) return { status: 'unreadable', reason: 'kayıt başka bir projenin anahtarını taşıyor' };
  if (typeof raw.saved !== 'number' || !Number.isFinite(raw.saved)) return { status: 'unreadable', reason: 'kaydın zamanı okunamadı' };
  if (!isObj(raw.book)) return { status: 'unreadable', reason: 'kitabın içeriği yok' };
  return { status: 'ok', record: { format: BOOK_FORMAT, version: BOOK_VERSION, key, saved: raw.saved, book: raw.book } };
}

/**
 * The sheets' ids and names in a stored book, for the tabs until the engine
 * reads the book (it is fetched when a tab is opened): only strings are
 * taken, anything else is passed over. Never a book: just its names.
 */
export function peekSheets(book: unknown): { id: string; name: string }[] {
  if (!isObj(book) || !Array.isArray(book.sheets)) return [];
  const out: { id: string; name: string }[] = [];
  for (const s of book.sheets) if (isObj(s) && typeof s.id === 'string' && typeof s.name === 'string') out.push({ id: s.id, name: s.name });
  return out;
}

/** The projects' books on this device. */
export class BookStore {
  private readonly kv: KeyValue;
  private readonly now: () => number;

  constructor(kv: KeyValue, now: () => number = Date.now) {
    this.kv = kv;
    this.now = now;
  }

  async load(key: string): Promise<ReadBook> {
    return readStoredBook(await this.kv.get(BOOKS, key), key);
  }

  /**
   * Writes a project's book. Whatever is stored under its key that this code
   * cannot read is first kept aside (`keptKey`), so a newer version's book is
   * never written over by an older one.
   */
  async save(key: string, book: unknown): Promise<StoredBook> {
    const stored = await this.kv.get(BOOKS, key);
    if (readStoredBook(stored, key).status === 'unreadable') await this.kv.put(BOOKS, keptKey(key, this.now()), stored);
    const record: StoredBook = { format: BOOK_FORMAT, version: BOOK_VERSION, key, saved: this.now(), book };
    await this.kv.put(BOOKS, key, record);
    return record;
  }

  /**
   * Moves a book to a project's new key (a drawing saved for the first time
   * gets its lasting name): nothing moves when the new key already has a
   * book of its own, which is kept, and the answer says so.
   */
  async move(from: string, to: string): Promise<'moved' | 'kept' | 'none'> {
    const read = await this.load(from);
    if (read.status !== 'ok') return 'none';
    if ((await this.load(to)).status !== 'none') return 'kept';
    await this.save(to, read.record.book);
    await this.kv.delete(BOOKS, from);
    return 'moved';
  }

  /** Copies a book to another key that has none (the old key keeps its own); 'kept' when the new key has one. */
  async copy(from: string, to: string): Promise<'copied' | 'kept' | 'none'> {
    const read = await this.load(from);
    if (read.status !== 'ok') return 'none';
    if ((await this.load(to)).status !== 'none') return 'kept';
    await this.save(to, read.record.book);
    return 'copied';
  }

  remove(key: string): Promise<void> {
    return this.kv.delete(BOOKS, key);
  }
}

// ── Pictures' bytes ─────────────────────────────────────────────────────

export const ASSET_FORMAT = 'kentos.sheets.asset';
export const ASSET_VERSION = 1;

/** A picture's bytes as this device keeps them, under their SHA-256 (design §3.4: the book holds only the metadata). */
export interface StoredAsset {
  readonly format: typeof ASSET_FORMAT;
  readonly version: typeof ASSET_VERSION;
  readonly sha256: string;
  readonly meta: AssetMeta;
  readonly bytes: Uint8Array;
}

/** The SHA-256 of bytes, 64 lowercase hex digits (the key the engine names a picture by). */
export async function sha256Hex(bytes: Uint8Array): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes as Uint8Array<ArrayBuffer>));
  return [...digest].map((b) => b.toString(16).padStart(2, '0')).join('');
}

/** Bytes as base64 (RFC 4648, with padding): how a template and a `.kpafta` file carry a picture. */
export function toBase64(bytes: Uint8Array): string {
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}

export function fromBase64(text: string): Uint8Array {
  const s = atob(text);
  const out = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i);
  return out;
}

/**
 * The pictures' bytes on this device. Bytes are written only under their own
 * digest (checked here), so a book that names a picture gets those bytes or
 * none; one already kept is not written again.
 */
export class AssetStore {
  private readonly kv: KeyValue;
  private readonly cache = new Map<string, StoredAsset>();

  constructor(kv: KeyValue) {
    this.kv = kv;
  }

  async put(meta: AssetMeta, bytes: Uint8Array): Promise<void> {
    const sha = await sha256Hex(bytes);
    if (sha !== meta.sha256) throw new Error(`“${meta.name}” resminin baytları adıyla uyuşmuyor (SHA-256 farklı); dosya bozulmuş olabilir.`);
    if (this.cache.has(sha) || (await this.kv.get(ASSETS, sha)) !== null) return;
    const record: StoredAsset = { format: ASSET_FORMAT, version: ASSET_VERSION, sha256: sha, meta, bytes };
    await this.kv.put(ASSETS, sha, record);
    this.cache.set(sha, record);
  }

  /** A picture's bytes, or null when this device does not have them (the preflight says `missing_asset`). */
  async get(sha: string): Promise<StoredAsset | null> {
    const hit = this.cache.get(sha);
    if (hit) return hit;
    const raw = await this.kv.get(ASSETS, sha);
    if (!isObj(raw) || raw.format !== ASSET_FORMAT || raw.sha256 !== sha || !(raw.bytes instanceof Uint8Array) || !isObj(raw.meta)) return null;
    const record = raw as unknown as StoredAsset;
    this.cache.set(sha, record);
    return record;
  }

  /** The digests this device has bytes for. */
  async digests(): Promise<string[]> {
    return (await this.kv.keys(ASSETS)).filter((k) => /^[0-9a-f]{64}$/.test(k));
  }
}
