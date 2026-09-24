import { Signal } from '../core/signal';
import { DOCUMENT_EXTENSION, readSnapshot, toSnapshot } from '../model/snapshot';
import { h } from '../ui/dom';
import { Dialog } from '../ui/widgets/Dialog';
import { projectStylesProblem } from './cloud/incoming';
import type { AppContext } from './context';
import type { DocumentContent } from '../model/document';

/**
 * Local drawing files (`.kcad`, the versioned `DocumentSnapshotV1`): Save,
 * Save as, Open. A save counts only when the file was really written: the
 * drawing turns clean after the writer closes without error, and only for
 * the revision that was written (an edit made meanwhile stays unsaved;
 * CLAUDE.md §13.1, §21.3). Where the browser cannot write files, the drawing
 * is offered as a download and stays marked unsaved, since nothing confirms
 * it was kept. Cloud save arrives with the server (Faz B).
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

const DRAWING: FileKind = { description: 'KentOS çizimi', accept: { 'application/json': [DOCUMENT_EXTENSION] } };

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
      if (this.ctx.doc.dirty.value && !(await this.confirmDiscard())) return false;
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
      let text: string;
      try {
        text = await (await handle.getFile()).text();
      } catch (e) {
        this.ctx.log.error(`“${handle.name}” okunamadı: ${message(e)}.`);
        return false;
      }
      return this.load(text, handle, readOnly);
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

  /** Reads a drawing's text into the app; `handle` becomes the file Save writes to unless `readOnly`. */
  load(text: string, handle: DrawingFileHandle | null, readOnly = false): boolean {
    const { ctx } = this;
    const read = readSnapshot(text);
    if (!read.ok) {
      ctx.log.error(`${handle ? `“${handle.name}”` : 'Dosya'} açılamadı: ${read.error}`);
      return false;
    }
    // The project's own symbols are checked like any shared style file (untrusted data).
    const styles = projectStylesProblem(read.content.styles);
    if (styles) {
      ctx.log.error(`${handle ? `“${handle.name}”` : 'Dosya'} açılamadı: ${styles}.`);
      return false;
    }
    // A local file replaces an open cloud project (its unsent changes stay in the device draft).
    ctx.cloud.detach();
    replaceDrawing(ctx, read.content);
    this.handle = readOnly ? null : handle;
    ctx.log.success(`“${handle?.name ?? ctx.doc.name.value}” açıldı: ${ctx.doc.size} nesne, ${ctx.doc.layers.leaves().length} katman.`);
    return true;
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

  private async writeTo(handle: DrawingFileHandle): Promise<boolean> {
    const doc = this.ctx.doc;
    const revision = doc.revision;
    const text = `${JSON.stringify(toSnapshot(doc))}\n`;
    try {
      const w = await handle.createWritable();
      await w.write(text);
      await w.close();
    } catch (e) {
      this.ctx.log.error(`“${handle.name}” yazılamadı: ${message(e)}. Değişiklikler kaydedilmemiş sayılıyor; başka bir yere kaydetmeyi deneyin (Farklı kaydet).`);
      return false;
    }
    doc.markSaved(revision);
    if (doc.dirty.value) this.ctx.log.warn(`“${handle.name}” kaydedildi; kayıt sürerken yapılan değişiklikler henüz kaydedilmedi.`);
    else this.ctx.log.success(`“${handle.name}” kaydedildi.`);
    return true;
  }

  /** Without file access the drawing is offered as a download; nothing confirms it was kept. */
  private download(): boolean {
    const doc = this.ctx.doc;
    const name = withExtension(doc.name.value);
    const url = URL.createObjectURL(new Blob([`${JSON.stringify(toSnapshot(doc))}\n`], { type: 'application/json' }));
    const a = h('a', { href: url, download: name });
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    this.ctx.log.warn(`Bu tarayıcı dosyaya doğrudan yazamıyor; “${name}” indirme olarak verildi. Kaydedildiği doğrulanamadığı için çizim kaydedilmemiş sayılıyor.`);
    return false;
  }

  /** Unsaved changes: save them, drop them, or stay. Resolves true when the caller may go on. */
  private confirmDiscard(): Promise<boolean> {
    return new Promise((resolve) => {
      let answered = false;
      const done = (v: boolean) => {
        if (answered) return;
        answered = true;
        dialog.close();
        resolve(v);
      };
      const save = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet ve devam et');
      const drop = h('button', { class: 'btn', type: 'button' }, 'Kaydetmeden devam et');
      const stay = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
      const dialog = new Dialog({
        title: 'Kaydedilmemiş değişiklikler',
        width: 460,
        content: [h('p', null, `“${this.ctx.doc.name.value}” içinde kaydedilmemiş değişiklikler var. Başka bir çizim açılırsa bu değişiklikler kaybolur.`)],
        footer: [h('div', { class: 'dialog__foot-spacer' }), stay, drop, save],
        onClose: () => done(false),
      });
      stay.addEventListener('click', () => done(false));
      drop.addEventListener('click', () => done(true));
      save.addEventListener('click', () => {
        answered = true;
        dialog.close();
        // Saving is part of this command: the busy flag is already held.
        void (this.handle ? this.writeTo(this.handle) : this.chooseAndWrite()).then(resolve);
      });
    });
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
    this.handle = handle;
    // The drawing takes the file's name (before writing, so the file holds it).
    const name = withoutExtension(handle.name);
    if (name && doc.name.value !== name) doc.name.set(name);
    return this.writeTo(handle);
  }
}

/** Where the browser has no open dialog API: a hidden file input. */
function pickWithInput(accept = `${DOCUMENT_EXTENSION},application/json`): Promise<DrawingFileHandle | null> {
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
