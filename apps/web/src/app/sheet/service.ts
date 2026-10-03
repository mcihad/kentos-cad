import type { AssetMeta } from '../../contracts/generated/sheet/AssetMeta';
import type { Capabilities } from '../../contracts/generated/sheet/Capabilities';
import type { DisplayList } from '../../contracts/generated/sheet/DisplayList';
import type { Finding } from '../../contracts/generated/sheet/Finding';
import type { NorthInfo } from '../../contracts/generated/sheet/NorthInfo';
import type { GroundPoint } from '../../contracts/generated/sheet/GroundPoint';
import type { Op } from '../../contracts/generated/sheet/Op';
import type { PaperChoice } from '../../contracts/generated/sheet/PaperChoice';
import type { SheetBook } from '../../contracts/generated/sheet/SheetBook';
import type { Template } from '../../contracts/generated/sheet/Template';
import type { ToolInfo } from '../../contracts/generated/sheet/ToolInfo';
import type { Workspace } from '../../contracts/generated/Workspace';
import { Signal, type ReadonlySignal } from '../../core/signal';
import { uuidv7 } from '../../core/uuid';
import { bookView } from '../../product/sheet/adapter';
import { bookText, errorText, loadSheetEngine, type BookText, type SheetEngine } from '../../product/sheet/engine';
import { SheetHistory } from '../../product/sheet/history';
import { NO_PROFILE, type SheetProfile } from '../../product/sheet/profile';
import { ENGINE_TEXTS, SheetState } from '../../product/sheet/state';
import { AssetStore, BookStore, deviceKeyValue, type KeyValue, type ReadBook } from '../../product/sheet/store';
import { DeviceTemplateStore } from '../../product/sheet/templateStore';
import type { ProjectTraits, TemplateCard, TemplateProvider } from '../../product/sheet/templates';
import type { PaintSources } from '../../render/sheet/painter';
import type { GalleryAbilities, TemplateAction } from '../../ui/sheet/galleryPlan';
import type { LibraryLine, SheetHost, TemplateCloud } from '../../ui/sheet/host';
import type { AppContext } from '../context';
import type { TemplateCloudApi } from './cloudApi';
import type { SheetPaint } from './paint';
import { ProjectBooks } from './projectBooks';
import type { ProjectKey } from './projectKey';
import type { LibraryProvider } from './providers';
import { TemplateShelf } from './templateShelf';
import { NO_SOURCES, ProjectReading, type EngineParts, type ServiceOptions } from './serviceParts';

/**
 * The sheet layouts' service (docs/sheet/design.md §10, §11, §13): the sheet
 * mode's state, the engine and the book's history, the work mode's profile,
 * the template sources, the project's key and what this device keeps for
 * it. It is the interface's host (ui/sheet/host.ts): every change of the
 * book is a list of the engine's operations applied as one undo step, and
 * an operation the engine refuses is said in the message log with the
 * engine's own Turkish message and code; the book stays as it was.
 *
 * The engine (3.3 MB) is fetched the first time it is needed: a sheet comes
 * forward, the gallery opens, a sheet is made; what comes with it (the
 * painter, the template actions, the cloud library: withEngine.ts) with it.
 * Until then the project's stored sheets are shown on their tabs by name
 * only. The book is written to this device after each change.
 */

export class SheetService implements SheetHost {
  readonly state = new SheetState();
  readonly profile = new Signal<SheetProfile>(NO_PROFILE);
  /** Every tool of the mode with its state, the hidden ones too (a command says why it is not offered). */
  readonly tools = new Signal<readonly ToolInfo[]>([]);
  readonly providers: readonly TemplateProvider[];
  /** The gallery's sources and the cloud library (templateShelf.ts). */
  readonly shelf: TemplateShelf;
  /** “Benim” (made here, and the account's own in the cloud) and “Benimle paylaşılanlar”. */
  readonly device: LibraryProvider;
  readonly shared: LibraryProvider;
  /** True while a sheet is in front (the shell, the ribbon and the keys follow it). */
  readonly inSheet: ReadonlySignal<boolean>;
  readonly history = new Signal<SheetHistory | null>(null);
  readonly findings = new Signal<readonly Finding[]>([]);
  /** Whether the stores survive a reload (a browser without IndexedDB keeps them in memory only). */
  readonly durable: boolean;
  readonly books: BookStore;
  readonly projectBooks: ProjectBooks;
  readonly templates: DeviceTemplateStore;
  readonly assets: AssetStore;
  private readonly ctx: AppContext;
  private engineRef: SheetEngine | null = null;
  private partsRef: EngineParts | null = null;
  private paintRef: SheetPaint | null = null;
  private loading: Promise<SheetEngine> | null = null;
  private readonly fetchEngine: () => Promise<SheetEngine>;
  private readonly fetchParts: () => Promise<EngineParts>;
  private readonly templateApi: TemplateCloudApi | undefined;
  private readonly paintedSig = new Signal(0);
  private readonly project: ProjectReading;

