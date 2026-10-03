import type { ProjectType } from '../../contracts/generated/ProjectType';
import type { Applied } from '../../contracts/generated/sheet/Applied';
import type { AssetWithBytes } from '../../contracts/generated/sheet/AssetWithBytes';
import type { Bindable } from '../../contracts/generated/sheet/Bindable';
import type { Capabilities } from '../../contracts/generated/sheet/Capabilities';
import type { DisplayList } from '../../contracts/generated/sheet/DisplayList';
import type { EngineInfo } from '../../contracts/generated/sheet/EngineInfo';
import type { Finding } from '../../contracts/generated/sheet/Finding';
import type { Hits } from '../../contracts/generated/sheet/Hits';
import type { HitQuery } from '../../contracts/generated/sheet/HitQuery';
import type { Instance } from '../../contracts/generated/sheet/Instance';
import type { InstanceIds } from '../../contracts/generated/sheet/InstanceIds';
import type { InstanceOptions } from '../../contracts/generated/sheet/InstanceOptions';
import type { Item } from '../../contracts/generated/sheet/Item';
import type { LocalTemplate } from '../../contracts/generated/sheet/LocalTemplate';
import type { RemoteTemplate } from '../../contracts/generated/sheet/RemoteTemplate';
import type { SyncPlan } from '../../contracts/generated/sheet/SyncPlan';
import type { NewItem } from '../../contracts/generated/sheet/NewItem';
import type { Op } from '../../contracts/generated/sheet/Op';
import type { PaperSize } from '../../contracts/generated/sheet/PaperSize';
import type { Profile } from '../../contracts/generated/sheet/Profile';
import type { RankedTemplate } from '../../contracts/generated/sheet/RankedTemplate';
import type { RectUm } from '../../contracts/generated/sheet/RectUm';
import type { RenderInputs } from '../../contracts/generated/sheet/RenderInputs';
import type { ResizeSnap } from '../../contracts/generated/sheet/ResizeSnap';
import type { KpaftaFile } from '../../contracts/generated/sheet/KpaftaFile';
import type { PdfFace } from '../../contracts/generated/sheet/PdfFace';
import type { PdfInputs } from '../../contracts/generated/sheet/PdfInputs';
import type { PdfOptions } from '../../contracts/generated/sheet/PdfOptions';
import type { PdfSvgSize } from '../../contracts/generated/sheet/PdfSvgSize';
import type { TmCrs } from '../../contracts/generated/sheet/TmCrs';
import type { MagneticField } from '../../contracts/generated/sheet/MagneticField';
import type { WmmInfo } from '../../contracts/generated/sheet/WmmInfo';
import type { NorthInfo } from '../../contracts/generated/sheet/NorthInfo';
import type { SheetBook } from '../../contracts/generated/sheet/SheetBook';
import type { SheetError } from '../../contracts/generated/sheet/SheetError';
import type { SnapOptions } from '../../contracts/generated/sheet/SnapOptions';
import type { SnapResult } from '../../contracts/generated/sheet/SnapResult';
import type { SvgOptions } from '../../contracts/generated/sheet/SvgOptions';
import type { Template } from '../../contracts/generated/sheet/Template';
import type { TemplateMeta } from '../../contracts/generated/sheet/TemplateMeta';
import type { ToolInfo } from '../../contracts/generated/sheet/ToolInfo';
import type { Handle } from '../../contracts/generated/sheet/Handle';
import type { Workspace } from '../../contracts/generated/Workspace';
import type * as Pkg from './pkg/kentos_sheet_wasm';

/**
 * The sheet engine (kentos-sheet over WASM, crates/wasm/sheet-wasm;
 * docs/sheet/design.md §1): a thin, typed face over the package. Every
 * answer of the package is a JSON envelope, `{ok: true, value}` or
 * `{ok: false, error: {code, message, path}}`; a refusal comes up here as a
 * `SheetEngineError` that carries the engine's stable code and its Turkish
 * message (the cause and the fix), never swallowed and never retold. Books
 * cross as JSON text: the history keeps each book's text beside it, so a
 * book is written once per edit, not once per call.
 *
 * The package is 3.3 MB (0.94 MB compressed): it is fetched the first time a
 * sheet comes forward or the template gallery opens (`loadSheetEngine`),
 * never when the app starts (CLAUDE.md §20).
 */

/** A refusal of the engine: its stable code (`duplicate_id`, `bad_json` …), its Turkish message and where. */
export class SheetEngineError extends Error {
  readonly code: string;
  readonly path: string | undefined;

  constructor(e: SheetError) {
    super(e.message);
    this.name = 'SheetEngineError';
    this.code = e.code;
    this.path = e.path;
  }

  /** As the interface writes it: the message, then the code to quote when reporting it. */
  get text(): string {
    return `${this.message} (${this.code})`;
  }
}

