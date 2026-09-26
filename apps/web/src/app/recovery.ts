import { uuidv7 } from '../core/uuid';
import { snapshotHead } from '../model/snapshot';
import { confirmDialog } from '../ui/widgets/confirm';
import type { AppContext } from './context';

/**
 * Local recovery copies of unsaved work (TODOS.md FILE-19, docs/adr/0030):
 * while a local drawing has changes no save wrote, a copy of it is kept on
 * this device, apart from any `.kcad` file, so a crash, a closed tab or a
 * lost laptop battery does not lose the work. The copy is the drawing's KCAD
 * v2 bytes (the save's own verified codec) with a few facts about it, in
 * IndexedDB `kentos.recovery/copies`, not in the cloud drafts' store and
 * never in the user's file.
 *
 * - Written a few seconds after the drawing changes (and at most every half
 *   minute while it keeps changing) and when the tab is hidden; never while
 *   an open runs and never for an open cloud project (its device draft keeps
 *   its changes, app/cloud/drafts.ts). While a save writes its file the copy
 *   is the save's own verified bytes, taken when the tab is hidden (the tab
 *   may be closed or discarded before the save ends).
 * - Removed when the drawing is saved (clean again) or its changes are
 *   dropped on purpose (Kaydetmeden devam et). A drawing replaced without
 *   that question (a cloud project opened over it) leaves its copy, to be
 *   offered later: unsaved work is never deleted by itself.
 * - Offered when the app starts: a copy whose tab is gone (the tab holds a
 *   Web Lock named after it while it lives) is shown in a question: Geri
 *   yükle puts it on screen unsaved and without a file (Save asks where; no
 *   file is ever written over by itself), Sonra keeps it for the next start,
 *   Sil deletes it.
 */

/** A copy as it is stored. */
export interface RecoveryCopy {
  /** Its key: the tab and the drawing it holds (a drawing replaced gets a new key). */
  id: string;
  /** The tab that wrote it; while that tab lives it holds the Web Lock `kentos.recovery/<tab>`. */
  tab: string;
  name: string;
  /** The file the drawing was opened from or last saved to (its name), if any. */
  file: string | null;
  /** When it was written (ms since the epoch). */
  savedAt: number;
  objects: number;
  /** The drawing: KCAD v2, as a save writes it. */
  bytes: Uint8Array;
  /** The copy's own format. */
  version: 1;
}

/** Where copies are kept: IndexedDB in the app, memory in tests. */
export interface RecoveryStore {
  list(): Promise<unknown[]>;
  put(copy: RecoveryCopy): Promise<void>;
  delete(id: string): Promise<void>;
}

/** A stored value read back: checked, not trusted (another version, a broken store). */
export function readCopy(v: unknown): RecoveryCopy | null {
  if (typeof v !== 'object' || v === null) return null;
  const c = v as Record<string, unknown>;
  const ok =
    c.version === 1 &&
    typeof c.id === 'string' &&
    typeof c.tab === 'string' &&
    typeof c.name === 'string' &&
    (c.file === null || typeof c.file === 'string') &&
    typeof c.savedAt === 'number' &&
    typeof c.objects === 'number' &&
    c.bytes instanceof Uint8Array;
  return ok ? (c as unknown as RecoveryCopy) : null;
}

const DB = 'kentos.recovery';
const STORE = 'copies';

function request<T>(r: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error ?? new Error('IndexedDB isteği başarısız'));
  });
}

/** The browser's store: IndexedDB `kentos.recovery/copies`. */
export function indexedDbStore(): RecoveryStore {
  let db: Promise<IDBDatabase> | null = null;
  const open = () =>
    (db ??= new Promise((resolve, reject) => {
      const r = indexedDB.open(DB, 1);
      r.onupgradeneeded = () => r.result.createObjectStore(STORE, { keyPath: 'id' });
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error ?? new Error('kurtarma deposu açılamadı'));
      r.onblocked = () => reject(new Error('kurtarma deposu başka bir sekmede kilitli'));
    }));
  const write = async (fn: (s: IDBObjectStore) => void) => {
    const t = (await open()).transaction(STORE, 'readwrite');
    fn(t.objectStore(STORE));
    await new Promise<void>((resolve, reject) => {
      t.oncomplete = () => resolve();
      t.onerror = () => reject(t.error);
      t.onabort = () => reject(t.error);
    });
  };
  return {
    list: async () => request((await open()).transaction(STORE, 'readonly').objectStore(STORE).getAll()),
    put: (copy) => write((s) => s.put(copy)),
    delete: (id) => write((s) => s.delete(id)),
  };
}

type Locks = {
  request(name: string, fn: () => Promise<void>): Promise<void>;
  query(): Promise<{ held?: { name?: string }[] }>;
};
const locks = (): Locks | null => (typeof navigator !== 'undefined' ? ((navigator as unknown as { locks?: Locks }).locks ?? null) : null);
const LOCK = 'kentos.recovery/';