  /** `fetchEngine` fetches the engine (the tests start one in Node instead). */
  constructor(ctx: AppContext, kv?: KeyValue, fetchEngine: () => Promise<SheetEngine> = loadSheetEngine, options: ServiceOptions = {}) {
    this.ctx = ctx;
    this.fetchEngine = fetchEngine;
    this.fetchParts = options.parts ?? (() => import('./withEngine'));
    this.templateApi = options.templateApi;
    this.project = new ProjectReading(ctx);
    const store = kv ? { kv, durable: true } : deviceKeyValue();
    this.durable = store.durable;
    this.books = new BookStore(store.kv);
    this.projectBooks = new ProjectBooks(ctx, this.books, {
      state: this.state,
      engine: () => this.engineRef,
      history: () => this.history.value,
      emptyBook: () => (this.engineRef ? this.emptyBook(this.engineRef) : null),
    });
    this.templates = new DeviceTemplateStore(store.kv);
    this.assets = new AssetStore(store.kv);
    const inSheet = new Signal(false);
    this.state.open.subscribe((id) => inSheet.set(id !== null));
    this.inSheet = inSheet;
    this.shelf = new TemplateShelf(ctx, this.templates, this.assets, () => this.ensureEngine(), () => this.engineRef);
    this.device = this.shelf.mine;
    this.shared = this.shelf.shared;
    this.providers = this.shelf.providers;
    // The view follows the book, the sheet in front (its items' notes) and the mode; the profile the mode and the project.
    this.state.open.subscribe(() => this.refreshView());
    ctx.doc.settings.workspace.subscribe(() => {
      this.refreshProfile();
      this.refreshView();
    });
    ctx.doc.settings.plotScale.subscribe(() => this.refreshProfile());
    ctx.doc.crs.subscribe(() => this.refreshProfile());
    // A sheet's “Yeni sürüm var” follows the templates kept here (a download brings a newer revision).
    for (const p of this.providers) p.cards.subscribe(() => this.refreshView());
    // The preflight of the sheet in front follows the book and the drawing.
    this.paintedSig.subscribe(() => this.refreshFindings());
    this.state.open.subscribe(() => this.refreshFindings());
  }

  /** The project the book belongs to (projectKey.ts). */
  get key(): ReadonlySignal<ProjectKey | null> {
    return this.projectBooks.key;
  }

  /** What this device keeps for the project, read when it was opened. */
  get stored(): ReadonlySignal<ReadBook> {
    return this.projectBooks.stored;
  }

  get painted(): ReadonlySignal<number> {
    return this.paintedSig;
  }

  get sources(): PaintSources {
    return this.paintRef ?? NO_SOURCES;
  }

  /** The painter's sources and caches; there once the engine is (they come with it). */
  get paint(): SheetPaint {
    if (!this.paintRef) throw new Error(ENGINE_TEXTS.loading);
    return this.paintRef;
  }

  /** What comes with the engine, once it is there. */
  async parts(): Promise<EngineParts> {
    await this.ensureEngine();
    return this.partsRef!;
  }

  get cloudLine(): ReadonlySignal<LibraryLine> {
    return this.shelf.line;
  }

  cloud(): TemplateCloud | null {
    return this.shelf.library;
  }

  // ── The engine ───────────────────────────────────────────────────

