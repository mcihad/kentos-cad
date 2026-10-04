import type { DatumTransform } from '../contracts/generated/DatumTransform';
import { fixed } from '../core/displayNumber';
import { Signal } from '../core/signal';
import { forgetGrid, loadGrid, type GridError, type GridInfo } from '../model/geom/crsGrid';

/**
 * The device's NTv2 grid library (docs/adr/0168 §4; the desktop's `apps/desktop/src/grids.rs`): grids kept on this
 * device under their SHA-256, in IndexedDB `kentos.grids` (what each header says in `grids`, the bytes apart in
 * `bytes`), as QGIS and ArcGIS keep theirs in the user's folder. A project names its grid choice's file by its SHA-256
 * and size. The grids the open project names are read into the core (`loadGrid`) when it opens and when its choices
 * change; one this device does not have is said once, with the file's name, and the transforms give no value by it
 * (`noGrid`) until it is added.
 */

/** A grid of the library: what its header says, kept when it is added so the list needs no grid read whole. */
export interface GridEntry {
  /** Its SHA-256, 64 lower-case hex digits. */
  readonly id: string;
  /** The name of the file it was added from. */
  readonly file: string;
  readonly size: number;
  /** The datums it shifts from and to, as its header names them. */
  readonly from: string;
  readonly to: string;
  readonly subgrids: number;
  /** West, south, east, north (degrees). */
  readonly extent: readonly [number, number, number, number];
}

/** Where the library keeps its grids: IndexedDB in the app, memory in tests. */
export interface GridStore {
  list(): Promise<GridEntry[]>;
  bytes(id: string): Promise<Uint8Array | null>;
  put(entry: GridEntry, bytes: Uint8Array): Promise<void>;
  delete(id: string): Promise<void>;
}

/** The largest grid file read (the core's limit). */
const MAX_BYTES = 256 * 1024 * 1024;

/** Why a file is not read as NTv2, in the user's words (the desktop's `grid_error_text`). */
export function gridErrorText(e: GridError): string {
  switch (e) {
    case 'tooLarge':
      return "256 MiB'tan büyük";
    case 'notNtv2':
      return 'NTv2 dosyası değil';
    case 'notSeconds':
      return 'kaymaları saniye cinsinden değil';
    case 'header':
      return 'başlığı bozuk';
    case 'extent':
      return 'kapsamı ya da aralığı geçersiz';
    case 'count':
      return 'kayıt sayısı kapsamıyla tutmuyor';
    case 'truncated':
      return 'dosya kayıtları bitmeden kesiliyor';
    case 'values':
      return 'sonlu olmayan kayma değerleri var';
  }
}

/** A grid's line under its name (the desktop's `grid_line`): the datums, the extent, the subgrids when more than one, the size, the SHA-256's start. */
export function gridLine(e: GridEntry): string {
  const [w, s, east, n] = e.extent;
  const parts = [`${e.from} → ${e.to}`, `${fixed(w, 1)}°–${fixed(east, 1)}° D, ${fixed(s, 1)}°–${fixed(n, 1)}° K`];
  if (e.subgrids > 1) parts.push(`${e.subgrids} alt ızgara`);
  parts.push(e.size < 1024 * 1024 ? `${fixed(e.size / 1024, 0)} KB` : `${fixed(e.size / (1024 * 1024), 1)} MB`);
  parts.push(e.id.slice(0, 12));
  return parts.join(' · ');
}

/** SHA-256 of `bytes` as 64 lower-case hex digits. */
async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes as Uint8Array<ArrayBuffer>));
  return [...digest].map((b) => b.toString(16).padStart(2, '0')).join('');
}

/** What the project's choices are told when a grid is missing, unreadable or changed: what, then how to put it right. */
const missingText = (why: string) => `${why} Proje ayarları › Koordinat sistemi › Izgaralar'dan ekleyin; o zamana dek datum dönüşümü bu seçimle değer vermez.`;

export class GridLibrary {
  /** The library's grids, by the names of the files they came from (after `refresh`). */
  readonly entries = new Signal<readonly GridEntry[]>([]);
  /** Bumped when a grid is read into the core or let go: what shows values by the grids takes them again. */
  readonly revision = new Signal(0);
  private readonly store: GridStore;
  /** The grids the open project named that were looked for in this page. */
  private readonly asked = new Set<string>();

  constructor(store: GridStore) {
    this.store = store;
  }

  /** The list read again from the store. */
  async refresh(): Promise<void> {
    const list = await this.store.list();
    this.entries.set([...list].sort((a, b) => a.file.localeCompare(b.file, 'tr') || a.id.localeCompare(b.id)));
  }