/** The tabs alive now (the locks they hold), or null when the browser cannot tell. */
export async function liveTabs(): Promise<ReadonlySet<string> | null> {
  const l = locks();
  if (!l) return null;
  const { held = [] } = await l.query();
  return new Set(held.map((h) => h.name ?? '').filter((n) => n.startsWith(LOCK)).map((n) => n.slice(LOCK.length)));
}

/** What the copies need of the app (the whole context in the app; parts in tests). */
export type RecoveryContext = Pick<AppContext, 'doc' | 'log'> & {
  cloud: Pick<AppContext['cloud'], 'project'>;
  files: Pick<AppContext['files'], 'busy' | 'handle' | 'kcad' | 'recover' | 'writing'>;
};

/** What the offer asks: restore the copy, keep it for later, delete it. */
export type RecoveryAnswer = 'restore' | 'later' | 'delete';

export interface RecoveryOptions {
  store?: RecoveryStore;
  /** The tabs alive now; `liveTabs` in the app. */
  live?: () => Promise<ReadonlySet<string> | null>;
  /** Asks about one copy; the app's question window. */
  ask?: (copy: RecoveryCopy) => Promise<RecoveryAnswer>;
  now?: () => number;
  /** A copy is written this long after the last change… */
  quietMs?: number;
  /** …and at most this long after the first change it has not written. */
  maxMs?: number;
}

/** Quiet time before a copy is written, and the longest wait while changes go on. */
const QUIET_MS = 3000;
const MAX_MS = 30_000;

export class RecoveryCopies {
  /** This tab (its Web Lock is held while it lives). */
  readonly tab = uuidv7();
  private readonly ctx: RecoveryContext;
  private readonly store: RecoveryStore;
  private readonly live: () => Promise<ReadonlySet<string> | null>;
  private readonly ask: (copy: RecoveryCopy) => Promise<RecoveryAnswer>;
  private readonly now: () => number;
  private readonly quietMs: number;
  private readonly maxMs: number;
  /** The drawing on screen: a new one (replaced) gets a new id, so its copy never takes the old one's place. */
  private drawing = uuidv7();
  /** The revision the drawing's copy holds, or null when it has none. */
  private written: number | null = null;
  /** When the first change the copy does not hold was made. */
  private firstUnwritten: number | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private writing: Promise<void> | null = null;
  private failed = false;
  private readonly off: (() => void)[] = [];

  constructor(ctx: RecoveryContext, o: RecoveryOptions = {}) {
    this.ctx = ctx;
    this.store = o.store ?? indexedDbStore();
    this.live = o.live ?? liveTabs;
    this.ask = o.ask ?? askAboutCopy;
    this.now = o.now ?? Date.now;
    this.quietMs = o.quietMs ?? QUIET_MS;
    this.maxMs = o.maxMs ?? MAX_MS;
  }

  /** The id the copy of the drawing on screen has (tests). */
  get current(): string {
    return `${this.tab}/${this.drawing}`;
  }

  /** Starts keeping copies: holds this tab's lock and follows the drawing. */
  start(): void {
    // Held until the tab goes: a copy whose lock nobody holds is one to offer.
    void locks()?.request(`${LOCK}${this.tab}`, () => new Promise<void>(() => {}));
    const doc = this.ctx.doc;
    this.off.push(doc.events.on('changed', () => this.changed()));
    this.off.push(doc.events.on('reset', () => this.replaced()));
    this.off.push(doc.dirty.subscribe((dirty) => (dirty ? this.changed() : this.saved())));
    if (typeof document !== 'undefined') {
      const hidden = () => document.visibilityState === 'hidden' && void this.flush();
      document.addEventListener('visibilitychange', hidden);
      this.off.push(() => document.removeEventListener('visibilitychange', hidden));
    }
  }

  dispose(): void {
    for (const f of this.off.splice(0)) f();
    if (this.timer) clearTimeout(this.timer);
  }

  /** The user dropped the drawing's unsaved changes on purpose: its copy goes. */
  discard(): void {
    this.cancelTimer();
    const id = this.current;
    this.drawing = uuidv7();
    this.written = null;
    this.firstUnwritten = null;
    void this.store.delete(id).catch(() => {});
  }

  /** Writes the copy now if the drawing has changes it does not hold (the tab is being hidden). */
  async flush(): Promise<void> {
    this.cancelTimer();
    await this.write();
  }

  /** The drawing changed: a copy is written once it rests (or after the longest wait). */
  private changed(): void {
    const doc = this.ctx.doc;
    if (!doc.dirty.value) return;
    const now = this.now();
    this.firstUnwritten ??= now;
    const at = Math.min(now + this.quietMs, this.firstUnwritten + this.maxMs);
    this.cancelTimer();
    this.timer = setTimeout(() => void this.write(), Math.max(0, at - now));
  }

  /** A save wrote the drawing: its copy goes. */
  private saved(): void {
    this.cancelTimer();
    this.firstUnwritten = null;
    if (this.written === null) return;
    this.written = null;
    void this.store.delete(this.current).catch(() => {});
  }

  /** Another drawing is on screen: its copies get their own id; the old copy stays unless dropped. */
  private replaced(): void {
    this.cancelTimer();
    this.drawing = uuidv7();
    this.written = null;
    this.firstUnwritten = null;
  }