  engine(): SheetEngine | null {
    return this.engineRef;
  }

  /**
   * The engine, fetched and started when first asked: then the project's
   * stored book is read and checked by it, and the history starts. A failed
   * fetch is said and tried again next time.
   */
  ensureEngine(): Promise<SheetEngine> {
    if (this.engineRef) return Promise.resolve(this.engineRef);
    this.loading ??= (async () => {
      this.state.engine.set({ state: 'loading' });
      try {
        // The engine and what comes with it (the painter, the template actions, the cloud library) together.
        const [engine, parts] = await Promise.all([this.fetchEngine(), this.fetchParts()]);
        this.partsRef = parts;
        this.paintRef = new parts.SheetPaint(this.ctx, this.assets, () => this.capabilities());
        this.paintRef.painted.subscribe((n) => this.paintedSig.set(n));
        this.engineRef = engine;
        this.shelf.attach(new parts.CloudLibrary(this.ctx, this.shelf, this.templateApi));
        const history = new SheetHistory(engine, this.emptyBook(engine));
        history.book.subscribe((b) => this.bookChanged(b));
        this.history.set(history);
        this.state.engine.set({ state: 'ready' });
        this.refreshProfile();
        this.projectBooks.openStored();
        return engine;
      } catch (e) {
        const reason = ENGINE_TEXTS.failed(errorText(e));
        this.state.engine.set({ state: 'failed', reason });
        this.ctx.log.error(reason);
        throw e;
      } finally {
        this.loading = null;
      }
    })();
    return this.loading;
  }

  private emptyBook(engine: SheetEngine): BookText {
    const empty: SheetBook = { schema: engine.info.bookSchema, sheets: [], masters: [], assets: [], variables: [] };
    return bookText(empty);
  }

  book(): BookText | null {
    return this.history.value?.book.value ?? null;
  }

  // ── The project and its book (projectBooks.ts) ───────────────────

  /** Follows which project is open and keeps its book on this device. */
  watchProject(): () => void {
    return this.projectBooks.watch();
  }

  /** Writes the book now if it changed since it was last written. */
  flush(): void {
    this.projectBooks.flush();
  }

  /** The book changed (an edit, an undo, a load): the view and the preflight follow, the device's copy is written soon. */
  private bookChanged(b: BookText): void {
    // What follows a change must not make the change look refused: its own trouble is said on its own.
    try {
      this.refreshView();
      this.refreshFindings();
    } catch (e) {
      this.report('Paftanın görünüşü güncellenemedi', e);
    }
    this.projectBooks.changed(b);
  }

  private refreshView(): void {
    const engine = this.engineRef;
    const b = this.book();
    if (!engine || !b) return;
    const ws = this.workspace();
    const known = this.shelf.known();
    this.state.setBook(
      bookView(b.book, {
        papers: engine.paperSizes(),
        template: (id) => known.get(id) ?? null,
        note: (item) => {
          try {
            return engine.itemNote(ws, item);
          } catch {
            return null;
          }
        },
        open: this.state.open.value,
      }),
    );
  }

  private refreshProfile(): void {
    const engine = this.engineRef;
    if (!engine) return;
    try {
      this.profile.set(engine.profileFor(this.workspace(), this.capabilities()));
      this.tools.set(engine.toolAvailability(this.workspace(), this.capabilities()));
    } catch (e) {
      this.ctx.log.error(`Pafta araçları okunamadı: ${errorText(e)}`);
    }
  }

  private refreshFindings(): void {
    const engine = this.engineRef;
    const b = this.book();
    const sheet = this.state.open.value;
    const next = engine && b && sheet && this.paintRef && b.book.sheets.some((s) => s.id === sheet) ? this.paintRef.preflight(engine, b, sheet) : [];
    const same = next.length === this.findings.value.length && next.every((f, i) => JSON.stringify(f) === JSON.stringify(this.findings.value[i]));
    if (!same) this.findings.set(next);
  }

  // ── The host's answers (ui/sheet/host.ts) ────────────────────────

  openSheet(id: string | null): void {
    if (id === null) return this.state.openSheet(null);
    if (this.engineRef) return this.state.openSheet(id);
    void this.ensureEngine().then(
      () => this.state.openSheet(id),
      () => {},
    );
  }