/** The message of anything a sheet action threw, as the interface says it (the engine's with its code). */
export function errorText(e: unknown): string {
  if (e instanceof SheetEngineError) return e.text;
  return e instanceof Error ? e.message : String(e);
}

type Envelope<T> = { ok: true; value: T } | { ok: false; error: SheetError };

/** The value of an envelope, or the engine's refusal thrown. */
function open<T>(answer: string): T {
  const r = JSON.parse(answer) as Envelope<T>;
  if (!r.ok) throw new SheetEngineError(r.error);
  return r.value;
}

/** An Error the engine threw (no envelope: `SnapSession`, `toPdf`), its message "code: message", as its refusal. */
function thrown(e: unknown, fallback: string): SheetEngineError {
  const text = e instanceof Error ? e.message : String(e);
  const at = text.indexOf(': ');
  return new SheetEngineError(at > 0 && /^[a-z_]+$/.test(text.slice(0, at)) ? { code: text.slice(0, at), message: text.slice(at + 2) } : { code: fallback, message: text });
}

/** A book as the engine reads it: the object and its JSON text, made together. */
export interface BookText {
  readonly book: SheetBook;
  readonly text: string;
}

export const bookText = (book: SheetBook): BookText => ({ book, text: JSON.stringify(book) });

/** A drag's snapping, worked out once at its start (`SnapSession`); free it when the drag ends. */
export interface SnapDrag {
  /** The moving items' box at the start. */
  readonly box: RectUm;
  /** The offset asked (µm) snapped within `tolerance` µm, with the lines, gaps and marks to paint. */
  query(dx: number, dy: number, tolerance: number): SnapResult;
  /** The one moving item's handle dragged to (x, y), snapped. */
  resize(handle: Handle, x: number, y: number, tolerance: number, keepAspect: boolean): ResizeSnap;
  free(): void;
}

/** The functions the package exports (its generated `.d.ts`). */
export type SheetPackage = typeof Pkg;

export class SheetEngine {
  readonly info: EngineInfo;
  private readonly m: SheetPackage;
  private templates: Template[] | null = null;
  private papers: PaperSize[] | null = null;
  private scales: number[] | null = null;
  private bindables: Bindable[] | null = null;

  constructor(m: SheetPackage) {
    this.m = m;
    this.info = open<EngineInfo>(m.engineInfo());
  }

  // ── Tables (the engine's data) ───────────────────────────────────

  /** ISO A and B papers, portrait, µm. */
  paperSizes(): readonly PaperSize[] {
    return (this.papers ??= open<PaperSize[]>(this.m.paperSizes()));
  }

  /** The standard scales' denominators, smallest first. */
  standardScales(): readonly number[] {
    return (this.scales ??= open<number[]>(this.m.standardScales()));
  }

  /** What an expression may drive (ƒ). */
  bindableProperties(): readonly Bindable[] {
    return (this.bindables ??= open<Bindable[]>(this.m.bindableProperties()));
  }

  /** The program's ten templates (read-only, `sys:` ids). */
  systemTemplates(): readonly Template[] {
    return (this.templates ??= open<Template[]>(this.m.systemTemplates()));
  }

  // ── Books ────────────────────────────────────────────────────────

  /** A stored book read, checked against every rule and normalised (a load). */
  readBook(json: string): BookText {
    return bookText(open<SheetBook>(this.m.readBook(json)));
  }

  /** The SHA-256 of a book's canonical JSON (a change test, a cache key). */
  bookDigest(book: BookText): string {
    return open<string>(this.m.bookDigest(book.text));
  }

  /** One operation: the new book, the operations that take it back and the undo step's name. */
  applyOp(book: BookText, op: Op): Applied {
    return open<Applied>(this.m.applyOp(book.text, JSON.stringify(op)));
  }

  /** Several operations as one undo step; none is applied if one fails. */
  applyOps(book: BookText, ops: readonly Op[]): Applied {
    return open<Applied>(this.m.applyOps(book.text, JSON.stringify(ops)));
  }

  // ── Templates ────────────────────────────────────────────────────

  /** A user's template read (an older one migrated), checked and normalised. */
  validateTemplate(json: string): Template {
    return open<Template>(this.m.validateTemplate(json));
  }

  /**
   * A `.kpafta` file's text (design §10; the one codec, the core's, the
   * desktop's too): the book checked, the pictures' bytes in the book's
   * order, only those it lists; one whose bytes do not match its digest is
   * refused (`bad_asset`), one without bytes is left out.
   */
  encodeKpafta(book: SheetBook, assets: readonly AssetWithBytes[]): string {
    return open<string>(this.m.encodeKpafta(JSON.stringify(book), JSON.stringify(assets)));
  }

