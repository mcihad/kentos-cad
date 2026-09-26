import type { V1Identities } from '../contracts/generated/V1Identities';
import { Signal } from '../core/signal';
import { crsBySrid } from '../geo/crs';
import { sheetAround } from '../model/newProject';
import { DOCUMENT_EXTENSION, DOCUMENT_MIME, snapshotHead } from '../model/snapshot';
import type { OpeningView } from '../ui/io/OpeningDialog';
import { h } from '../ui/dom';
import { askUnsaved } from '../ui/widgets/confirm';
import type { AppContext } from './context';
import type { DocumentContent } from '../model/document';
import { describeDropped, readDrawing, yieldToPage, type DrawingCodec, type ReadDrawing, type ReadProgress } from './drawingFile';
import { readFile, tooLarge, writeAccess, writeFailure, writeFile } from './fileAccess';
import { RecentFiles, type RecentFile } from './recentFiles';

/**
 * Local drawing files (`.kcad`): Save, Save as, Open. A drawing is saved as
 * the binary `.kcad` v2 (docs/specs/kcad-v2.md, docs/adr/0025), made and read
 * back in the formats worker before a byte is written; a file opens by what
 * it holds (v2, or the old v1 JSON), never by its name.
 *
 * A save counts only when the file was really written: the drawing turns
 * clean after the writer closes without error, and only for the revision
 * that was written (an edit made meanwhile stays unsaved; CLAUDE.md §13.1,
 * §21.3). The drawing is packed into typed columns in one turn, so the file
 * holds the drawing of that moment (docs/adr/0030). A write that fails
 * (permission taken back, disk full, the tab closed) leaves the previous
 * file as it was: the browser writes to a copy and puts it in place only
 * when the writer closes (app/fileAccess.ts). Where the browser cannot write
 * files, the drawing is offered as a download and stays marked unsaved,
 * since nothing confirms it was kept. A drawing opened from a v1 file is not
 * written back there: Save asks where to write the v2 file, and only the
 * user's own choice replaces the old one (FILE-21). Yeni proje replaces the
 * drawing with an empty one. Whatever replaces the drawing asks first about
 * unsaved local changes; an open cloud project needs no question, since its
 * changes are sent or kept in the device draft (it is left first,
 * `CloudSession.leave`).
 *
 * Opening is staged and can be stopped (TODOS.md FILE-20, docs/adr/0030): a
 * modal window shows the file, the project once it is known and how far the
 * reading and the checks are; Vazgeç stops it. The drawing on screen is
 * replaced in one step, and only by a drawing read and checked whole, only
 * if the open was not stopped or overtaken and the drawing did not change
 * meanwhile (generation and revision, CLAUDE.md §21.2).
 *
 * Objects keep their persistent ids (docs/adr/0014): a v2 file holds them; a
 * v1 file's are derived from its content, the same every time, here and in
 * the desktop app, and a v2 save records where they came from.
 */

/** A file the app can read and write: a File System Access handle, or an in-memory one in tests. */
export interface DrawingFileHandle {
  readonly name: string;
  getFile(): Promise<Blob>;
  createWritable(): Promise<{ write(data: string | Uint8Array): Promise<void>; close(): Promise<void>; abort?(): Promise<void> }>;
}

/**
 * A kind of file for the file dialogs: a description and the accepted
 * extensions by media type. Without one the dialogs ask for a drawing (.kcad).
 */
export interface FileKind {
  description: string;
  accept: Record<string, string[]>;
}

/** Asks for a file: a handle, null when the user cancels, undefined when the browser cannot. */
export interface DrawingFilePicker {
  save(suggestedName: string, kind?: FileKind): Promise<DrawingFileHandle | null | undefined>;
  open(kind?: FileKind): Promise<DrawingFileHandle | null | undefined>;
}

type FsaWindow = Window & {
  showSaveFilePicker?: (o: unknown) => Promise<DrawingFileHandle>;
  showOpenFilePicker?: (o: unknown) => Promise<DrawingFileHandle[]>;
};

// v1 and v2 files alike: no registered KCAD media type exists (FILE-13).
const DRAWING: FileKind = { description: 'KentOS çizimi', accept: { [DOCUMENT_MIME]: [DOCUMENT_EXTENSION] } };