  private cancelTimer(): void {
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
  }

  /** Writes the drawing's copy, unless it has none to keep or one is being written. */
  private write(): Promise<void> {
    this.writing ??= this.writeOnce().finally(() => (this.writing = null));
    return this.writing;
  }

  private async writeOnce(): Promise<void> {
    const { doc, cloud, files } = this.ctx;
    if (!doc.dirty.value) return this.saved();
    // An open cloud project keeps its changes in its device draft.
    if (cloud.project.value) return;
    const revision = doc.revision;
    if (revision === this.written) return;
    // A save writing the drawing of this moment: its bytes are the copy. Any other busy moment waits.
    const saving = files.writing?.revision === revision ? files.writing.bytes : null;
    if (files.busy.value && !saving) return this.changed();
    const id = this.current;
    try {
      const objects = doc.size;
      const name = doc.name.value;
      let bytes = saving;
      if (!bytes) {
        const [codec, { packDrawing }] = await Promise.all([files.kcad(), import('../io/columns')]);
        // The drawing of this moment (one turn), as a save takes it.
        if (doc.revision !== revision || this.current !== id) return this.changed();
        bytes = await codec.encode(packDrawing(snapshotHead(doc), doc.all()).drawing);
      }
      // The drawing was replaced, dropped or saved meanwhile: this copy is no longer its.
      if (this.current !== id || !doc.dirty.value) return;
      await this.store.put({ id, tab: this.tab, name, file: files.handle?.name ?? null, savedAt: this.now(), objects, bytes, version: 1 });
      // Saved, dropped or replaced while the copy was being stored: it goes again.
      if (this.current !== id || !doc.dirty.value) return void (await this.store.delete(id).catch(() => {}));
      this.written = revision;
      if (doc.revision === revision) this.firstUnwritten = null;
      else this.changed();
      this.failed = false;
    } catch (e) {
      // The worker was ended under the copy (Vazgeç of an open ends it, io/client.ts): nothing failed,
      // and the copy is written again once the drawing rests.
      if ((e as { code?: unknown } | null)?.code === 'cancelled') return this.changed();
      // Said once: the drawing itself is untouched, only the safety copy is missing.
      if (!this.failed)
        this.ctx.log.warn(
          `Kaydedilmemiş çalışmanın kurtarma kopyası yazılamadı (${e instanceof Error ? e.message : String(e)}). Çizim etkilenmedi; çalışmanızı kaydetmeyi unutmayın.`,
        );
      this.failed = true;
    }
  }

  /**
   * Offers the copies of tabs that are gone (a crash, a closed tab), newest
   * first, one question each. True when one was restored.
   */
  async offer(): Promise<boolean> {
    let copies: RecoveryCopy[];
    let alive: ReadonlySet<string> | null;
    try {
      copies = (await this.store.list()).map(readCopy).filter((c): c is RecoveryCopy => !!c);
      alive = await this.live();
    } catch {
      return false;
    }
    // Without Web Locks no tab can be told alive: only this tab's own copies are left out then.
    const orphans = copies.filter((c) => c.tab !== this.tab && !alive?.has(c.tab)).sort((a, b) => b.savedAt - a.savedAt);
    for (const copy of orphans) {
      const answer = await this.ask(copy);
      if (answer === 'later') continue;
      if (answer === 'delete') {
        await this.store.delete(copy.id).catch(() => {});
        this.ctx.log.info(`“${copy.name}” çiziminin kurtarma kopyası silindi.`);
        continue;
      }
      if (await this.ctx.files.recover(copy.bytes, copy.name)) {
        // The drawing on screen is this copy now, unsaved: it gets its own copy at once.
        await this.store.delete(copy.id).catch(() => {});
        await this.flush();
        return true;
      }
    }
    return false;
  }
}

/** The question about one copy (the app's confirm window). */
function askAboutCopy(copy: RecoveryCopy): Promise<RecoveryAnswer> {
  const when = new Date(copy.savedAt).toLocaleString('tr-TR', { dateStyle: 'medium', timeStyle: 'short' });
  return confirmDialog<RecoveryAnswer>({
    title: 'Kaydedilmemiş çalışma bulundu',
    message: `“${copy.name}” çiziminin kaydedilmemiş bir kopyası var: ${when}, ${copy.objects.toLocaleString('tr-TR')} nesne${copy.file ? `, dosyası “${copy.file}”` : ''}. KentOS beklenmedik biçimde kapanmış ya da sekme kaydedilmeden kapatılmış olabilir.`,
    details: [
      'Geri yükle: kopya açılır ve kaydedilmemiş sayılır; Kaydet dosyanın yerini sorar, hiçbir dosyanın üzerine kendiliğinden yazılmaz.',
      'Sonra: kopya bu cihazda kalır; KentOS bir sonraki açılışta yeniden sorar.',
      'Sil: kopya kalıcı olarak silinir.',
    ],
    answers: [
      { value: 'delete', label: 'Sil', kind: 'danger', aside: true },
      { value: 'later', label: 'Sonra' },
      { value: 'restore', label: 'Geri yükle', kind: 'primary' },
    ],
    cancel: 'later',
  });
}