  /** A `.kpafta` file read: its book checked and normalised as `readBook` does, its pictures checked; never half read. */
  decodeKpafta(text: string): KpaftaFile {
    return open<KpaftaFile>(this.m.decodeKpafta(text));
  }

  /** A new sheet (and master) from a template, with the host's ids. */
  instantiateTemplate(template: Template, ids: InstanceIds, options: InstanceOptions): Instance {
    return open<Instance>(this.m.instantiateTemplate(JSON.stringify(template), JSON.stringify(ids), JSON.stringify(options)));
  }

  /** A sheet saved as a template; `assets` are the bytes of the pictures it uses. */
  extractTemplate(book: BookText, sheet: string, meta: TemplateMeta, assets: readonly AssetWithBytes[]): Template {
    return open<Template>(this.m.extractTemplate(book.text, sheet, JSON.stringify(meta), JSON.stringify(assets)));
  }

  /** The gallery's order of templates for a project's mode and type. */
  rankTemplates(metas: readonly TemplateMeta[], workspace: Workspace | null, projectType: ProjectType | null): RankedTemplate[] {
    return open<RankedTemplate[]>(this.m.rankTemplates(JSON.stringify(metas), JSON.stringify(workspace), JSON.stringify(projectType)));
  }

  // ── The work mode (design §11a) ──────────────────────────────────

  profileFor(workspace: Workspace | null, capabilities: Capabilities): Profile {
    return open<Profile>(this.m.profileFor(JSON.stringify(workspace), JSON.stringify(capabilities)));
  }

  toolAvailability(workspace: Workspace | null, capabilities: Capabilities): ToolInfo[] {
    return open<ToolInfo[]>(this.m.toolAvailability(JSON.stringify(workspace), JSON.stringify(capabilities)));
  }

  /** A new item from a tool of the mode (the host gives its id, name and frame). */
  newItem(workspace: Workspace | null, capabilities: Capabilities, request: NewItem): Item {
    return open<Item>(this.m.newItem(JSON.stringify(workspace), JSON.stringify(capabilities), JSON.stringify(request)));
  }

  /** The inspector's note for an item another mode's tool made; null for this mode's. */
  itemNote(workspace: Workspace | null, item: Item): string | null {
    return open<string | null>(this.m.itemNote(JSON.stringify(workspace), JSON.stringify(item)));
  }

  // ── On the paper ─────────────────────────────────────────────────

  /** The items under a point, inside a box, or a chosen item's handle. */
  hitTest(book: BookText, sheet: string, query: HitQuery): Hits {
    return open<Hits>(this.m.hitTest(book.text, sheet, JSON.stringify(query)));
  }

  /** A drag's snapping for these items; a refusal (an unknown id) is thrown as the engine's error. */
  snapDrag(book: BookText, sheet: string, moving: readonly string[], options?: Partial<SnapOptions>): SnapDrag {
    let s: Pkg.SnapSession;
    try {
      s = new this.m.SnapSession(book.text, sheet, JSON.stringify(moving), options ? JSON.stringify(options) : '{}');
    } catch (e) {
      // The constructor throws: an Error whose message is "code: message".
      throw thrown(e, 'snap_failed');
    }
    const box = open<RectUm>(s.movingBox());
    let freed = false;
    return {
      box,
      query: (dx, dy, tolerance) => open<SnapResult>(s.query(Math.round(dx), Math.round(dy), Math.max(0, Math.round(tolerance)))),
      resize: (handle, x, y, tolerance, keepAspect) => open<ResizeSnap>(s.queryResize(handle, Math.round(x), Math.round(y), Math.max(0, Math.round(tolerance)), keepAspect)),
      free: () => {
        if (freed) return;
        freed = true;
        s.free();
      },
    };
  }

  /** A rotation (millidegrees) snapped: with `step` to 15°, otherwise to a quarter turn within 2°. */
  snapRotation(angle: number, step: boolean): number {
    return this.m.snapRotation(Math.round(angle), step);
  }

  /** What to paint for a sheet. */
  displayList(book: BookText, sheet: string, inputs: RenderInputs): DisplayList {
    return open<DisplayList>(this.m.displayList(book.text, sheet, JSON.stringify(inputs)));
  }

  /** A display list as an SVG document. */
  toSvg(list: DisplayList, options: SvgOptions): string {
    return open<string>(this.m.toSvg(JSON.stringify(list), JSON.stringify(options)));
  }

  /** What is wrong with a sheet before export, errors first. */
  preflight(book: BookText, sheet: string, inputs: RenderInputs): Finding[] {
    return open<Finding[]>(this.m.preflight(book.text, sheet, JSON.stringify(inputs)));
  }

