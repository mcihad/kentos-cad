import type { DisplayList } from '../../contracts/generated/sheet/DisplayList';
import type { Finding } from '../../contracts/generated/sheet/Finding';
import type { NorthInfo } from '../../contracts/generated/sheet/NorthInfo';
import type { MapPrim } from '../../contracts/generated/sheet/MapPrim';
import type { RenderInputs } from '../../contracts/generated/sheet/RenderInputs';
import type { RenderMode } from '../../contracts/generated/sheet/RenderMode';
import type { Sheet } from '../../contracts/generated/sheet/Sheet';
import type { DrawingFont } from '../../contracts/generated/DrawingFont';
import { Signal } from '../../core/signal';
import type { Bounds } from '../../model/geometry';
import type { Symbol } from '../../model/style';
import { errorText, type BookText, type SheetEngine } from '../../product/sheet/engine';
import type { AssetStore } from '../../product/sheet/store';
import { fontsOf, type PaintSources } from '../../render/sheet/painter';
import { drawSymbolPreview } from '../../render/symbolPreview';
import { LEGEND_PAPER } from '../../style/legend';
import { drawingFontById } from '../appearance';
import type { AppContext } from '../context';
import { extentsOf, renderInputs } from './inputs';
import { MapFrames } from './mapFrames';

/**
 * What a sheet is painted with (docs/sheet/design.md §8): the engine's plan
 * for a sheet with the project's inputs (inputs.ts), kept while neither the
 * book nor the drawing changes; the pictures' bytes from this device's
 * store, decoded once; the map frames' content (mapFrames.ts); the legends'
 * symbols as small pictures drawn by the legend window's own preview; the
 * drawing's typefaces, loaded before text is set in them. `painted` is
 * bumped when any of them arrives, and the paper is painted again.
 */

/** A legend symbol's picture: drawn at 300 dpi, enough for an export and sharp on the screen. */
const SYMBOL_DPI = 300;

export class SheetPaint implements PaintSources {
  readonly painted = new Signal(0);
  private mapsRef: MapFrames | null = null;
  private readonly ctx: AppContext;
  private readonly assets: AssetStore;
  private readonly decoded = new Map<string, ImageBitmap | HTMLImageElement | 'loading' | 'missing'>();
  private readonly symbols = new Map<string, HTMLCanvasElement>();
  private readonly fontsAsked = new Set<string>();
  /** Bumped by what changes the inputs while the book stays: the drawing, the selection a list reads, the project's name. */
  private epoch = 0;
  /** Plans kept by book, sheet, mode and inputs (the desk's, the small papers', the gallery's cards). */
  private readonly plans = new Map<string, DisplayList>();
  private readonly extents = new Map<string, Map<string, Bounds>>();
  private readonly offs: (() => void)[] = [];
  private readonly capabilities: () => RenderInputs['capabilities'];
  /** The digests this device has bytes for (the preflight's `missing_asset`); null until read. */
  private have: string[] | null = null;

  constructor(ctx: AppContext, assets: AssetStore, capabilities: () => RenderInputs['capabilities']) {
    this.ctx = ctx;
    this.assets = assets;
    this.capabilities = capabilities;
    const stale = () => {
      this.epoch++;
      this.bump();
    };
    this.offs.push(
      ctx.doc.events.on('touched', stale),
      ctx.doc.events.on('reset', stale),
      ctx.doc.layers.events.on('state', stale),
      ctx.doc.layers.events.on('structure', stale),
      ctx.selection.ids.subscribe(stale),
      ctx.doc.name.subscribe(stale),
      ctx.doc.crs.subscribe(stale),
      ctx.cloud.me.subscribe(stale),
    );
    void this.refreshAssets();
  }

  /** Reads again which pictures this device has (after one was added or a file was read). */
  async refreshAssets(): Promise<void> {
    this.have = await this.assets.digests().catch(() => null);
    for (const [sha, d] of this.decoded) if (d === 'missing') this.decoded.delete(sha);
    this.epoch++;
    this.bump();
  }

