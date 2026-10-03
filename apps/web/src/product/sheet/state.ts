import { Signal } from '../../core/signal';
import { EMPTY_BOOK, type BookView, type ItemView, type SheetView } from './view';

/**
 * The sheet mode's session state (docs/sheet/design.md §10, §11): the book of
 * the open project as the interface shows it, which tab is in front (Model or
 * a sheet), the items chosen on it, the paper tool in hand and whether the
 * engine that edits sheets is there. A sheet is a mode of the main window,
 * not a window of its own: Model is always the first tab and never closes.
 * Nothing here knows the DOM; the interface reads these signals and asks
 * through the methods. Edits to the book go through the engine (its
 * history, history.ts): this state only takes the book it gives back.
 */

/**
 * Whether the sheet engine (kentos-sheet over WASM) can be used, and why
 * not. It is fetched the first time it is needed (`idle` until then): a
 * sheet comes forward, the gallery opens, a sheet is made.
 */
export type EngineStatus =
  | { readonly state: 'idle' }
  | { readonly state: 'loading' }
  | { readonly state: 'ready' }
  | { readonly state: 'failed'; readonly reason: string };

export const ENGINE_TEXTS = {
  loading: 'Pafta motoru yükleniyor…',
  failed: (why: string) => `Pafta motoru yüklenemedi: ${why}. Bağlantıyı denetleyip sayfayı yenileyin.`,
  noSheet: 'Önce bir pafta açın: çizim alanının altındaki sekmelerden birine tıklayın.',
} as const;

/**
 * The tool in hand on the paper: Seç, El, or a tool of the mode's profile
 * that adds an item (`tool` is the profile's id, `preset` one of its ready
 * looks): a drag on the paper gives the new item its frame, a click a
 * frame of the tool's own size.
 */
export type PaperTool =
  | { readonly kind: 'select' }
  | { readonly kind: 'hand' }
  /** `item`: the kind it adds (the profile's `ToolInfo.item`). */
  | { readonly kind: 'add'; readonly tool: string; readonly preset?: string; readonly label: string; readonly item: string };

export const SELECT: PaperTool = { kind: 'select' };
export const HAND: PaperTool = { kind: 'hand' };

/** A sheet kept on this device whose book waits for the engine to be read: its tab shows its name. */
export interface WaitingSheet {
  readonly id: string;
  readonly name: string;
}

/** What Hizala lines the chosen items up with (design §4 `Align.to`). */
export type AlignTarget = 'selection' | 'page' | 'margins';

/** Which way a click changes the selection: anew, adds (Shift), or flips one (Ctrl). */
export type SelectMode = 'replace' | 'add' | 'toggle';

const sameSet = (a: ReadonlySet<string>, b: ReadonlySet<string>) => a.size === b.size && [...a].every((x) => b.has(x));

export class SheetState {
  /** The open project's sheets (an empty book until it has one or while the engine is away). */
  readonly book = new Signal<BookView>(EMPTY_BOOK);
  /** The tab in front: a sheet's id, or null for Model. */
  readonly open = new Signal<string | null>(null);
  /** Items chosen on the open sheet, by id (never a master page's). */
  readonly selection = new Signal<ReadonlySet<string>>(new Set(), sameSet);
  readonly tool = new Signal<PaperTool>(SELECT);
  /** What Hizala lines items up with; kept while the window is open. */
  readonly alignTo = new Signal<AlignTarget>('selection');
  readonly engine = new Signal<EngineStatus>({ state: 'idle' });
  /** The project's sheets kept on this device, by name, until the engine reads their book (their tabs open it). */
  readonly waiting = new Signal<readonly WaitingSheet[]>([]);

  /** The sheet in front; null while Model is. */
  get sheet(): SheetView | null {
    const id = this.open.value;
    return id === null ? null : (this.book.value.sheets.find((s) => s.id === id) ?? null);
  }

  /** The chosen items of the sheet in front, in its drawing order. */
  get chosen(): ItemView[] {
    const sheet = this.sheet;
    const ids = this.selection.value;
    return sheet ? sheet.items.filter((i) => ids.has(i.id)) : [];
  }

  /** Why the engine cannot do something now; null when it can (an engine not fetched yet is fetched when asked). */
  whyNoEngine(): string | null {
    const e = this.engine.value;
    if (e.state === 'failed') return e.reason;
    return null;
  }

  /** Why a paper action cannot run now: no sheet in front, or no engine (`needsEngine`); null when it can. */
  whyNot(needsEngine: boolean): string | null {
    if (!this.sheet) return ENGINE_TEXTS.noSheet;
    return needsEngine ? this.whyNoEngine() : null;
  }

  /** Brings a sheet (or Model, null) to the front; an unknown id changes nothing. The choice is dropped. */
  openSheet(id: string | null): void {
    if (id !== null && !this.book.value.sheets.some((s) => s.id === id)) return;
    if (id !== this.open.value) this.selection.set(new Set());
    this.open.set(id);
  }

  /**
   * Takes a new book (opened, or given back by the engine after an edit). The
   * sheet in front stays; if it is gone, the tab left of where it was comes
   * forward (Model when it was the first), as a closed tab does. Chosen items
   * that are gone are dropped.
   */
  setBook(next: BookView): void {
    const open = this.open.value;
    const before = this.book.value;
    this.book.set(next);
    if (open !== null && !next.sheets.some((s) => s.id === open)) {
      const at = before.sheets.findIndex((s) => s.id === open);
      this.selection.set(new Set());
      this.open.set(at > 0 ? (next.sheets[Math.min(at - 1, next.sheets.length - 1)]?.id ?? null) : null);
      return;
    }
    this.retainSelection();
  }

  /** Chooses items of the sheet in front: anew, added, or flipped one by one. Masters' and unknown ids are left out. */
  select(ids: Iterable<string>, mode: SelectMode = 'replace'): void {
    const sheet = this.sheet;
    if (!sheet) return;
    const own = new Set(sheet.items.filter((i) => !i.master).map((i) => i.id));
    const next = mode === 'replace' ? new Set<string>() : new Set(this.selection.value);
    for (const id of ids) {
      if (!own.has(id)) continue;
      if (mode === 'toggle' && next.has(id)) next.delete(id);
      else next.add(id);
    }
    this.selection.set(next);
  }

  /** Every own item of the sheet in front that is not hidden or locked. */
  selectAll(): void {
    const sheet = this.sheet;
    if (sheet) this.select(sheet.items.filter((i) => !i.master && !i.hidden && !i.locked).map((i) => i.id));
  }

  clearSelection(): void {
    this.selection.set(new Set());
  }

  private retainSelection(): void {
    const sheet = this.sheet;
    const own = new Set(sheet ? sheet.items.filter((i) => !i.master).map((i) => i.id) : []);
    this.selection.set(new Set([...this.selection.value].filter((id) => own.has(id))));
  }
}