  /** Adds a file: read whole, checked as NTv2, kept under its SHA-256 with what its header says, read into the core; or why not. */
  async add(file: string, bytes: Uint8Array): Promise<GridEntry | { readonly error: string }> {
    if (bytes.length > MAX_BYTES) return { error: `“${file}” eklenmedi: ${gridErrorText('tooLarge')}.` };
    const id = await sha256(bytes);
    const info = loadGrid(id, bytes);
    if ('error' in info) return { error: `“${file}” NTv2 ızgarası olarak okunamadı: ${gridErrorText(info.error)}.` };
    const entry = entryOf(id, file, bytes.length, info);
    try {
      await this.store.put(entry, bytes);
    } catch (e) {
      return { error: `“${file}” ızgara kitaplığına yazılamadı: ${e instanceof Error ? e.message : String(e)}.` };
    }
    this.revision.update((v) => v + 1);
    await this.refresh();
    return entry;
  }

  /** Takes the grid out of the library and the core; a project that names it is told again. */
  async remove(id: string): Promise<void> {
    forgetGrid(id);
    await this.store.delete(id);
    this.asked.delete(id);
    this.revision.update((v) => v + 1);
    await this.refresh();
  }

  /**
   * Reads the grids the project's datum choices name into the core, once each; one this device does not have, has
   * changed or cannot read is said with its file's name.
   */
  async follow(choices: readonly DatumTransform[], say: (text: string) => void): Promise<void> {
    for (const t of choices) {
      const g = t.grid;
      if (!g || this.asked.has(g.id)) continue;
      this.asked.add(g.id);
      const bytes = await this.store.bytes(g.id);
      if (!bytes) {
        say(missingText(`NTv2 ızgarası bu cihazda yok: ${g.file}.`));
        continue;
      }
      if ((await sha256(bytes)) !== g.id) {
        say(missingText(`NTv2 ızgarası ${g.file} cihazdaki kitaplıkta değişmiş (SHA-256'sı tutmuyor).`));
        continue;
      }
      const info = loadGrid(g.id, bytes);
      if ('error' in info) {
        say(missingText(`NTv2 ızgarası ${g.file} okunamadı: ${gridErrorText(info.error)}.`));
        continue;
      }
      this.revision.update((v) => v + 1);
    }
  }
}

const entryOf = (id: string, file: string, size: number, info: GridInfo): GridEntry => ({
  id,
  file,
  size,
  from: info.from,
  to: info.to,
  subgrids: info.subgrids,
  extent: info.extent,
});

/** The library in memory: tests, and a browser without IndexedDB (nothing is kept past the page). */
export function memoryGridStore(): GridStore {
  const items = new Map<string, { entry: GridEntry; bytes: Uint8Array }>();
  return {
    list: async () => [...items.values()].map((i) => i.entry),
    bytes: async (id) => items.get(id)?.bytes ?? null,
    put: async (entry, bytes) => void items.set(entry.id, { entry, bytes }),
    delete: async (id) => void items.delete(id),
  };
}

const DB = 'kentos.grids';
/** What each grid's header says; the bytes are apart, in `BYTES`, so the list reads none. */
const STORE = 'grids';
const BYTES = 'bytes';

function request<T>(r: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error ?? new Error('IndexedDB isteği başarısız'));
  });
}

function done(t: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    t.oncomplete = () => resolve();
    t.onerror = () => reject(t.error ?? new Error('IndexedDB yazılamadı'));
    t.onabort = () => reject(t.error ?? new Error('IndexedDB yazımı durdu'));
  });
}

/** The device's library in IndexedDB `kentos.grids`: the entries in `grids`, the bytes in `bytes`; memory where the browser has none. */
export function indexedGridStore(): GridStore {
  if (typeof indexedDB === 'undefined') return memoryGridStore();
  let db: Promise<IDBDatabase> | null = null;
  const open = () =>
    (db ??= new Promise((resolve, reject) => {
      const r = indexedDB.open(DB, 1);
      r.onupgradeneeded = () => {
        r.result.createObjectStore(STORE, { keyPath: 'id' });
        r.result.createObjectStore(BYTES, { keyPath: 'id' });
      };
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error ?? new Error('Izgara kitaplığı açılamadı'));
      r.onblocked = () => reject(new Error('Izgara kitaplığı başka bir sekmede açık'));
    }));
  return {
    list: async () => (await request((await open()).transaction(STORE, 'readonly').objectStore(STORE).getAll())) as GridEntry[],
    bytes: async (id) => {
      const row = (await request((await open()).transaction(BYTES, 'readonly').objectStore(BYTES).get(id))) as { bytes: ArrayBuffer } | undefined;
      return row ? new Uint8Array(row.bytes) : null;
    },
    put: async (entry, bytes) => {
      const t = (await open()).transaction([STORE, BYTES], 'readwrite');
      t.objectStore(BYTES).put({ id: entry.id, bytes: bytes.slice().buffer });
      t.objectStore(STORE).put(entry);
      await done(t);
    },
    delete: async (id) => {
      const t = (await open()).transaction([STORE, BYTES], 'readwrite');
      t.objectStore(STORE).delete(id);
      t.objectStore(BYTES).delete(id);
      await done(t);
    },
  };
}