  /** The map frames' pictures, made when a map is first painted (a WebGL2 context of their own). */
  get maps(): MapFrames {
    if (!this.mapsRef) {
      this.mapsRef = new MapFrames(this.ctx);
      this.mapsRef.onChange = () => this.bump();
    }
    return this.mapsRef;
  }

  dispose(): void {
    this.offs.forEach((off) => off());
    this.mapsRef?.dispose();
  }

  private bump(): void {
    this.painted.set(this.painted.value + 1);
  }

  /** The project's inputs for a sheet (the maps' ground from the engine, `extentsOf`). */
  inputs(engine: SheetEngine, book: BookText, sheet: Sheet, mode: RenderMode, more: { assets?: readonly string[]; dpi?: number } = {}): RenderInputs {
    return renderInputs(this.ctx, book.book, sheet, {
      mode,
      capabilities: this.capabilities(),
      extents: this.extentsFor(engine, book, sheet),
      legendSymbol: (s, size) => this.legendSymbol(s, size),
      ...more,
    });
  }

  /**
   * The ground each map covers: the plan of the sheet without the drawing's
   * rows, kept while the book stays (a map's view is the book's).
   */
  private extentsFor(engine: SheetEngine, book: BookText, sheet: Sheet): Map<string, Bounds> {
    const key = `${sheet.id}\u0000${book.text}`;
    const hit = this.extents.get(key);
    if (hit) return hit;
    const needs = sheet.items.some((i) => (i.kind.type === 'table' && i.kind.source.type === 'layer' && i.kind.source.onlyInMap) || (i.kind.type === 'legend' && i.kind.map));
    let map = new Map<string, Bounds>();
    if (needs) {
      try {
        const bare = renderInputs(this.ctx, book.book, sheet, { mode: 'design', capabilities: this.capabilities(), extents: new Map(), legendSymbol: () => null });
        map = extentsOf(engine.displayList(book, sheet.id, { ...bare, tables: [], legends: [], coordinates: [] }));
      } catch {
        // The full plan that follows says what is wrong with the sheet.
      }
    }
    remember(this.extents, key, map);
    return map;
  }

  /** The plan of a sheet, kept while the book, the sheet and the drawing stay; the engine's refusal is thrown. */
  plan(engine: SheetEngine, book: BookText, sheetId: string, mode: RenderMode = 'design'): DisplayList {
    const key = `${sheetId}\u0000${mode}\u0000${this.epoch}\u0000${book.text}`;
    const hit = this.plans.get(key);
    if (hit) return hit;
    const sheet = book.book.sheets.find((s) => s.id === sheetId);
    if (!sheet) throw new Error('Pafta kitapta yok.');
    const list = engine.displayList(book, sheetId, this.inputs(engine, book, sheet, mode));
    remember(this.plans, key, list);
    this.loadFonts(list);
    return list;
  }

  /** The preflight of a sheet as designed; the engine's refusal comes back as one error finding. */
  /** A north arrow's lines (`northInfo`), with the inputs its paper is drawn with; null when it cannot be read. */
  northInfo(engine: SheetEngine, book: BookText, sheetId: string, itemId: string): NorthInfo | null {
    const sheet = book.book.sheets.find((s) => s.id === sheetId);
    if (!sheet) return null;
    try {
      return engine.northInfo(book, sheetId, itemId, this.inputs(engine, book, sheet, 'design'));
    } catch {
      return null;
    }
  }

  preflight(engine: SheetEngine, book: BookText, sheetId: string, more: { dpi?: number } = {}): Finding[] {
    const sheet = book.book.sheets.find((s) => s.id === sheetId);
    if (!sheet) return [];
    try {
      return engine.preflight(book, sheetId, this.inputs(engine, book, sheet, 'export', { ...more, ...(this.have ? { assets: this.have } : {}) }));
    } catch (e) {
      return [{ severity: 'error', code: 'engine', message: `Ön denetim yapılamadı: ${errorText(e)}`, fix: 'Son değişikliği geri alıp yeniden deneyin.', fixes: [], placeholder: false }];
    }
  }

  // ── PaintSources ─────────────────────────────────────────────────

  family(font: string): string {
    return drawingFontById(font as DrawingFont).family;
  }