  plan(book: BookText, sheet: string): DisplayList | null {
    const engine = this.engineRef;
    if (!engine || !this.paintRef) return null;
    try {
      return this.paintRef.plan(engine, book, sheet);
    } catch (e) {
      this.report('Pafta çizilemedi', e);
      return null;
    }
  }

  preflight(sheet: string, dpi?: number): Finding[] {
    const engine = this.engineRef;
    const b = this.book();
    return engine && b && this.paintRef ? this.paintRef.preflight(engine, b, sheet, dpi ? { dpi } : {}) : [];
  }

  northInfo(sheet: string, item: string): NorthInfo | null {
    const engine = this.engineRef;
    const b = this.book();
    return engine && b && this.paintRef ? this.paintRef.northInfo(engine, b, sheet, item) : null;
  }

  workspace(): Workspace {
    return this.project.workspace();
  }

  capabilities(): Capabilities {
    return this.project.capabilities();
  }

  traits(): ProjectTraits {
    return this.project.traits();
  }

  whyReadOnly(): string | null {
    if (!this.history.value) return this.state.whyNoEngine() ?? ENGINE_TEXTS.loading;
    return null;
  }

  apply(ops: readonly Op[], label?: string): boolean {
    const h = this.history.value;
    if (!h) {
      this.ctx.log.warn(`${label ?? 'İşlem'} uygulanmadı: ${this.whyReadOnly()}`);
      return false;
    }
    try {
      h.apply(ops, label);
      return true;
    } catch (e) {
      this.report(`${label ?? 'İşlem'} uygulanmadı`, e);
      return false;
    }
  }

  undo(): void {
    this.step('undo');
  }

  redo(): void {
    this.step('redo');
  }

  private step(dir: 'undo' | 'redo'): void {
    const h = this.history.value;
    if (!h) return;
    try {
      const label = dir === 'undo' ? h.undo() : h.redo();
      if (label) this.ctx.log.info(`${dir === 'undo' ? 'Geri alındı' : 'Yinelendi'} (pafta): ${label}`);
    } catch (e) {
      this.report(dir === 'undo' ? 'Geri alınamadı' : 'Yinelenemedi', e);
    }
  }

  /** The engine's refusal (or anything thrown) in the message log: what was asked, the engine's words and code. */
  report(what: string, e: unknown): void {
    this.ctx.log.error(`${what}: ${errorText(e)}`);
  }

  newId(): string {
    return uuidv7();
  }

  async addPicture(file: File): Promise<AssetMeta | null> {
    return (await this.parts()).addPicture(this.ctx, this, file);
  }

  mapPlace(): { center: GroundPoint; scale: number } {
    const cam = this.ctx.view.camera;
    return { center: { x: cam.center.x, y: cam.center.y }, scale: this.ctx.doc.settings.plotScale.value || 1000 };
  }

  galleryAbilities(): GalleryAbilities {
    return { engine: this.state.whyNoEngine(), cloud: this.shelf.why(), account: this.ctx.cloud.auth.value === 'signedIn' ? null : this.shelf.why(), publishTo: this.shelf.organizations()?.filter((o) => o.canPublish).length ?? 0 };
  }

  templateNeeds(template: Template): Finding[] {
    return this.engineRef && this.partsRef ? this.partsRef.templateNeeds(this.engineRef, this, template) : [];
  }

  templatePreview(template: Template): { book: BookText; sheet: string } | null {
    return this.engineRef && this.partsRef ? this.partsRef.templatePreview(this.engineRef, this, template) : null;
  }

  async templateAction(card: TemplateCard, action: TemplateAction, paper?: PaperChoice | null): Promise<boolean> {
    return (await this.parts()).templateAction(this.ctx, this, card, action, paper ?? null);
  }

  /** A new sheet from the work mode's default template (asks the template's questions first); its id, or null. */
  async newSheet(): Promise<string | null> {
    return (await this.parts()).newSheet(this);
  }

  dispose(): void {
    this.flush();
    this.paintRef?.dispose();
    this.shelf.library?.dispose();
  }
}