  /**
   * The sheets as one PDF (design §9a; the core's writer, the desktop's too):
   * a page each, the maps' content as given (vectors or a picture), the
   * faces' subsets embedded, GeoPDF where asked. Thrown as the engine's
   * refusal (`pdf_font_missing` …) when it cannot be written.
   */
  toPdf(book: BookText, inputs: PdfInputs, options: PdfOptions): Uint8Array {
    try {
      return this.m.toPdf(book.text, JSON.stringify(inputs), JSON.stringify(options));
    } catch (e) {
      throw thrown(e, 'pdf_failed');
    }
  }

  /** The faces those sheets and maps write with: the TrueType files `toPdf` wants in `inputs.fonts`. */
  pdfFonts(book: BookText, inputs: PdfInputs, options: PdfOptions): PdfFace[] {
    return open<PdfFace[]>(this.m.pdfFonts(book.text, JSON.stringify(inputs), JSON.stringify(options)));
  }

  /**
   * The SVG pictures of those sheets, each once, and the pixels the host draws each at for
   * `PdfAsset.raster` (its largest frame at `dpi`, the longest side at most 8192): the core draws no SVG.
   */
  pdfSvgSizes(book: BookText, inputs: PdfInputs, options: PdfOptions, dpi: number): PdfSvgSize[] {
    return open<PdfSvgSize[]>(this.m.pdfSvgSizes(book.text, JSON.stringify(inputs), JSON.stringify(options), dpi));
  }

  /**
   * What the PDF writes differently from the screen by what the inputs hold: an SVG picture as the
   * host's PNG (`svg_as_picture`, information) or, without one, as the missing picture's box
   * (`svg_not_in_pdf`, a warning).
   */
  pdfFindings(book: BookText, inputs: PdfInputs, options: PdfOptions): Finding[] {
    return open<Finding[]>(this.m.pdfFindings(book.text, JSON.stringify(inputs), JSON.stringify(options)));
  }

  /** A transverse Mercator system's WKT 1 from the registry's values (GeoPDF's `GCS`). */
  tmWkt(crs: TmCrs): string {
    return open<string>(this.m.tmWkt(JSON.stringify(crs)));
  }

  /**
   * A sheet's export file name (design §9): its `export.fileName` written in the sheet's scope, a fixed
   * name as it is, the sheet's name when it writes nothing. The host takes out what its files refuse.
   */
  exportName(book: BookText, sheet: string, inputs: RenderInputs): string {
    return open<string>(this.m.exportName(book.text, sheet, JSON.stringify(inputs)));
  }

  /**
   * The World Magnetic Model's field (design §8a, WMM2025) at a geodetic latitude and longitude
   * (degrees, WGS84), a height above the ellipsoid (km) and a decimal year.
   */
  magneticField(lat: number, lon: number, heightKm: number, year: number): MagneticField {
    return open<MagneticField>(this.m.magneticField(lat, lon, heightKm, year));
  }

  /** The magnetic model's name, release and the decimal years it is valid for. */
  wmmInfo(): WmmInfo {
    return open<WmmInfo>(this.m.wmmInfo());
  }

  /** An ISO date (`2026-10-03`) as the decimal year the magnetic model takes. */
  decimalYear(iso: string): number {
    return open<number>(this.m.decimalYear(iso));
  }

  /**
   * What a north arrow shows and from what (design §8a): its map's centre as latitude and longitude, the
   * convergence, the declination with its source, the model, the date used and where it came from, and
   * whether that date is inside the model's years. `inputs` are `displayList`'s.
   */
  northInfo(book: BookText, sheet: string, item: string, inputs: RenderInputs): NorthInfo {
    return open<NorthInfo>(this.m.northInfo(book.text, sheet, item, JSON.stringify(inputs)));
  }

  /** What to download, upload, delete or keep for the cloud's templates (design §13; the sync of Part C). */
  planSync(local: readonly LocalTemplate[], remote: readonly RemoteTemplate[]): SyncPlan {
    return open<SyncPlan>(this.m.planSync(JSON.stringify(local), JSON.stringify(remote)));
  }

  /** Null when an expression compiles; otherwise the engine's reason (`expression_error`). */
  checkExpression(source: string): SheetError | null {
    const r = JSON.parse(this.m.checkExpression(source)) as Envelope<null>;
    return r.ok ? null : r.error;
  }
}

let loading: Promise<SheetEngine> | null = null;

/**
 * The engine, fetched and started the first time it is asked for: a sheet
 * comes forward, the gallery opens, a stored book is opened. A failed load
 * is not kept: the next ask tries again.
 */
export function loadSheetEngine(): Promise<SheetEngine> {
  return (loading ??= import('./pkg/kentos_sheet_wasm')
    .then(async (m) => {
      await m.default();
      return new SheetEngine(m);
    })
    .catch((e: unknown) => {
      loading = null;
      throw e;
    }));
}

/** The engine from a package already started (the tests: `initSync` in Node). */
export const sheetEngineOf = (m: SheetPackage): SheetEngine => new SheetEngine(m);