  map(prim: MapPrim, pxPerUm: number): CanvasImageSource | null {
    return this.maps.picture(prim, pxPerUm);
  }

  image(sha: string): CanvasImageSource | null {
    const symbol = this.symbols.get(sha);
    if (symbol) return symbol;
    const d = this.decoded.get(sha);
    if (d === 'loading' || d === 'missing') return null;
    if (d) return d;
    this.decoded.set(sha, 'loading');
    void this.decode(sha);
    return null;
  }

  private async decode(sha: string): Promise<void> {
    const rec = await this.assets.get(sha).catch(() => null);
    if (!rec) {
      this.decoded.set(sha, 'missing');
      return;
    }
    try {
      const type = rec.meta.kind === 'svg' ? 'image/svg+xml' : rec.meta.kind === 'jpeg' ? 'image/jpeg' : 'image/png';
      const blob = new Blob([rec.bytes as Uint8Array<ArrayBuffer>], { type });
      // An SVG is decoded by an image element (createImageBitmap does not take one everywhere).
      const img =
        rec.meta.kind === 'svg'
          ? await new Promise<HTMLImageElement>((resolve, reject) => {
              const el = new Image();
              const url = URL.createObjectURL(blob);
              el.onload = () => {
                URL.revokeObjectURL(url);
                resolve(el);
              };
              el.onerror = () => {
                URL.revokeObjectURL(url);
                reject(new Error('SVG çözülemedi'));
              };
              el.src = url;
            })
          : await createImageBitmap(blob);
      this.decoded.set(sha, img);
      this.bump();
    } catch (e) {
      this.decoded.set(sha, 'missing');
      this.ctx.log.warn(`“${rec.meta.name}” resmi çözülemedi: ${(e as Error).message}. Dosyayı yeniden ekleyin.`);
    }
  }

  /** A legend row's symbol as a small picture, keyed by what it draws (each drawn once). */
  legendSymbol(symbol: Symbol, size: { width: number; height: number }): string | null {
    const key = `lejant:${hash(JSON.stringify(symbol))}:${size.width}x${size.height}`;
    if (this.symbols.has(key)) return key;
    const px = (um: number) => Math.max(4, Math.round((um / 25_400) * SYMBOL_DPI));
    const canvas = document.createElement('canvas');
    canvas.setAttribute('width', String(px(size.width)));
    canvas.setAttribute('height', String(px(size.height)));
    const palette = { ...this.ctx.view.palette, ...LEGEND_PAPER, background: [...LEGEND_PAPER.background] as [number, number, number, number] };
    const ok = drawSymbolPreview(canvas, symbol, {
      palette,
      library: this.ctx.styles.library,
      pixelRatio: 1,
      background: null,
      onLoad: () => {
        drawSymbolPreview(canvas, symbol, { palette, library: this.ctx.styles.library, pixelRatio: 1, background: null });
        this.bump();
      },
    });
    if (!ok) return null;
    this.symbols.set(key, canvas);
    return key;
  }

  /** A legend picture's PNG as a data URL, for the SVG export. */
  symbolHref(key: string): string | null {
    return this.symbols.get(key)?.toDataURL('image/png') ?? null;
  }

  /** Asks the browser for the typefaces a plan sets text in; the paper is painted again once they are there. */
  private loadFonts(list: DisplayList): void {
    if (typeof document === 'undefined' || !document.fonts) return;
    const wanted = fontsOf(list, (f) => this.family(f)).filter((f) => !this.fontsAsked.has(f));
    if (!wanted.length) return;
    wanted.forEach((f) => this.fontsAsked.add(f));
    void Promise.all(wanted.map((f) => document.fonts.load(f).catch(() => []))).then(() => this.bump());
  }
}

/** Keeps a value, the last used last, at most `KEEP` of them. */
const KEEP = 16;
function remember<V>(m: Map<string, V>, key: string, v: V): void {
  m.delete(key);
  m.set(key, v);
  while (m.size > KEEP) m.delete(m.keys().next().value!);
}

/** FNV-1a, 32 bits, as hex: a key for what a symbol draws (not a digest of anything kept). */
function hash(text: string): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return (h >>> 0).toString(16).padStart(8, '0');
}
