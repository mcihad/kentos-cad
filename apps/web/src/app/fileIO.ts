import type { V1Identities } from '../contracts/generated/V1Identities';
import { Signal } from '../core/signal';
import { crsBySrid } from '../geo/crs';
import { sheetAround } from '../model/newProject';
import { DOCUMENT_EXTENSION, DOCUMENT_MIME, toSnapshotV2 } from '../model/snapshot';
import { h } from '../ui/dom';
import { askUnsaved } from '../ui/widgets/confirm';
import type { AppContext } from './context';
import type { DocumentContent } from '../model/document';
import { describeDropped, readDrawing, type DrawingCodec, type ReadDrawing } from './drawingFile';
import { ENDED } from './cloud/syncCore';
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
 * §21.3). Where the browser cannot write files, the drawing is offered as a
 * download and stays marked unsaved, since nothing confirms it was kept. A
 * drawing opened from a v1 file is not written back there: Save asks where
 * to write the v2 file, and only the user's own choice replaces the old one
 * (FILE-21). Yeni proje replaces the drawing with an empty one. Whatever
 * replaces the drawing asks first about unsaved local changes; an open
 * cloud project needs no question, since its changes are sent or kept in
 * the device draft (it is left first, `CloudSession.leave`).
 *
 * Objects keep their persistent ids (docs/adr/0014): a v2 file holds them; a
 * v1 file's are derived from its content, the same every time, here and in
 * the desktop app, and a v2 save records where they came from.
 */

/** A file the app can read and write: a File System Access handle, or an in-memory one in tests. */
export interface DrawingFileHandle {
  readonly name: string;
  getFile(): Promise<Blob>;
  createWritable(): Promise<{ write(data: string | Uint8Array): Promise<void>; close(): Promise<void> }>;
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
   * run the module in process. Its `encode` copies the drawing before it returns.
   */
  kcad: () => Promise<DrawingCodec> = async () => {
    const f = (await import('../io/client')).formats();
    return { encode: (snapshot) => f.encodeKcad(snapshot), decode: (bytes) => f.decodeKcad(bytes) };
  };
  private readonly ctx: AppContext;

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
      let bytes: Uint8Array;
      try {
        bytes = new Uint8Array(await (await handle.getFile()).arrayBuffer());
      } catch (e) {
        this.ctx.log.error(`“${handle.name}” okunamadı: ${message(e)}.`);
        return false;
      }
      const read = await this.read(bytes, handle);
      if (!read) return false;
      // A local file replaces an open cloud project: what waits is sent, the rest stays in the device draft.
      await this.leaveCloud();
      this.show(read, handle, readOnly);
      return true;
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
      let bytes: Uint8Array;
      try {
        bytes = new Uint8Array(await (await handle.getFile()).arrayBuffer());
      } catch (e) {
        const gone = (e as DOMException).name === 'NotFoundError';
        if (gone) void this.recent.remove(entry.id);
        this.ctx.log.error(gone ? `“${entry.name}” artık bulunamıyor (taşınmış ya da silinmiş); son dosyalardan kaldırıldı.` : `“${entry.name}” okunamadı: ${message(e)}.`);
        return false;
      }
      const read = await this.read(bytes, handle);
      if (!read) return false;
      await this.leaveCloud();
      this.show(read, handle, false);
      return true;
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
    return this.run(async () => {
      const read = await this.read(typeof data === 'string' ? new TextEncoder().encode(data) : data, handle);
      if (!read) return false;
      // Nothing is waited for here: an open cloud project's unsent changes stay in its device draft.
      this.ctx.cloud.detach();
      this.show(read, handle, readOnly);
      return true;
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
   * The drawing in a file's bytes (v2 or v1, by content), checked like any
   * file someone sent, its objects with their persistent ids (ADR 0014);
   * null (and the reason, said) when unreadable. A v1 file's ids that cannot
   * be derived do not stop the drawing: its objects get new ones for this
   * opening, and the log says so.
   */
  private async read(bytes: Uint8Array, handle: DrawingFileHandle | null): Promise<Extract<ReadDrawing, { ok: true }> | null> {
    const where = handle ? `“${handle.name}”` : 'Dosya';
    const read = await readDrawing(bytes, { codec: this.kcad, identities: this.identities });
    if (!read.ok) {
      this.ctx.log.error(`${where} açılamadı: ${read.error}`);
      return null;
    }
    if (read.warning) this.ctx.log.warn(`${where}: ${read.warning}`);
    return read;
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
   * codec copies the drawing before it returns), checked in the worker; null
   * (said) when the drawing cannot be written, and then nothing is.
   */
  private async encode(target: string): Promise<{ bytes: Uint8Array<ArrayBuffer>; revision: number; dropped: string | null } | null> {
    const doc = this.ctx.doc;
    try {
      const codec = await this.kcad();
      const revision = doc.revision;
      const { bytes, dropped } = await codec.encode(toSnapshotV2(doc));
      return { bytes, revision, dropped: describeDropped(dropped) };
    } catch (e) {
      this.ctx.log.error(`${target} yazılamadı: ${message(e)} Değişiklikler kaydedilmemiş sayılıyor.`);
      return null;
    }
  }

  private async writeTo(handle: DrawingFileHandle): Promise<boolean> {
    const doc = this.ctx.doc;
    const encoded = await this.encode(`“${handle.name}”`);
    if (!encoded) return false;
    try {
      const w = await handle.createWritable();
      await w.write(encoded.bytes);
      await w.close();
    } catch (e) {
      this.ctx.log.error(`“${handle.name}” yazılamadı: ${message(e)}. Değişiklikler kaydedilmemiş sayılıyor; başka bir yere kaydetmeyi deneyin (Farklı kaydet).`);
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
    // The drawing of a cloud project that was deleted, archived or whose access was taken away becomes
    // this file: it leaves the project first (nothing more goes there; its unsent edits stay in the
    // device draft).
    const cloud = this.ctx.cloud;
    const state = cloud.sync.value?.state.value;
    if (cloud.project.value && state && ENDED.includes(state)) cloud.detach();
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
