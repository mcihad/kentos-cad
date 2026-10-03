import { Signal } from '../../core/signal';
import { uuidv7 } from '../../core/uuid';
import { errorText, type BookText, type SheetEngine } from '../../product/sheet/engine';
import type { SheetHistory } from '../../product/sheet/history';
import type { SheetState } from '../../product/sheet/state';
import { peekSheets, type BookStore, type ReadBook } from '../../product/sheet/store';
import type { AppContext } from '../context';
import { keyChange, projectKeyOf, type ProjectKey } from './projectKey';

/**
 * A project's book on this device (docs/sheet/design.md §10): which project
 * is open (its key, projectKey.ts), its book read when it opens (by the
 * engine once it is there; until then its sheets' names only, for their
 * tabs), written back shortly after each change, moved or copied along
 * when the drawing gets a lasting name. A stored book the engine refuses is
 * kept as it is: nothing is written over it until a sheet is made anew.
 */

export const BOOK_TEXTS = {
  unreadable: (why: string) => `Bu projenin bu cihazdaki pafta kaydı okunamadı (${why}); kayıt olduğu gibi korunuyor.`,
  storeFailed: (why: string) => `Pafta kaydı okunamadı: ${why}. Sayfayı yenileyip yeniden deneyin.`,
  saveFailed: (why: string) => `Paftalar bu cihaza yazılamadı: ${why}. Tarayıcının depolama iznini ve boş yerini denetleyin.`,
  bookRefused: (why: string) => `Bu projenin bu cihazdaki paftaları açılamadı: ${why} Kayıt olduğu gibi korunuyor.`,
} as const;

/** How long after the last change the book is written (a drag's many frames are one write). */
const SAVE_MS = 400;

/** What the books reach of the sheet service. */
export interface BookHost {
  readonly state: SheetState;
  engine(): SheetEngine | null;
  history(): SheetHistory | null;
  /** An empty book (the engine's schema). */
  emptyBook(): BookText | null;
}

export class ProjectBooks {
  /** The project the book belongs to. */
  readonly key = new Signal<ProjectKey | null>(null);
  /** What this device keeps for the project, read when it was opened. */
  readonly stored = new Signal<ReadBook>({ status: 'none' });
  private readonly ctx: AppContext;
  private readonly store: BookStore;
  private readonly host: BookHost;
  /** A key for this session's unsaved drawing. */
  private session = uuidv7();
  private loads = 0;
  private saveTimer: ReturnType<typeof setTimeout> | undefined;
  /** The book written last, so a load or an undo back to it is not written again. */
  private savedText = '';

  constructor(ctx: AppContext, store: BookStore, host: BookHost) {
    this.ctx = ctx;
    this.store = store;
    this.host = host;
  }

  /**
   * Follows which project is open: the cloud project, the drawing's lasting
   * id, its file. A new drawing that gets a lasting name takes its book along;
   * another drawing opened loads its own (nothing of the last stays on screen).
   */
  watch(): () => void {
    const { ctx, host } = this;
    let replaced = true;
    const identity = () =>
      projectKeyOf({
        cloud: ctx.cloud.project.value ? { tenantId: ctx.cloud.project.value.tenantId, projectId: ctx.cloud.project.value.projectId, name: ctx.cloud.project.value.name } : null,
        projectId: ctx.doc.projectId,
        fileName: ctx.files.handle?.name ?? null,
        session: this.session,
      });
    const update = () => {
      const next = identity();
      const was = this.key.value;
      const change = keyChange(was, next, replaced);
      replaced = false;
      if (change === 'keep') return;
      this.key.set(next);
      if ((change === 'move' || change === 'copy') && was) void this.follow(change, was, next);
      else void this.load(next);
    };
    const offReset = ctx.doc.events.on('reset', () => {
      replaced = true;
      // The open sheet was the last drawing's: Model comes forward, the book goes until the new one is read.
      host.state.openSheet(null);
      this.flush();
      this.stored.set({ status: 'none' });
      host.state.waiting.set([]);
      const h = host.history();
      const empty = host.emptyBook();
      if (h && empty) {
        this.savedText = empty.text;
        h.reset(empty);
      }
      // After the task that put the drawing on screen: a cloud project names itself right after its drawing.
      setTimeout(update, 0);
    });
    const subs = [ctx.cloud.project.subscribe(() => queueMicrotask(update)), ctx.doc.name.subscribe(() => queueMicrotask(update))];
    update();
    return () => {
      offReset();
      subs.forEach((s) => s());
      this.flush();
    };
  }

  private async load(key: ProjectKey): Promise<void> {
    const n = ++this.loads;
    try {
      const read = await this.store.load(key.id);
      // A later project overtook this read: it is not this project's any more.
      if (n !== this.loads) return;
      this.stored.set(read);
      if (read.status === 'unreadable') this.ctx.log.warn(BOOK_TEXTS.unreadable(read.reason));
      if (read.status === 'ok') {
        if (this.host.engine()) this.openStored();
        else this.host.state.waiting.set(peekSheets(read.record.book));
      }
    } catch (e) {
      if (n === this.loads) this.ctx.log.warn(BOOK_TEXTS.storeFailed((e as Error).message));
    }
  }

  /** The stored book read by the engine and put in the history (once the engine is there). */
  openStored(): void {
    const engine = this.host.engine();
    const h = this.host.history();
    const read = this.stored.value;
    if (!engine || !h || read.status !== 'ok') return;
    this.host.state.waiting.set([]);
    try {
      const book = engine.readBook(JSON.stringify(read.record.book));
      this.savedText = book.text;
      h.reset(book);
    } catch (e) {
      // Kept as it is under its key: nothing is written over it until the user makes a sheet anew.
      this.stored.set({ status: 'unreadable', reason: errorText(e) });
      this.ctx.log.error(BOOK_TEXTS.bookRefused(errorText(e)));
    }
  }

  /** The drawing on screen got another key: its book goes along (a session's moves, a lasting name's is copied). */
  private async follow(how: 'move' | 'copy', from: ProjectKey, to: ProjectKey): Promise<void> {
    this.flush();
    try {
      const done = how === 'move' ? await this.store.move(from.id, to.id) : await this.store.copy(from.id, to.id);
      if (done === 'kept') this.ctx.log.warn(`${to.label} için bu cihazda zaten pafta kaydı var; o korundu, açık paftalar ${from.label} altında kaldı.`);
    } catch (e) {
      this.ctx.log.warn(BOOK_TEXTS.storeFailed((e as Error).message));
    }
  }

  /** The book changed: written shortly (not when it is the book written last). */
  changed(b: BookText): void {
    if (b.text === this.savedText) return;
    clearTimeout(this.saveTimer);
    this.saveTimer = setTimeout(() => this.flush(), SAVE_MS);
  }

  /** Writes the book now if it changed since it was last written. */
  flush(): void {
    clearTimeout(this.saveTimer);
    this.saveTimer = undefined;
    const b = this.host.history()?.book.value;
    const key = this.key.value;
    if (!b || !key || b.text === this.savedText) return;
    // A book that was there and could not be read stays as it is until a sheet is made anew (it is not overwritten by an empty one).
    if (this.stored.value.status === 'unreadable' && !b.book.sheets.length) return;
    this.savedText = b.text;
    this.store.save(key.id, b.book).then(
      (record) => this.stored.set({ status: 'ok', record }),
      (e: Error) => this.ctx.log.error(BOOK_TEXTS.saveFailed(e.message)),
    );
  }
}