/** The browser's own file dialogs (File System Access API), where available. */
export const browserPicker: DrawingFilePicker = {
  async save(suggestedName, kind = DRAWING) {
    const w = window as FsaWindow;
    if (!w.showSaveFilePicker) return undefined;
    try {
      return await w.showSaveFilePicker({ suggestedName, types: [kind] });
    } catch (e) {
      if ((e as DOMException).name === 'AbortError') return null;
      throw e;
    }
  },
  async open(kind = DRAWING) {
    const w = window as FsaWindow;
    if (!w.showOpenFilePicker) return undefined;
    try {
      const [handle] = await w.showOpenFilePicker({ types: [kind], multiple: false });
      return handle ?? null;
    } catch (e) {
      if ((e as DOMException).name === 'AbortError') return null;
      throw e;
    }
  },
};

/** What the user chose about unsaved changes. */
export type DiscardChoice = 'save' | 'drop' | 'stay';

/** A file picked for an import: its name and bytes. */
export interface PickedFile {
  name: string;
  bytes: Uint8Array;
}

/** Puts a whole drawing on screen: select tool, no selection, the new content, its start view. */
export function replaceDrawing(ctx: AppContext, content: DocumentContent): void {
  ctx.tools.activate('select');
  ctx.selection.clear();
  ctx.doc.replaceWith(content);
  const home = ctx.doc.homeView;
  if (home) ctx.view.camera.fit(home);
  else ctx.view.zoomExtents();
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
const withoutExtension = (name: string) => (name.toLowerCase().endsWith(DOCUMENT_EXTENSION) ? name.slice(0, -DOCUMENT_EXTENSION.length) : name);
const withExtension = (name: string) => `${withoutExtension(name)}${DOCUMENT_EXTENSION}`;
const count = (n: number) => n.toLocaleString('tr-TR');
const share = (done: number, total: number) => (total > 0 ? Math.min(1, done / total) : 1);

/** After the next frame is drawn (at once where there are no frames: tests). */
const nextFrame = () => (typeof requestAnimationFrame === 'function' ? new Promise<void>((r) => requestAnimationFrame(() => requestAnimationFrame(() => r()))) : Promise.resolve());

/** An open's window where there is none (tests, a window that did not load): the open goes on without it. */
const unseen: OpeningView = { step: () => {}, project: () => {}, close: () => {} };

/** Where an open's bytes come from, and what it becomes once on screen. */
interface Source {
  /** The file's name, or what the bytes are (a recovery copy). */
  label: string;
  handle: DrawingFileHandle | null;
  /** The bytes, when they are in hand already (`load`, a recovery copy). */
  bytes?: Uint8Array;
  /** Save does not write back to it (a read-only file, a v1 file, a recovery copy). */
  readOnly: boolean;
  /** Leaves an open cloud project before the drawing is replaced (else only detaches from it). */
  leaveCloud: boolean;
  /** The file vanished while it was being read (the recent list forgets it). */
  gone?: () => void;
  /** Puts the drawing on screen once read; the default is `show`. */
  put?: (read: Extract<ReadDrawing, { ok: true }>) => void;
}

export class DocumentFiles {
  /** The file the drawing was opened from or last saved to; Save writes here without asking. */
  handle: DrawingFileHandle | null = null;
  picker: DrawingFilePicker = browserPicker;
  /** A save or open is in progress (commands stay disabled meanwhile). */
  readonly busy = new Signal(false);
  /** Drawings opened or saved lately (Son dosyalar): the start screen and the application menu list them. */
  readonly recent = new RecentFiles();
  /** Asks about unsaved changes; `after` says what would lose them. A dialog in the app; tests answer themselves. */
  ask: (name: string, after: string) => Promise<DiscardChoice> = askAboutUnsaved;
  /**
   * The persistent ids of a v1 drawing's objects, from its text: the formats
   * worker, whose client loads with the first file opened, as the import and
   * export windows load it (CLAUDE.md §20); tests run the module in process.
   */
  identities: (text: string) => Promise<V1Identities> = async (text) => (await import('../io/client')).formats().v1Identities(text);
  /**
   * The `.kcad` v2 codec: the formats worker, loaded like `identities`; tests
   * run the module in process. `cancel` ends the worker (an open stopped).
   */
  kcad: () => Promise<DrawingCodec> = async () => {
    const f = (await import('../io/client')).formats();
    return { encode: (drawing, progress) => f.encodeKcad(drawing, progress), decode: (bytes, progress) => f.decodeKcad(bytes, progress), cancel: () => f.cancel() };
  };
  /** An open's window (loaded with the first open); tests have none. `cancel` is Vazgeç. */
  opening: (name: string, cancel: () => void) => Promise<OpeningView> = async (name, cancel) => {
    if (typeof document === 'undefined') return unseen;
    try {
      return (await import('../ui/io/OpeningDialog')).openOpeningDialog(name, cancel);
    } catch {
      return unseen;
    }
  };
  /** The user chose to drop the drawing's unsaved changes (the recovery copy goes too, app/recovery.ts). */
  discarded: () => void = () => {};
  private readonly ctx: AppContext;
  /** Counts opens: a later one overtakes an earlier that is still running. */
  private opens = 0;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  /** Saves to the current file, or asks where the first time. True when the file was written. */
  save(): Promise<boolean> {
    return this.handle ? this.run(() => this.writeTo(this.handle!)) : this.saveAs();
  }

  /** Asks where to save, then writes. True when the file was written. */
  saveAs(): Promise<boolean> {
    return this.run(() => this.chooseAndWrite());
  }

  /** Opens a drawing, asking first about unsaved changes. True when one was opened. */
  open(): Promise<boolean> {
    return this.run(async () => {
      if (this.mustAsk() && !(await this.confirmDiscard('Başka bir çizim açılırsa bu değişiklikler kaybolur.'))) return false;
      let handle: DrawingFileHandle | null | undefined;
      try {
        handle = await this.picker.open();
      } catch (e) {
        this.ctx.log.error(`Açma penceresi açılamadı: ${message(e)}. Tarayıcının dosya iznini denetleyin.`);
        return false;
      }
      // Without an open dialog API a hidden input reads the file; it cannot be written back.
      const readOnly = handle === undefined;
      if (readOnly) handle = await pickWithInput();
      if (!handle) return false;
      // A local file replaces an open cloud project: what waits is sent, the rest stays in the device draft.
      return this.openStaged({ label: handle.name, handle, readOnly, leaveCloud: true });
    });
  }

  /**
   * Opens a drawing from the recent list: asks about unsaved changes, then
   * for the browser's permission to the file again (it forgets it between
   * visits). A file that is gone leaves the list. True when it was opened.
   */
  openRecent(entry: RecentFile): Promise<boolean> {
    return this.run(async () => {
      if (this.mustAsk() && !(await this.confirmDiscard('Başka bir çizim açılırsa bu değişiklikler kaybolur.'))) return false;
      const handle = entry.handle as DrawingFileHandle & {
        queryPermission?(o: { mode: string }): Promise<PermissionState>;
        requestPermission?(o: { mode: string }): Promise<PermissionState>;
      };
      try {
        if (handle.queryPermission && (await handle.queryPermission({ mode: 'readwrite' })) !== 'granted') {
          const answer = await handle.requestPermission?.({ mode: 'readwrite' });
          if (answer !== 'granted') {
            this.ctx.log.warn(`“${entry.name}” için dosya izni verilmedi; çizim açılmadı.`);
            return false;
          }
        }
      } catch (e) {
        this.ctx.log.error(`“${entry.name}” için izin istenemedi: ${message(e)}. Dosyayı Aç ile seçin.`);
        return false;
      }
      return this.openStaged({ label: entry.name, handle, readOnly: false, leaveCloud: true, gone: () => void this.recent.remove(entry.id) });
    });
  }

  /**
   * Asks for a file to import (not the drawing's own file) and reads its
   * bytes; null when the user cancels or it cannot be read (said in the log).
   * Call it straight from the command: the dialog needs the user's click.
   */
  async pickForImport(kind: FileKind): Promise<PickedFile | null> {
    let handle: DrawingFileHandle | null | undefined;
    try {
      handle = await this.picker.open(kind);
    } catch (e) {
      this.ctx.log.error(`Açma penceresi açılamadı: ${message(e)}. Tarayıcının dosya iznini denetleyin.`);
      return null;
    }
    if (handle === undefined) handle = await pickWithInput(Object.values(kind.accept).flat().join(','));
    if (!handle) return null;
    try {
      return { name: handle.name, bytes: new Uint8Array(await (await handle.getFile()).arrayBuffer()) };
    } catch (e) {
      this.ctx.log.error(`“${handle.name}” okunamadı: ${message(e)}.`);
      return null;
    }
  }

  /**
   * Reads a drawing (a file's bytes, or v1 text) into the app; `handle`
   * becomes the file Save writes to unless `readOnly` or the file is v1.
   */
  load(data: string | Uint8Array, handle: DrawingFileHandle | null, readOnly = false): Promise<boolean> {
    return this.run(() =>
      // Nothing is waited for here: an open cloud project's unsent changes stay in its device draft.
      this.openStaged({ label: handle?.name ?? 'Dosya', handle, bytes: typeof data === 'string' ? new TextEncoder().encode(data) : data, readOnly, leaveCloud: false }),
    );
  }

  /**
   * Puts a recovery copy of unsaved work on screen (app/recovery.ts): read
   * and checked as a file is, with its persistent ids; it stays unsaved and
   * has no file, so Save asks where and no file is replaced by itself.
   */
  recover(bytes: Uint8Array, name: string): Promise<boolean> {
    return this.run(async () => {
      if (this.mustAsk() && !(await this.confirmDiscard('Kurtarma kopyası açılırsa bu değişiklikler kaybolur.'))) return false;
      return this.openStaged({
        label: `“${name}” kurtarma kopyası`,
        handle: null,
        bytes,
        readOnly: true,
        leaveCloud: false,
        put: (read) => {
          const { ctx } = this;
          replaceDrawing(ctx, read.content);
          this.handle = null;
          ctx.doc.markUnsaved();
          ctx.log.success(
            `“${ctx.doc.name.value}” kaydedilmemiş çalışması geri yüklendi: ${count(ctx.doc.size)} nesne. Çizim kaydedilmemiş sayılıyor; Kaydet dosyanın yerini sorar, hiçbir dosyanın üzerine kendiliğinden yazılmaz.`,
          );
        },
      });
    });
  }

  /**
   * Replaces the drawing with a new, empty project (Dosya → Yeni proje).
   * Unsaved local changes are asked about first; an open cloud project is
   * left. The new drawing has no file yet, so the next Save asks where.
   * True when it is on screen.
   */
  newProject(content: DocumentContent): Promise<boolean> {
    return this.run(async () => {
      const { ctx } = this;
      if (ctx.doc.busy) {
        ctx.log.warn('Bir işlem sürerken yeni proje açılamaz; işlem bitince yeniden deneyin.');
        return false;
      }
      if (this.mustAsk() && !(await this.confirmDiscard('Yeni proje açılırsa bu değişiklikler kaybolur.'))) return false;
      await this.leaveCloud();
      replaceDrawing(ctx, content);
      this.handle = null;
      const crs = crsBySrid(content.settings.srid);
      // An empty drawing has no extent: it opens on one sheet around its origin.
      if (!content.entities.length) ctx.view.camera.fit(sheetAround(content.origin, content.settings.plotScale, crs?.unit));
      const system = crs ? `${crs.name} (EPSG:${crs.srid})` : `EPSG:${content.settings.srid}`;
      ctx.log.success(`“${ctx.doc.name.value}” yeni projesi açıldı: ${system}, 1:${content.settings.plotScale}. İlk kayıtta dosyanın yeri sorulur.`);
      return true;
    });
  }

  /**
   * Opens a drawing in stages behind the open's window: the file's bytes,
   * then the drawing read and checked whole (app/drawingFile.ts), then the
   * drawing replaced in one step. A stopped or overtaken open replaces
   * nothing; nor does one during which another drawing was put on screen or
   * the drawing changed in a way nothing keeps. True when opened.
   */
  private async openStaged(src: Source): Promise<boolean> {
    const { ctx } = this;
    const open = ++this.opens;
    // The drawing this open may replace, as it is now (CLAUDE.md §21.2): another drawing put on
    // screen meanwhile makes this open give way, and so does a change nothing keeps (a processing
    // result on a local drawing); a cloud project's own changes stay in the project.
    const revision = ctx.doc.revision;
    let replaced = false;
    const offReset = ctx.doc.events.on('reset', () => (replaced = true));
    let stopped = false;
    // Once the drawing is being put on screen, Vazgeç comes too late.
    let committed = false;
    const stale = () => stopped || open !== this.opens;
    let codec: DrawingCodec | null = null;
    const view = await this.opening(src.label, () => {
      if (committed) return;
      stopped = true;
      codec?.cancel?.();
    }).catch((e: unknown) => {
      offReset();
      throw e;
    });
    const said = (p: ReadProgress) => this.step(view, p);
    try {
      let bytes = src.bytes;
      if (!bytes) {
        try {
          const file = await src.handle!.getFile();
          const large = tooLarge(src.label, file.size);
          if (large) {
            ctx.log.error(large);
            return false;
          }
          bytes = (await readFile(file, (done, total) => view.step(`Dosya okunuyor: ${count(Math.round(done / 1e6))} / ${count(Math.round(total / 1e6))} MB`, 0.1 * share(done, total)), stale)) ?? undefined;
        } catch (e) {
          const gone = (e as DOMException).name === 'NotFoundError';
          if (gone) src.gone?.();
          ctx.log.error(gone ? `“${src.label}” artık bulunamıyor (taşınmış ya da silinmiş)${src.gone ? '; son dosyalardan kaldırıldı' : ''}.` : `“${src.label}” okunamadı: ${message(e)}.`);
          return false;
        }
      }
      if (!bytes || stale()) return this.stopped(src.label);
      codec = await this.kcad();
      const got = codec;
      const read = await readDrawing(bytes, { codec: async () => got, identities: this.identities }, { progress: said, stale });
      if (!read.ok) {
        if (read.cancelled || stale()) return this.stopped(src.label);
        ctx.log.error(`${src.handle ? `“${src.label}”` : src.label} açılamadı: ${read.error}`);
        return false;
      }
      if (stale()) return this.stopped(src.label);
      view.step('Çizim ekrana getiriliyor…', 0.97);
      // The window shows the last stage before the page is busy with the drawing.
      await yieldToPage();
      if (stale()) return this.stopped(src.label);
      if (replaced || (ctx.doc.revision !== revision && this.mustAsk())) {
        ctx.log.warn(`${src.handle ? `“${src.label}”` : src.label} açılmadı: açılış sürerken ekrandaki çizim değişti ya da başka bir çizim açıldı; o çizim olduğu gibi duruyor. Dosyayı yeniden açın.`);
        return false;
      }
      // From here the open goes through: the cloud project is left only now, so an open that gives way
      // above leaves it attached.
      committed = true;
      if (read.warning) ctx.log.warn(`${src.handle ? `“${src.label}”` : src.label}: ${read.warning}`);
      if (src.leaveCloud) await this.leaveCloud();
      else ctx.cloud.detach();
      try {
        (src.put ?? ((r) => this.show(r, src.handle, src.readOnly)))(read);
      } catch (e) {
        // The document refuses a drawing it cannot hold before it changes anything (`replaceWith`).
        ctx.log.error(`${src.handle ? `“${src.label}”` : src.label} açılamadı: ${message(e)} Ekrandaki çizim olduğu gibi duruyor.`);
        return false;
      }
      // The window stays until the drawing's first frame is drawn: a large one takes a moment.
      await nextFrame();
      return true;
    } finally {
      offReset();
      view.close();
    }
  }

  /** An open that was stopped: said once; the drawing on screen is as it was. */
  private stopped(label: string): false {
    this.ctx.log.info(`${label.startsWith('“') ? label : `“${label}”`} açılışı durduruldu; ekrandaki çizim olduğu gibi duruyor.`);
    return false;
  }

  /** A stage of an open in the open's window. */
  private step(view: OpeningView, p: ReadProgress): void {
    switch (p.stage) {
      case 'checking':
        view.step(`Dosya denetleniyor (bütünlük özeti): %${Math.round(100 * share(p.done, p.total))}`, 0.1 + 0.1 * share(p.done, p.total));
        break;
      case 'project':
        view.project(`“${p.name}”: ${count(p.objects)} nesne, ${count(p.layers)} üst katman`);
        break;
      case 'reading':
        view.step(`Nesneler okunuyor: ${count(p.done)} / ${count(p.total)}`, 0.2 + 0.4 * share(p.done, p.total));
        break;
      case 'objects':
        view.step(`Nesneler denetleniyor: ${count(p.done)} / ${count(p.total)}`, 0.6 + 0.35 * share(p.done, p.total));
        break;
      default:
        break;
    }
  }

  /** Puts a drawing read from `handle` on screen; Save writes back there unless it was read-only or v1. */
  private show(read: Extract<ReadDrawing, { ok: true }>, handle: DrawingFileHandle | null, readOnly: boolean): void {
    const { ctx } = this;
    replaceDrawing(ctx, read.content);
    const v1 = read.format === 'v1';
    this.handle = readOnly || v1 ? null : handle;
    if (handle && !readOnly) this.remember(handle);
    ctx.log.success(`“${handle?.name ?? ctx.doc.name.value}” açıldı: ${ctx.doc.size} nesne, ${ctx.doc.layers.leaves().length} katman.`);
    if (v1 && handle)
      ctx.log.info('Dosya eski biçimde (KCAD v1). Kaydet, yeni biçimde (v2) yazmak için yer sorar; eski dosyanın üzerine kendiliğinden yazmaz.');
  }

  /** Puts the file first in the recent list, with what it holds now. */
  private remember(handle: DrawingFileHandle): void {
    const doc = this.ctx.doc;
    void this.recent.add(handle, `${doc.size.toLocaleString('tr-TR')} nesne · ${doc.crs.value.name}`);
  }

  /** Unsaved changes that replacing the drawing would lose: local ones, or edits a cloud project does not keep (a viewer's). */
  private mustAsk(): boolean {
    return this.ctx.doc.dirty.value && !this.ctx.cloud.autosaves();
  }

  /** Leaves an open cloud project before the drawing is replaced, and says what stayed on this device. */
  private async leaveCloud(): Promise<void> {
    const project = this.ctx.cloud.project.value;
    if (!project) return;
    const unsent = await this.ctx.cloud.leave();
    if (unsent) this.ctx.log.info(`“${project.name}” bulut projesi kapatıldı. Gönderilemeyen ${unsent} değişiklik bu cihazda saklanıyor; proje yeniden açılınca geri gelir.`);
  }

  private async run(task: () => Promise<boolean>): Promise<boolean> {
    if (this.busy.value) return false;
    this.busy.set(true);
    try {
      return await task();
    } finally {
      this.busy.set(false);
    }
  }

  /**
   * The drawing's `.kcad` v2 bytes and its revision, taken in one turn (the
   * drawing is packed into typed columns before anything else runs, so the
   * file holds the drawing of that moment), checked in the worker; null
   * (said) when the drawing cannot be written, and then nothing is.
   */
  private async encode(target: string): Promise<{ bytes: Uint8Array<ArrayBuffer>; revision: number; dropped: string | null } | null> {
    const doc = this.ctx.doc;
    try {
      const [codec, { packDrawing }] = await Promise.all([this.kcad(), import('../io/columns')]);
      const revision = doc.revision;
      const { drawing, dropped } = packDrawing(snapshotHead(doc), doc.all());
      const bytes = await codec.encode(drawing);
      return { bytes, revision, dropped: describeDropped(dropped) };
    } catch (e) {
      this.ctx.log.error(`${target} yazılamadı: ${message(e)} Değişiklikler kaydedilmemiş sayılıyor.`);
      return null;
    }
  }

  private async writeTo(handle: DrawingFileHandle): Promise<boolean> {
    const doc = this.ctx.doc;
    // Asked while the user's Ctrl+S or click still counts, before the drawing is encoded.
    const denied = await writeAccess(handle);
    if (denied) {
      this.ctx.log.error(`${denied} Değişiklikler kaydedilmemiş sayılıyor.`);
      return false;
    }
    const encoded = await this.encode(`“${handle.name}”`);
    if (!encoded) return false;
    try {
      await writeFile(handle, encoded.bytes);
    } catch (e) {
      this.ctx.log.error(`${writeFailure(handle.name, e)} Değişiklikler kaydedilmemiş sayılıyor.`);
      return false;
    }
    doc.markSaved(encoded.revision);
    this.remember(handle);
    if (encoded.dropped) this.ctx.log.warn(`“${handle.name}”: KCAD v2'nin tanımadığı alanlar yazılmadı: ${encoded.dropped}.`);
    if (doc.dirty.value) this.ctx.log.warn(`“${handle.name}” kaydedildi; kayıt sürerken yapılan değişiklikler henüz kaydedilmedi.`);
    else this.ctx.log.success(`“${handle.name}” kaydedildi.`);
    return true;
  }

  /** Without file access the drawing is offered as a download; nothing confirms it was kept. */
  private async download(): Promise<boolean> {
    const doc = this.ctx.doc;
    const name = withExtension(doc.name.value);
    const encoded = await this.encode(`“${name}”`);
    if (!encoded) return false;
    const url = URL.createObjectURL(new Blob([encoded.bytes], { type: DOCUMENT_MIME }));
    const a = h('a', { href: url, download: name });
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    if (encoded.dropped) this.ctx.log.warn(`“${name}”: KCAD v2'nin tanımadığı alanlar yazılmadı: ${encoded.dropped}.`);
    this.ctx.log.warn(`Bu tarayıcı dosyaya doğrudan yazamıyor; “${name}” indirme olarak verildi. Kaydedildiği doğrulanamadığı için çizim kaydedilmemiş sayılıyor.`);
    return false;
  }

  /** Unsaved changes: save them, drop them, or stay. Resolves true when the caller may go on. */
  private async confirmDiscard(after: string): Promise<boolean> {
    const choice = await this.ask(this.ctx.doc.name.value, after);
    if (choice === 'drop') this.discarded();
    if (choice !== 'save') return choice === 'drop';
    // Saving is part of this command: the busy flag is already held.
    return this.handle ? this.writeTo(this.handle) : this.chooseAndWrite();
  }

  /** Save as, inside a command that already holds the busy flag. */
  private async chooseAndWrite(): Promise<boolean> {
    const doc = this.ctx.doc;
    let handle: DrawingFileHandle | null | undefined;
    try {
      handle = await this.picker.save(withExtension(doc.name.value));
    } catch (e) {
      this.ctx.log.error(`Kaydetme penceresi açılamadı: ${message(e)}. Tarayıcının dosya iznini denetleyin.`);
      return false;
    }
    if (handle === undefined) return this.download();
    if (!handle) return false;
    // A deleted cloud project's drawing, or one whose access was taken away, becomes this file: it leaves
    // the project first (nothing more goes there; its unsent edits stay in the device draft).
    const cloud = this.ctx.cloud;
    const state = cloud.sync.value?.state.value;
    if (cloud.project.value && (state === 'deleted' || state === 'revoked')) cloud.detach();
    this.handle = handle;
    // The drawing takes the file's name (before writing, so the file holds it).
    const name = withoutExtension(handle.name);
    if (name && doc.name.value !== name) doc.name.set(name);
    return this.writeTo(handle);
  }
}

/**
 * The question about unsaved changes (the app's confirm window). It stacks
 * over a dialog that asked for the replacement (Yeni proje), so staying goes
 * back to that dialog.
 */
async function askAboutUnsaved(name: string, after: string): Promise<DiscardChoice> {
  const a = await askUnsaved({ name, after, verb: 'devam et' });
  return a === 'discard' ? 'drop' : a;
}

/** Where the browser has no open dialog API: a hidden file input. */
function pickWithInput(accept = DOCUMENT_EXTENSION): Promise<DrawingFileHandle | null> {
  return new Promise((resolve) => {
    const input = h('input', { type: 'file', accept, style: 'display:none' });
    input.addEventListener('change', () => {
      const file = input.files?.[0];
      input.remove();
      if (!file) return resolve(null);
      resolve({
        name: file.name,
        getFile: async () => file,
        createWritable: async () => {
          throw new Error('bu tarayıcıda dosyaya yazılamıyor');
        },
      });
    });
    input.addEventListener('cancel', () => (input.remove(), resolve(null)));
    document.body.append(input);
    input.click();
  });
}
