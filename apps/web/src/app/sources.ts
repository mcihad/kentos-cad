import { Signal } from '../core/signal';
import type { ProjectSummary } from '../contracts/generated/ProjectSummary';
import type { Json } from '../model/exchange';
import { sourceListing, type SourceEntry, type SourceFile } from '../model/sources';
import { COORD_FILES } from '../ui/io/coordFiles';
import type { AppContext } from './context';
import { verifyDownload } from './cloud/transfer';
import { otherDrawing, otherSystem } from './drawingExchange';
import { DXF_FILES, GEOJSON_FILES, GNSS_FILES, NCZ_FILES, SHAPEFILE_FILES } from './fileExchange';
import type { PickedFile } from './fileIO';

/**
 * Kaynaklar's sources on the page (docs/adr/0199 §7; the desktop's `sources/`): the folders the user added, kept as
 * the browser's folder handles in IndexedDB (`kentos.sources` / `folders`; the browser asks again for leave to read
 * one after a reload), their files handed to their format's import window; and the KentOS projects the account
 * reaches, a project's drawing downloaded (a database project's snapshot, a file project's newest revision) to take
 * one of its layers with its objects (app/drawingExchange.ts `takeLayerInto`).
 */

/** What the panel needs of a folder the browser gave (File System Access). */
export interface FolderHandle {
  readonly kind: 'directory';
  readonly name: string;
  values(): AsyncIterable<FolderHandle | FileHandle>;
  getDirectoryHandle(name: string): Promise<FolderHandle>;
  getFileHandle(name: string): Promise<FileHandle>;
  queryPermission?(o: { mode: 'read' }): Promise<PermissionState>;
  requestPermission?(o: { mode: 'read' }): Promise<PermissionState>;
}

export interface FileHandle {
  readonly kind: 'file';
  readonly name: string;
  getFile(): Promise<File>;
}

/** A folder of the list: its handle, its name, when it was added. */
export interface SourceFolder {
  readonly id: string;
  readonly name: string;
  readonly at: number;
  readonly handle: FolderHandle;
}

const DB = 'kentos.sources';
const STORE = 'folders';

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

function request<T>(r: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error ?? new Error('IndexedDB isteği başarısız'));
  });
}

/** The folders added, oldest first; a handle the browser cannot store stays for this session only. */
export class SourceFolders {
  readonly list = new Signal<readonly SourceFolder[]>([]);
  readonly ready: Promise<void>;
  private db: Promise<IDBDatabase | null> | null = null;

  constructor() {
    this.ready = this.load();
  }

  /** Adds a folder (one already in the list is not added again); its entry. */
  async add(handle: FolderHandle): Promise<SourceFolder> {
    await this.ready;
    for (const f of this.list.value) if (await sameEntry(f.handle, handle)) return f;
    const entry: SourceFolder = { id: `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`, name: handle.name, at: Date.now(), handle };
    this.list.set([...this.list.value, entry]);
    await this.write((s) => s.put(entry));
    return entry;
  }

  async remove(id: string): Promise<void> {
    this.list.set(this.list.value.filter((f) => f.id !== id));
    await this.write((s) => s.delete(id));
  }

  private async load(): Promise<void> {
    const db = await this.open();
    if (!db) return;
    try {
      const all = (await request(db.transaction(STORE, 'readonly').objectStore(STORE).getAll())) as SourceFolder[];
      const kept = all.filter((f) => f && typeof f.id === 'string' && typeof f.name === 'string' && f.handle && typeof f.handle.values === 'function');
      kept.sort((a, b) => a.at - b.at);
      this.list.set([...kept, ...this.list.value.filter((f) => !kept.some((k) => k.id === f.id))]);
    } catch {
      /* unreadable storage: the list starts empty */
    }
  }

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
      /* a handle the browser cannot store, quota: this session's list still has it */
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

async function sameEntry(a: FolderHandle, b: FolderHandle): Promise<boolean> {
  if (a === b) return true;
  const same = (a as FolderHandle & { isSameEntry?(o: unknown): Promise<boolean> }).isSameEntry;
  if (!same) return false;
  try {
    return await same.call(a, b);
  } catch {
    return false;
  }
}

/** Whether this browser lets the page read a folder the user picks. */
export const foldersSupported = (): boolean => typeof (window as { showDirectoryPicker?: unknown }).showDirectoryPicker === 'function';

/** Asks for a folder (the user's gesture opens the browser's window); null when cancelled or refused. */
export async function pickFolder(ctx: AppContext): Promise<FolderHandle | null> {
  const w = window as { showDirectoryPicker?: (o: unknown) => Promise<FolderHandle> };
  if (!w.showDirectoryPicker) {
    ctx.log.warn('Bu tarayıcı klasör seçtirmiyor; klasörler Chrome ya da Edge’de eklenir. Dosyaları Dosya › İçe aktar ile alın.');
    return null;
  }
  try {
    return await w.showDirectoryPicker({ id: 'kentos-kaynaklar', mode: 'read' });
  } catch (e) {
    if ((e as { name?: string }).name !== 'AbortError') ctx.log.error(`Klasör seçilemedi: ${message(e)}. Tarayıcının dosya iznini denetleyin.`);
    return null;
  }
}

/** Whether the page may read the folder; with `ask`, the browser asks the user (a click's gesture needed). */
export async function mayRead(handle: FolderHandle, ask: boolean): Promise<boolean> {
  try {
    if (!handle.queryPermission) return true;
    if ((await handle.queryPermission({ mode: 'read' })) === 'granted') return true;
    return ask && !!handle.requestPermission && (await handle.requestPermission({ mode: 'read' })) === 'granted';
  } catch {
    return false;
  }
}

/** What a folder shows (model/sources.ts); its entries read through the handle. */
export async function folderListing(handle: FolderHandle): Promise<ReturnType<typeof sourceListing>> {
  const entries: SourceEntry[] = [];
  for await (const e of handle.values()) entries.push({ name: e.name, dir: e.kind === 'directory' });
  return sourceListing(entries);
}

/** A file's parts read from its folder. */
async function readParts(folder: FolderHandle, file: SourceFile): Promise<PickedFile[]> {
  const out: PickedFile[] = [];
  for (const name of file.parts) {
    const f = await (await folder.getFileHandle(name)).getFile();
    out.push({ name, bytes: new Uint8Array(await f.arrayBuffer()) });
  }
  return out;
}

/** Katman olarak ekle on a file: its format's import window with it (the window asks what the import asks). */
export async function addFileAsLayer(ctx: AppContext, folder: FolderHandle, file: SourceFile): Promise<void> {
  if (!(await mayRead(folder, true))) {
    ctx.log.warn(`“${folder.name}” okunamıyor: tarayıcı klasörü okuma izni vermedi. Klasörü açıp izin verin.`);
    return;
  }
  let parts: PickedFile[];
  try {
    parts = await readParts(folder, file);
  } catch (e) {
    ctx.log.error(`“${file.name}” okunamadı: ${message(e)}. Dosya taşınmış ya da silinmiş olabilir; listeyi yenileyin.`);
    return;
  }
  const [first] = parts;
  const failed = (e: unknown) => ctx.log.error(`İçe aktarma penceresi yüklenemedi: ${message(e)}. Bağlantınızı denetleyip yeniden deneyin.`);
  switch (file.kind) {
    case 'dxf':
      return void import('../ui/io/DrawingImportDialog').then((m) => m.openDxfImport(ctx, first, DXF_FILES), failed);
    case 'ncz':
      return void import('../ui/io/DrawingImportDialog').then((m) => m.openNczImport(ctx, first, NCZ_FILES), failed);
    case 'geojson':
      return void import('../ui/io/GisImportDialog').then((m) => m.openGeoJsonImport(ctx, first, GEOJSON_FILES), failed);
    case 'shapefile':
      return void import('../ui/io/GisImportDialog').then((m) => m.openShapefileImport(ctx, parts, SHAPEFILE_FILES), failed);
    case 'gnss':
      return void import('../ui/io/GnssImportDialog').then((m) => m.openGnssImport(ctx, first, GNSS_FILES), failed);
    case 'coords':
      return void import('../ui/io/CoordImportDialog').then((m) => m.openCoordImport(ctx, first, COORD_FILES), failed);
  }
}

// ── KentOS projects ───────────────────────────────────────────────────

/** The lists of projects the panel shows. */
export const CLOUD_LISTS = [
  { view: 'mine', label: 'Projelerim' },
  { view: 'shared', label: 'Benimle paylaşılanlar' },
] as const;

/** Projects a list shows at most. */
export const CLOUD_LIMIT = 200;

/** A list's projects, and how many the list holds. */
export async function cloudProjects(ctx: AppContext, view: (typeof CLOUD_LISTS)[number]['view']): Promise<{ projects: ProjectSummary[]; total: number }> {
  const page = await ctx.cloud.api.catalog({ view, sort: 'name', limit: CLOUD_LIMIT });
  return { projects: page.projects, total: page.total };
}

/** A project's drawing as the contract's JSON (a database project's snapshot, a file project's newest revision). */
export async function cloudDrawing(ctx: AppContext, p: ProjectSummary, signal?: AbortSignal): Promise<Json> {
  const api = ctx.cloud.api;
  let download;
  if (p.storage === 'file') {
    const revs = await api.fileRevisions(p.tenantId, p.id, signal);
    if (!revs.current) throw new Error('Projenin henüz kaydedilmiş bir revizyonu yok.');
    const listed = revs.revisions.find((r) => r.revision === revs.current);
    download = await api.fileRevision(p.tenantId, p.id, revs.current, undefined, signal);
    await verifyDownload(download, listed?.sha256);
  } else {
    download = await api.snapshot(p.tenantId, p.id, undefined, signal);
    await verifyDownload(download);
  }
  const read = await otherDrawing(ctx, download.bytes);
  if (!read.ok) throw new Error(read.error);
  const system = otherSystem(ctx, read.json);
  if (system) ctx.log.warn(`“${p.name}”: ${system}`);
  return read.json;
}
