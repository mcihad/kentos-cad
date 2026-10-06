import type { AppContext } from '../app/context';
import { DisposableStore, listen } from '../core/disposable';
import { Emitter } from '../core/emitter';
import { Signal } from '../core/signal';
import { changedDefinitions } from '../model/blocks';
import type { Entity, TextAlign, TextRun } from '../model/entities';
import type { DimensionLook, TextFace } from '../model/annotationStyles';
import type { DimensionLayout } from '../model/geom/dimension';
import type { Affine } from '../model/geom/affine';
import type { Edge } from '../model/geom/intersect';
import type { ExtendResult, TrimResult } from '../model/ops/trim';
import type { Bounds, Vec2 } from '../model/geometry';
import { parseHex, readCanvasPalette, withAlpha, type CanvasPalette } from '../render/color';
import { onFaceLoaded } from '../render/drawingFaces';
import { createBackend } from '../render/createBackend';
import { buildGrid, gridExtent, type GridExtent } from '../render/grid';
import { Atlas } from '../render/atlas';
import { buildSceneLayer } from '../render/sceneBuilder';
import { buildStyledLayer } from '../render/styledLayer';
import type { BackendKind, RenderBackend } from '../render/types';
import { webgpuSupported } from '../render/webgpu/support';
import type { ToolPointer } from '../tools/Tool';
import { Camera } from './Camera';
import { drawCrosshair, drawGrips, drawLabels, drawMarkedVertices, drawNorthArrow, drawObjectTracking, drawScaleBar, drawSearchMark, drawSnap, drawUcsIcon, GRIP_HIT_PX, midGripVisible, paragraphRecords } from './overlay';
import { alongTrack, trackAngles, trackPoint, type TrackHit } from './objectTracking';
import { ViewNavigation } from './viewHistory';
import { NavigationCards } from './navigationCards';
import { METRES_PER_PX, symbolScaleOf } from './symbolScale';
import type { ExprColumnData } from '../wasm/core';
import { extensionAlong, extensionAt, PickIndex, type Extension, type SnapHit, type SnapKind } from './picking';
import { screenScale, snapInRange } from './snapRange';

/** Right-button menus the UI draws: idle selection, a running command, or snap overrides. */
export type ViewportMenuKind = 'select' | 'command' | 'snap';

const isConstruction = (e: Entity) => e.kind === 'xline' || e.kind === 'ray';

/** New text typed in place: where, how big and at what angle, and where the result goes. */
export interface TextInputRequest {
  at: Vec2;
  /** Text height in metres. */
  height: number;
  /** Degrees, counter-clockwise from east. */
  rotation: number;
  /** Which point of the text `at` is (docs/adr/0145); none: the left of its baseline. */
  align?: TextAlign | null;
  /** The letters' width times this; none: 1. */
  widthFactor?: number;
  /** What the field opens with, selected (Yazı's Artır); none: empty. */
  initial?: string;
  /** What the empty field shows, and the hint under it; none: Yazı's. */
  placeholder?: string;
  hint?: string;
  /** The text style's face it will have (docs/adr/0183 §2): the field writes in it; none: the project's look. */
  face?: TextFace;
  commit(text: string): void;
  /** Enter in the empty field (Kılavuz: the arrow without a note, docs/adr/0146 §7); none: as Esc. */
  empty?(): void;
  cancel(): void;
}

/**
 * Where the paragraph editor opens (Çok satırlı yazı, docs/adr/0182 §4) and how its text will look: its point (the
 * box's top left; the text's alignment the top's left), height in metres, turn in degrees, box width (none: the lines
 * end at their breaks), line spacing (none: 1) and mask; what Tamam and Vazgeç answer.
 */
export interface ParagraphInputRequest {
  at: Vec2;
  height: number;
  rotation: number;
  boxWidth?: number;
  lineSpacing?: number;
  mask?: boolean;
  /** The tool's style's width factor and face (docs/adr/0183 §2): the text is drawn and measured as it will be. */
  widthFactor?: number;
  face?: TextFace;
  commit(text: string, runs: TextRun[]): void;
  cancel(): void;
}

/** A multi-line text being written or edited, drawn as it will be: its label records (`textLines`), words, runs and face. */
export interface ParagraphPreview {
  records: readonly number[];
  text: string;
  runs: readonly TextRun[];
  face?: TextFace;
}

/** Holding the right button this long opens the command menu instead of confirming. */
/** The label picture reaches this share of the view beyond each edge, so a pan copies it instead of drawing the labels. */
const LABEL_MARGIN = 0.35;
const RIGHT_HOLD_MS = 300;
/** Resting on a snap (or an edge, Paralel) this long acquires (or releases) it. */
const TRACK_DWELL_MS = 350;
/** Acquisitions kept at most, the oldest going first. */
const MAX_TRACK_POINTS = 3;
/** How near a point lies to a parallel line through the last point to be on it, metres. */
const ON = 1e-6;

/** What rests acquire (docs/adr/0085, 0163 §2): each aid on its own. */
interface Aids {
  tracking: boolean;
  extension: boolean;
  parallel: boolean;
}

/** An acquisition: a point rested on (a tracking point, and an end's extensions with Uzantı), or an edge's direction (Paralel). */
type Acquired = { kind: 'point'; at: Vec2; extensions: Extension[] } | { kind: 'edge'; at: Vec2; dir: Vec2 };

/** What the cursor rests on: a snap point (`end`, the object whose end it is when Uzantı takes its extensions), or an edge by its direction. */
type Rest = { kind: 'point'; p: Vec2; track: boolean; end: number | null } | { kind: 'edge'; at: Vec2; dir: Vec2 };

/** The same rest goes on: the same point, or an edge of the same direction. */
const sameRest = (a: Rest, b: Rest) =>
  a.kind === 'point' ? b.kind === 'point' && a.p.x === b.p.x && a.p.y === b.p.y : b.kind === 'edge' && a.dir.x === b.dir.x && a.dir.y === b.dir.y;
/** How close (px) the cursor must come to an alignment line to lock onto it. */
const TRACK_PX = 8;
/** Snaps that make sense as tracking origins. */
const TRACKABLE = new Set<SnapKind>(['endpoint', 'midpoint', 'center', 'node', 'quadrant', 'intersection']);

/**
 * Main-thread timings (ms) the interaction harness collects in dev builds
 * (scripts/perf/interaction.mjs, docs/perf). It sets `kentos.view.probe` to
 * an empty probe and reads it back. Every use is behind import.meta.env.DEV,
 * which a production build folds to false, so production code is unchanged.
 */
export interface ViewportProbe {
  /** Per pointer move: object snap and tracking, the active tool's move handler, the whole handler. */
  moves: { snap: number; tool: number; total: number }[];
  /** Per frame: when it started (performance.now), the `stats` steps and, inside the overlay, labels and the active tool's preview. */
  frames: { at: number; build: number; render: number; overlay: number; labels: number; tool: number }[];
  /** The overlay being drawn writes its split here; the frame copies it into its entry. */
  overlay: { labels: number; tool: number };
}

interface ViewportEvents {
  contextmenu: { clientX: number; clientY: number; world: Vec2; screen: Vec2; kind: ViewportMenuKind };
  /** A tool asks the UI to edit the text of an entity in place. */
  editText: { id: number };
  /** A tool asks the UI for new text typed in place (see requestTextInput). */
  textInput: TextInputRequest;
  /** Çok satırlı yazı asks for the paragraph editor at its box (see requestParagraphInput). */
  paragraphInput: ParagraphInputRequest;
}

/**
 * Owns the GPU canvas, the overlay canvas and the camera. Translates DOM
 * input into tool calls and keeps GPU buffers in sync with the document.
 */
export class ViewportController {
  readonly camera = new Camera();
  /** The camera with the views it has been at, for Önceki and Sonraki görünüm (docs/adr/0141); the session's, empty when another drawing opens. */
  readonly navigation = new ViewNavigation(this.camera);
  readonly cursorWorld = new Signal<Vec2 | null>(null);
  /** One-shot object snap chosen from the right-button menu; cleared after the next pick. */
  readonly snapOverride = new Signal<SnapKind | null>(null);
  readonly backendKind = new Signal<BackendKind | null>(null);
  readonly backendLabel = new Signal('Başlatılıyor…');
  readonly events = new Emitter<ViewportEvents>();
  palette: CanvasPalette;

  private readonly ctx: AppContext;
  private readonly picker: PickIndex;
  private backend: RenderBackend | null = null;
  private glCanvas: HTMLCanvasElement | null = null;
  /** Backend a live switch is starting (guards against double clicks). */
  private switching: BackendKind | null = null;
  private overlay!: HTMLCanvasElement;
  private g!: CanvasRenderingContext2D;
  /** Genel bakış and Büyüteç over the drawing (docs/adr/0181). */
  private nav: NavigationCards | null = null;
  private host!: HTMLElement;
  /** The drawing's pixel ratio: the screen's, or one pixel per CSS pixel with HiDPI off (`graphics.hiDpi`). */
  private dpr = 1;
  /**
   * The overlay's pixel ratio: always the screen's. What it draws is the interface over the drawing (tool
   * previews, tags, snap marks, grips, the cross-hair, the scale bar); lowering the drawing's quality for
   * speed must not blur it, and it holds little enough that full resolution costs next to nothing.
   */
  private overlayDpr = 1;
  /**
   * The labels drawn last, as a picture: the overlay is redrawn at every pointer move (cross-hair, snap
   * marker), the labels only when the view, the drawing or their look changed (`labelsKey`).
   */
  private labelCache: { canvas: HTMLCanvasElement; key: string; center: Vec2 } | null = null;
  /** The camera the label picture is drawn with: the view and its margin. */
  private readonly labelView = new Camera();
  /** Bumped by whatever changes what the labels show (objects, layers, palette, typeface, editing). */
  private labelEpoch = 0;
  /** The camera change the last overlay frame drew (a new one means the view is moving). */
  private labelCamera = -1;
  private readonly d = new DisposableStore();

  private dirtyLayers = new Set<string>();
  /** Images of styled symbols (SVG, text, raster, pattern tiles), shared by the backends. */
  private readonly atlas = new Atlas();
  private allDirty = true;
  /** The scale symbols were last compiled at, and the wait before recompiling after a zoom (screen-sized symbols). */
  private builtSymbolScale = 0;
  private symbolTimer = 0;
  private highlightDirty = true;
  /** What the backend's grid was built for (null: it has none); rebuilt only when the view outgrows it. */
  private grid: GridExtent | null = null;
  private frameQueued = false;
  /** CPU time of the last frame's steps in ms: rebuilding dirty layers, GPU submit, the 2D overlay. */
  stats = { build: 0, render: 0, overlay: 0 };
  /** Dev builds only, set by the interaction harness (see ViewportProbe); `declare` emits no field. */
  declare probe?: ViewportProbe;
  private glQueued = false;
  /** Something besides the highlight changed since the last GPU frame: the backend draws every layer again. */
  private baseDirty = true;

  private screenCursor: Vec2 | null = null;
  /** The keys held at the last pointer move: the tool sees them again when it is given the pointer again (`repoint`). */
  private moveKeys = { shift: false, ctrl: false, alt: false };
  /** Entity whose text is being edited inline (hidden from the overlay). */
  private editingId: number | null = null;
  /** The multi-line text the paragraph editor writes (docs/adr/0182 §4), over the labels. */
  private paragraphPreview: ParagraphPreview | null = null;
  private snap: SnapHit | null = null;
  private panFrom: Vec2 | null = null;
  /** Whether the pan being dragged has kept the view it left (Önceki görünüm). */
  private panKept = false;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    this.picker = new PickIndex(ctx.doc);
    this.palette = readCanvasPalette();
  }

  /** The element the drawing is drawn in (null before `mount`). */
  get element(): HTMLElement | null {
    return this.host ?? null;
  }

  /**
   * The drawing's geometry store, to read: another picture of the same drawing (a sheet's map frames,
   * app/sheet/mapFrames.ts) is built from it, not from a second copy (docs/sheet/integration.md §3).
   */
  get geometry(): PickIndex {
    return this.picker;
  }

  async mount(host: HTMLElement): Promise<void> {
    this.host = host;
    this.overlay = document.createElement('canvas');
    this.overlay.className = 'viewport__overlay';
    this.overlay.tabIndex = 0;
    this.overlay.setAttribute('aria-label', 'Çizim alanı');
    host.append(this.overlay);
    this.g = this.overlay.getContext('2d')!;
    this.nav = new NavigationCards(host, {
      ctx: this.ctx,
      camera: this.camera,
      navigation: this.navigation,
      picker: this.picker,
      palette: () => this.palette,
      dpr: () => this.dpr,
      requestRender: () => this.requestLens(),
      zoomExtents: () => this.zoomExtents(),
    });

    // A browser without WebGPU cannot use it: the preference stays, the settings say why WebGL2 draws.
    if (!webgpuSupported())
      this.ctx.settingsStore.setConstraint('graphics.backend', { allowed: ['webgl2'], reason: 'device_unsupported', detail: 'Bu tarayıcı WebGPU sunmuyor; Chrome ya da Edge’in güncel sürümünü kullanın.' });
    // The effective engine: the device's preference, or `?renderer=` for this session (app/settings/browser.ts).
    const preferred: BackendKind[] = [this.ctx.prefs.rendererPreference.value];
    try {
      const { backend, canvas, errors } = await createBackend(host, preferred, { samples: this.ctx.prefs.msaa.value });
      this.backend = backend;
      this.glCanvas = canvas;
      backend.useAtlas(this.atlas);
      this.backendKind.set(backend.kind);
      this.backendLabel.set(backend.label);
      this.adopt(backend, preferred[0], errors);
      errors.forEach((e) => this.ctx.log.warn(`Çizim arka ucu atlandı: ${e}`));
      this.ctx.log.info(`Çizim motoru hazır: ${backend.label}`);
    } catch (err) {
      this.backendLabel.set('Kullanılamıyor');
      this.ctx.log.error(`Çizim alanı başlatılamadı: ${(err as Error).message}. Tarayıcıda donanım hızlandırmasını açın.`);
      host.classList.add('viewport--failed');
    }

    this.bindInput();
    this.bindModel();
    const ro = new ResizeObserver(() => this.resize());
    ro.observe(host);
    this.d.add(() => ro.disconnect());
    // The engine preference applies live (settings dialog, status bar, menu).
    this.d.add(this.ctx.prefs.rendererPreference.subscribe((k) => void this.switchBackend(k)));
    // Anti-aliasing applies live: new targets on the same canvas and context (TODOS.md AA-02).
    this.d.add(this.ctx.prefs.msaa.subscribe(() => this.applySamples()));
    // The pixel ratio applies at once: a resize of the same canvas.
    this.d.add(this.ctx.prefs.hiDpi.subscribe(() => this.resize()));
    this.resize();
    const home = this.ctx.doc.homeView;
    if (home) this.camera.fit(home);
    else this.showAll();
    document.fonts?.ready.then(() => this.requestOverlay());
  }

  dispose(): void {
    this.d.dispose();
    this.nav?.dispose();
    this.backend?.dispose();
  }

  /**
   * What a new backend can draw with, for the settings (TODOS.md AA-01, SET-03): its sample counts, so
   * a count it lacks resolves to the nearest one it has and the settings window says why; the engine
   * that could not start, if it fell back. Then it draws with the effective count.
   */
  private adopt(backend: RenderBackend, wanted: BackendKind, errors: readonly string[]): void {
    const store = this.ctx.settingsStore;
    const counts = [...backend.sampleCounts];
    const name = (n: number) => (n === 1 ? 'kapalı' : `${n}×`);
    store.setConstraint('graphics.msaa', {
      allowed: counts,
      reason: 'device_unsupported',
      detail: `${backend.label}: ${counts.map(name).join(', ')}.`,
    });
    if (backend.kind !== wanted) store.setConstraint('graphics.backend', { allowed: [backend.kind], reason: 'device_failed', detail: errors.join('; ') });
    backend.onSamplesFailed = (requested, working, error) => {
      // The count that failed and those above it are not asked for again with this backend.
      store.setConstraint('graphics.msaa', {
        allowed: counts.filter((c) => c < requested),
        reason: 'device_failed',
        detail: `${requested}× kurulamadı (${error}); ${working === 1 ? 'kenar yumuşatma kapalı' : `${working}×`} kullanılıyor.`,
      });
      this.ctx.log.warn(`Kenar yumuşatma ${requested}× bu aygıtta kurulamadı; son çalışan ayara (${name(working)}) dönüldü. Neden: ${error}`);
      this.requestRender();
    };
    this.applySamples();
  }

  /** The sample counts the running backend can draw with (TODOS.md AA-01); none before one runs. */
  get sampleCounts(): readonly number[] {
    return this.backend?.sampleCounts ?? [];
  }

  /** The sample count frames are drawn with now. */
  get samples(): number {
    return this.backend?.samples ?? 1;
  }

  /** Draws with the effective sample count from the next frame: the same canvas and context, new targets (AA-02). */
  private applySamples(): void {
    const backend = this.backend;
    if (!backend) return;
    const before = backend.samples;
    backend.setSamples(this.ctx.prefs.msaa.value);
    if (backend.samples !== before) this.requestRender();
  }

  /**
   * Replaces the drawing backend without reloading: the new one gets its own
   * canvas (a canvas cannot change context type), every layer is uploaded
   * again from the document and drawn before the old canvas is removed, so
   * the view never shows an empty frame. Falls back to WebGL2 like mount().
   * Anti-aliasing and the pixel ratio change without this (applySamples, resize).
   */
  async switchBackend(kind: BackendKind): Promise<void> {
    if (!this.backend) return;
    if (this.backendKind.value === kind) {
      this.switching = null; // cancels a switch still initialising
      return;
    }
    if (this.switching === kind) return;
    this.switching = kind;
    try {
      const { backend, canvas, errors } = await createBackend(this.host, [kind], { samples: this.ctx.settingsStore.requested('graphics.msaa') as number });
      if (this.switching !== kind) {
        // A newer switch started while this one initialised.
        backend.dispose();
        canvas.remove();
        return;
      }
      const old = this.backend;
      const oldCanvas = this.glCanvas;
      this.backend = backend;
      this.glCanvas = canvas;
      backend.useAtlas(this.atlas);
      backend.resize(this.size.w, this.size.h, this.dpr);
      this.allDirty = true;
      this.highlightDirty = true;
      this.grid = null;
      this.glQueued = true;
      this.baseDirty = true;
      this.frame();
      old.dispose();
      oldCanvas?.remove();
      this.backendKind.set(backend.kind);
      this.backendLabel.set(backend.label);
      this.adopt(backend, kind, errors);
      errors.forEach((e) => this.ctx.log.warn(`Çizim arka ucu atlandı: ${e}`));
      this.ctx.log.info(`Çizim motoru değişti: ${backend.label}`);
    } catch (err) {
      this.ctx.log.error(`Çizim motoru değiştirilemedi: ${(err as Error).message}`);
    } finally {
      if (this.switching === kind) this.switching = null;
    }
  }

  // ── Public API used by tools and commands ───────────────────────────

  requestRender(): void {
    this.glQueued = true;
    this.baseDirty = true;
    this.schedule();
  }

  /**
   * Only the hover or the selection changed: the backend keeps the picture of the layers and draws the
   * highlight on it (a pointer move over a large drawing no longer redraws every segment).
   */
  private requestHighlight(): void {
    this.highlightDirty = true;
    this.glQueued = true;
    this.schedule();
  }

  requestOverlay(): void {
    this.schedule();
  }

  /** Only the magnifier's picture changed (it looks elsewhere, zooms or moves): the kept layers are drawn on with it. */
  private requestLens(): void {
    this.glQueued = true;
    this.schedule();
  }

  /** Genel bakış and Büyüteç as the traces read them: the overview's extent, the magnifier's zoom, side and centre (docs/adr/0181). */
  get navigationState(): { overview: { extent: [number, number, number, number] } | null; magnifier: { zoom: number; side: string; center: [number, number] | null } | null } {
    const e = this.nav?.overviewExtent ?? null;
    const lens = this.nav?.lensState ?? null;
    return {
      overview: this.nav?.overviewShown ? { extent: e ? [e.minX, e.minY, e.maxX, e.maxY] : [0, 0, 0, 0] } : null,
      magnifier: lens && { zoom: lens.zoom, side: lens.side, center: lens.center && [lens.center.x, lens.center.y] },
    };
  }

  /** A drawing point in client pixels over the overview's picture (the traces press there); null when it shows nothing. */
  overviewClient(p: Vec2): Vec2 | null {
    return this.nav?.overviewClient(p) ?? null;
  }

  /** The object snap the marker shows now, if any (the interaction traces read it; docs/adr/0029). */
  get currentSnap(): SnapHit | null {
    return this.snap;
  }

  /** The alignment the cursor is locked to by object tracking now, if any (the interaction traces read it). A snap wins over it. */
  get currentTrack(): TrackHit | null {
    return this.snap ? null : this.track;
  }

  pick(screen: Vec2): Entity | null {
    return this.picker.hit(this.camera.screenToWorld(screen), this.ctx.prefs.pickAperture.value / this.camera.scale);
  }

  pickRect(r: Bounds, crossing: boolean): number[] {
    return this.picker.inRect(r, crossing);
  }

  /** Çitle seç (docs/adr/0141): the visible objects the fence, a path of points, crosses; a point within the pick aperture counts. */
  inFence(fence: readonly Vec2[]): number[] {
    return this.picker.inFence(fence, this.worldTolerance(this.ctx.prefs.pickAperture.value));
  }

  /** Daireyle seç: the visible objects wholly inside the circle, and with `crossing` those it touches too. */
  inCircle(c: Vec2, r: number, crossing: boolean): number[] {
    return this.picker.inCircle(c, r, crossing);
  }

  /** İçeren alanı seç: the visible closed shapes around a point with their areas, smallest first. */
  containing(p: Vec2): { entity: Entity; area: number }[] {
    return this.picker.containing(p);
  }

  /** Deliği sil and Deliği doldur: the visible areas with a hole around a point, the smallest hole first. */
  holesAt(p: Vec2): { entity: Entity; part: number; hole: number }[] {
    return this.picker.holesAt(p);
  }

  /** Edge-only pick for modify tools (ignores polygon interiors, points and text). */
  pickEdge(screen: Vec2, filter?: (e: Entity) => boolean): Entity | null {
    return this.picker.hitEdge(this.camera.screenToWorld(screen), this.ctx.prefs.pickAperture.value / this.camera.scale, filter);
  }

  /** Smallest closed shape around a world point (hatch boundary). */
  enclosingRing(world: Vec2): { entity: Entity; ring: Vec2[] } | null {
    return this.picker.enclosing(world);
  }

  /** Outlines of a block placed as an insert would place it (docs/adr/0144): Blok ekle's ghost, the Bloklar panel's picture. */
  blockOutlines(block: string, p: Vec2, scale = 1, rotation = 0, mirror = false): Float64Array {
    return this.picker.blockOutlines(block, p, scale, rotation, mirror);
  }

  /** A drawing point in the own coordinates of the definition the insert `id` places (docs/adr/0144); null when it is none. */
  insertLocal(id: number, p: Vec2): Vec2 | null {
    return this.picker.insertLocal(id, p);
  }

  /** Patlat of a block's insert: its definition's objects placed, each with its own fields (docs/adr/0144); or why not. */
  explodeInsert(e: Entity): ReturnType<PickIndex['explodeInsert']> {
    return this.picker.explodeInsert(e);
  }

  /** A dimension's measured value as drawn: its look's prefix and suffix, its kind's prefix, the value in its look's or the project's unit and decimals (length without unit; docs/adr/0183 §3). */
  dimensionText(l: Pick<DimensionLayout, 'prefix' | 'unit' | 'value'>, look: DimensionLook = {}): string {
    return this.ctx.format.dimension(l, look);
  }

  /** Visible entities whose bounds overlap `r` (candidates for boundaries and cut lines). */
  entitiesIn(r: Bounds): Entity[] {
    return this.picker.overlapping(r);
  }

  /** Ids of objects on every layer whose box overlaps `r`, in the document's order (the processing tools' "visible" scope). */
  inBox(r: Bounds): number[] {
    return this.picker.inBox(r);
  }

  /** Geometry values of these objects for expressions, one record each (`measuredAt`): processing previews and the layer style window. */
  measures(ids: readonly number[]): Float64Array {
    return this.picker.measures(ids);
  }

  /** An expression over these objects in the drawing's store, their geometry values read there: processing's previews. */
  evaluateExpression(source: string, ids: Float64Array, texts: string, textLens: Int32Array, numbers: Float64Array, scale: number, want: number): ExprColumnData {
    return this.picker.evaluateExpression(source, ids, texts, textLens, numbers, scale, want);
  }

  /** Trim `target` at `at` against the chosen boundaries, or every visible edge in view. */
  trim(target: Entity, at: Vec2, chosen: ReadonlySet<number> | null): TrimResult {
    return this.picker.trim(target, at, this.camera.visibleBounds(), chosen);
  }

  /** Extend the end of `target` nearest `at` to the chosen boundaries, or to any visible edge in view. */
  extend(target: Entity, at: Vec2, chosen: ReadonlySet<number> | null): ExtendResult {
    return this.picker.extend(target, at, this.camera.visibleBounds(), chosen);
  }

  /** Ghost outlines of objects moved by each affine (see PickIndex.ghosts). */
  ghosts(ids: readonly number[], affines: readonly Affine[], limit: number): Float64Array {
    return this.picker.ghosts(ids, affines, limit);
  }

  /** Ghost outlines of objects stretched by a window and (dx, dy). */
  stretchGhosts(ids: readonly number[], window: Bounds, dx: number, dy: number): Float64Array {
    return this.picker.stretchGhosts(ids, window, dx, dy);
  }

  /** Total length (polygons' perimeters left out) and area of objects. */
  measure(ids: Iterable<number>): { length: number; area: number } {
    return this.picker.measure(ids);
  }

  /** Boundary edges of visible entities overlapping `r`, optionally excluding one entity. */
  edgesIn(r: Bounds, exceptId?: number): Edge[] {
    return this.picker.edgesIn(r, exceptId);
  }

  /** Grip of a selected, unlocked entity under the cursor. */
  gripAt(screen: Vec2): { id: number; index: number } | null {
    const { doc, selection } = this.ctx;
    if (selection.size > 150) return null;
    const editable = [...selection.ids.value].filter((id) => {
      const e = doc.get(id);
      return !!e && !doc.layers.isLocked(e.layerId);
    });
    let found: { id: number; index: number } | null = null;
    let bestD = GRIP_HIT_PX;
    for (const set of this.picker.grips(editable))
      for (let index = 0; index < set.points.length; index++) {
        if (!midGripVisible(set, index, this.camera)) continue;
        const s = this.camera.worldToScreen(set.points[index]);
        const d = Math.hypot(s.x - screen.x, s.y - screen.y);
        if (d <= bestD) {
          bestD = d;
          found = { id: set.id, index };
        }
      }
    return found;
  }

  /** Tools call this; the UI layer owns the actual editor (tools never touch the DOM). */
  requestTextEdit(id: number): void {
    this.events.emit('editText', { id });
  }

  /** Opens a text field at a point so typing goes into the drawing, not to shortcuts. */
  requestTextInput(req: TextInputRequest): void {
    this.events.emit('textInput', req);
  }

  /** Opens the paragraph editor at a multi-line text's box (Çok satırlı yazı, docs/adr/0182 §4). */
  requestParagraphInput(req: ParagraphInputRequest): void {
    this.events.emit('paragraphInput', req);
  }

  /** The multi-line text the paragraph editor writes, drawn over the rest as it will be; null: none. */
  setParagraphPreview(preview: ParagraphPreview | null): void {
    this.paragraphPreview = preview;
    this.requestOverlay();
  }

  /** Hide an entity's overlay text while an inline editor covers it. */
  setEditing(id: number | null): void {
    this.editingId = id;
    this.labelEpoch++;
    this.requestOverlay();
  }

  /** CSS-pixel rectangle of the viewport in the page (for positioning overlays). */
  clientRect(): DOMRect {
    return this.overlay.getBoundingClientRect();
  }

  /** World distance equivalent to `px` screen pixels at the current zoom. */
  worldTolerance(px: number): number {
    return px / this.camera.scale;
  }

  /** Fits the view to every object without keeping the view it leaves: a drawing just put on screen has no history yet. */
  showAll(): void {
    const b = this.picker.extent();
    if (b) this.camera.fit(b);
  }

  /** Tümünü göster. */
  zoomExtents(): void {
    this.navigation.navigate(() => this.showAll());
  }

  zoomToSelection(): void {
    this.zoomToObjects(this.ctx.selection.ids.value);
  }

  /** Fits the view to these objects (Seçime, Katmana ve Gruba yakınlaştır); false when they have no box. */
  zoomToObjects(ids: Iterable<number>): boolean {
    const b = this.picker.extent(ids);
    if (b) this.navigation.navigate(() => this.camera.fit(b, 96));
    return !!b;
  }

  /** Brings `p` to the view's middle, its scale kept (Köşe tablosu's Göster, docs/adr/0172 §3). */
  centerOn(p: Vec2): void {
    this.navigation.navigate(() => this.camera.setView(p, this.camera.scale));
  }

  /** Fits the view to a box the user showed (Pencere yakınlaştır), `paddingPx` clear round it. */
  zoomToBox(b: Bounds, paddingPx = 0): void {
    this.navigation.navigate(() => this.camera.fit(b, paddingPx));
  }

  /** Keeps the view as it is, before a gesture that moves it (the Kaydır tool's drag). */
  rememberView(): void {
    this.navigation.remember();
  }

  /** Önceki görünüm. */
  viewBack(): boolean {
    return this.navigation.back();
  }

  /** Sonraki görünüm. */
  viewForward(): boolean {
    return this.navigation.forward();
  }

  /** The visible objects lying far from the rest of the drawing (Kapsam denetimi, docs/adr/0141). */
  extentOutliers(): number[] {
    return this.picker.extentOutliers();
  }

  /** The box around these objects (all of them without `ids`), from the geometry store; null when empty. */
  extent(ids?: Iterable<number>): Bounds | null {
    return this.picker.extent(ids);
  }

  zoomBy(factor: number): void {
    this.navigation.navigate(() => this.camera.zoomAt(factor, { x: this.camera.width / 2, y: this.camera.height / 2 }));
  }

  /** Zooms about the view's middle to a screen scale of 1:`n` (the status bar's scale selector, docs/adr/0165 §5). */
  zoomToScale(n: number): void {
    if (!(Number.isFinite(n) && n > 0)) return;
    this.zoomBy(1 / (n * METRES_PER_PX) / this.camera.scale);
  }

  focus(): void {
    this.overlay?.focus({ preventScroll: true });
  }

  /** Re-read canvas colours from CSS (theme switch). */
  refreshPalette(): void {
    // The grid follows by itself: its extent records the palette it was drawn with.
    this.palette = readCanvasPalette();
    this.labelEpoch++;
    this.allDirty = true;
    this.highlightDirty = true;
    this.requestRender();
  }

  /** The typefaces changed (interface or drawing): only the overlay draws text, so no layer is rebuilt. */
  refreshFonts(): void {
    const p = readCanvasPalette();
    this.palette = { ...this.palette, font: p.font, drawingFont: p.drawingFont };
    this.labelEpoch++;
    this.requestOverlay();
  }

  // ── Wiring ──────────────────────────────────────────────────────────

  private bindModel(): void {
    const { doc, selection, settings, tools } = this.ctx;
    const d = this.d;
    // Anything that can change a label's text, look or place (edits of any origin, a reload, layer state and style).
    const stale = () => this.labelEpoch++;
    d.add(doc.events.on('touched', stale));
    d.add(doc.events.on('reset', stale));
    // Another drawing: the views of the last one lead nowhere (docs/adr/0141).
    d.add(doc.events.on('reset', () => this.navigation.history.clear()));
    d.add(doc.events.on('attrs', stale));
    d.add(doc.layers.events.on('state', stale));
    d.add(doc.layers.events.on('structure', stale));
    // A text's own typeface came in (docs/adr/0183 §2): its labels are drawn again in it.
    d.add(
      onFaceLoaded(() => {
        stale();
        this.requestOverlay();
      }),
    );
    d.add(
      doc.events.on('changed', ({ layerIds }) => {
        layerIds.forEach((id) => this.dirtyLayers.add(id));
        this.highlightDirty = true;
        this.requestRender();
      }),
    );
    // A block definition changed (an edit, an undo, another editor's; docs/adr/0144): the layers holding an
    // insert of it, or of one placing it, are drawn again, and their labels found again.
    let blocks = doc.blocks.value;
    d.add(
      doc.blocks.subscribe((now) => {
        const changed = changedDefinitions(blocks, now);
        blocks = now;
        if (!changed.size) return;
        for (const e of doc.all()) if (e.kind === 'insert' && changed.has(e.block)) this.dirtyLayers.add(e.layerId);
        stale();
        this.highlightDirty = true;
        this.requestRender();
      }),
    );
    // Attribute changes can change data-defined symbols (a rotation, a text from a field).
    d.add(
      doc.events.on('attrs', ({ ids }) => {
        for (const id of ids) {
          const e = doc.get(id);
          // Symbols may read attributes (an object's own symbol or the layer's renderer).
          if (e && (e.symbol || doc.layers.get(e.layerId)?.style.renderer)) this.dirtyLayers.add(e.layerId);
        }
        this.requestRender();
      }),
    );
    // Paper-mm sizes follow the plot scale; library edits change symbols in use.
    d.add(
      doc.settings.plotScale.subscribe(() => {
        this.allDirty = true;
        this.requestRender();
      }),
    );
    // The scene's marks follow the project's type (docs/adr/0165 §5).
    // The project's type moves the magnifier's right place (a CBS project's grid north is above it, docs/adr/0181 §4).
    d.add(
      doc.settings.workspace.subscribe(() => {
        this.nav?.layout();
        this.requestOverlay();
      }),
    );
    d.add(
      this.ctx.styles.library.version.subscribe(() => {
        this.allDirty = true;
        this.requestRender();
      }),
    );
    this.atlas.onChange = () => this.requestRender();
    d.add(() => (this.atlas.onChange = null));
    d.add(
      doc.layers.events.on('state', ({ ids }) => {
        ids.forEach((id) => this.dirtyLayers.add(id));
        this.requestRender();
      }),
    );
    d.add(
      doc.layers.events.on('structure', () => {
        this.allDirty = true;
        this.requestRender();
      }),
    );
    const hl = () => this.requestHighlight();
    d.add(selection.ids.subscribe(hl));
    d.add(selection.hover.subscribe(hl));
    d.add(selection.vertices.subscribe(() => this.requestOverlay()));
    // The place Koordinata git marked is no more once another drawing is open (docs/adr/0178 §6).
    d.add(selection.mark.subscribe(() => this.requestOverlay()));
    d.add(doc.events.on('reset', () => selection.mark.set(null)));
    d.add(
      this.camera.changed.subscribe(() => {
        this.requestRender();
        // Screen-sized symbols keep their size in px while zooming (the core draws their paper mm as px); what
        // they place along lines (marker spacing, offsets) is recompiled for the new zoom once it settles.
        if (this.ctx.prefs.symbolSize.value !== 'screen' || this.symbolScale() === this.builtSymbolScale) return;
        clearTimeout(this.symbolTimer);
        this.symbolTimer = window.setTimeout(() => {
          this.allDirty = true;
          this.requestRender();
        }, 150);
      }),
    );
    d.add(() => clearTimeout(this.symbolTimer));
    for (const s of [this.ctx.prefs.symbolSize, this.ctx.prefs.lineWeights])
      d.add(
        s.subscribe(() => {
          this.allDirty = true;
          this.requestRender();
        }),
      );
    d.add(settings.grid.subscribe(() => this.requestRender()));
    d.add(this.ctx.prefs.crosshair.subscribe(() => this.requestOverlay()));
    // A lock that changes with the pointer still shows at once (docs/adr/0166 §6).
    d.add(settings.locks.subscribe(() => this.repoint()));
    // A reference set, asked for or Yapım kipi: the cursor rule's base changed (docs/adr/0166 §5).
    d.add(tools.reference.subscribe(() => this.repoint()));
    for (const s of [tools.referenceWait, tools.construction]) d.add(s.subscribe(() => this.requestOverlay()));
    d.add(
      tools.lockPick.subscribe((pick) => {
        this.overlay.dataset.cursor = pick ? 'pick' : tools.active.cursor;
        this.requestOverlay();
      }),
    );
    d.add(
      tools.activeId.subscribe(() => {
        this.snap = null;
        this.snapOverride.set(null);
        // Tracking points belong to one command.
        this.acquired = [];
        this.track = null;
        this.clearDwell();
        this.overlay.dataset.cursor = tools.active.cursor;
        this.requestOverlay();
      }),
    );
    this.overlay.dataset.cursor = 'pick';
  }

  /**
   * The locks changed with the pointer still (a key in the value card, a chip's ×, the menu): the running tool sees
   * the pointer again, so its preview and its measure show the point the locks hold now (the desktop's `repoint`).
   */
  private repoint(): void {
    const screen = this.screenCursor;
    if (!screen || this.panFrom) return;
    this.updateSnap(screen);
    const raw = this.camera.screenToWorld(screen);
    const track = this.snap ? null : this.track;
    this.ctx.tools.active.pointerMove?.({ world: this.snap?.point ?? track?.point ?? raw, raw, screen, snap: this.snap, track, button: 0, ...this.moveKeys });
    this.requestOverlay();
  }

  private screenOf(e: PointerEvent | MouseEvent): Vec2 {
    const r = this.overlay.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  private pointer(e: PointerEvent | MouseEvent): ToolPointer {
    const screen = this.screenOf(e);
    const raw = this.camera.screenToWorld(screen);
    const track = this.snap ? null : this.track;
    return { world: this.snap?.point ?? track?.point ?? raw, raw, screen, snap: this.snap, track, button: e.button, shift: e.shiftKey, ctrl: e.ctrlKey || e.metaKey, alt: e.altKey };
  }

  private snapKinds(): Set<SnapKind> {
    const p = this.ctx.prefs;
    const kinds = new Set<SnapKind>();
    if (p.snapEndpoint.value) kinds.add('endpoint');
    if (p.snapMidpoint.value) kinds.add('midpoint');
    if (p.snapCenter.value) kinds.add('center');
    if (p.snapNode.value) kinds.add('node');
    if (p.snapEndpoint.value) kinds.add('quadrant');
    if (p.snapIntersection.value) kinds.add('intersection');
    if (p.snapPerpendicular.value) kinds.add('perpendicular');
    if (p.snapNearest.value) kinds.add('nearest');
    if (p.snapTangent.value) kinds.add('tangent');
    if (p.snapCentroid.value) kinds.add('centroid');
    if (p.snapExtension.value) kinds.add('extension');
    if (p.snapParallel.value) kinds.add('parallel');
    if (p.snapGrid.value) kinds.add('grid');
    return kinds;
  }

  /** Whether the view's screen scale lies in the snap's range (docs/adr/0163 §5). */
  private snapScaleOk(): boolean {
    const { snapScaleMin: min, snapScaleMax: max } = this.ctx.prefs;
    return snapInRange(screenScale(this.camera.scale), min.value, max.value);
  }

  private updateSnap(screen: Vec2): void {
    const tool = this.ctx.tools.active;
    const override = this.snapOverride.value;
    const prefs = this.ctx.prefs;
    // A one-shot snap works even with running snaps off (F3), and only for its own kind; neither out of the scale range.
    const on = tool.snaps && (this.ctx.settings.snap.value || !!override) && this.snapScaleOk();
    // A one-shot Uzantı snaps to ends too: an end is rested on to acquire its extension (docs/adr/0163 §2).
    const kinds = override ? new Set<SnapKind>(override === 'extension' ? ['extension', 'endpoint'] : [override]) : this.snapKinds();
    const extras = {
      // What rests acquired: ends' extensions and edges' directions (docs/adr/0163 §2).
      extensions: this.extensions(),
      parallels: this.parallels(),
      draft: prefs.snapSelf.value ? (tool.draftPath?.() ?? null) : null,
      grid: [prefs.snapGridEast.value, prefs.snapGridNorth.value] as const,
    };
    this.snap = on ? this.picker.snapEx(this.camera.screenToWorld(screen), prefs.snapAperture.value / this.camera.scale, kinds, tool.snapFrom?.() ?? null, extras) : null;
    this.updateTracking(screen, { tracking: this.ctx.settings.tracking.value, extension: on && kinds.has('extension'), parallel: on && kinds.has('parallel') });
  }

  // ── Object tracking and the snap additions' rests ──────────────────

  private acquired: Acquired[] = [];
  private track: TrackHit | null = null;
  /** What is being rested on; `done` once it has toggled, so resting longer does not toggle back. */
  private dwell: { rest: Rest; timer: number; done: boolean } | null = null;

  /**
   * After the snap: a rest on what the aids take begins (or goes on), and with no snap the cursor's lock is found
   * (docs/adr/0085, 0163 §2). Paralel rests on the straight edge under the cursor when no point is snapped there. The
   * desktop's is `ObjectTracking::update` through `Session::follow`.
   */
  private updateTracking(screen: Vec2, aids: Aids): void {
    const { settings, prefs } = this.ctx;
    const tool = this.ctx.tools.active;
    if (!tool.snaps || tool.id === 'select' || !(aids.tracking || aids.extension || aids.parallel)) {
      this.track = null;
      return this.clearDwell();
    }
    const s = this.snap;
    const raw = this.camera.screenToWorld(screen);
    const edge = aids.parallel && (!s || s.kind === 'nearest') ? this.picker.directionAt(raw, prefs.snapAperture.value / this.camera.scale) : null;
    let rest: Rest | null = null;
    if (s) {
      const track = aids.tracking && TRACKABLE.has(s.kind);
      // An end of the drawing's own objects (the path being drawn has none to give).
      const end = aids.extension && s.kind === 'endpoint' && s.entityId >= 0 ? s.entityId : null;
      if (track || end !== null) rest = { kind: 'point', p: s.point, track, end };
      else if (edge) rest = { kind: 'edge', at: raw, dir: edge };
    } else if (edge) rest = { kind: 'edge', at: raw, dir: edge };
    if (rest) this.restOn(rest);
    else this.clearDwell();
    const angles = trackAngles(settings.polar.value ? prefs.polarIncrement.value : null);
    this.track = !s && aids.tracking ? trackPoint(raw, this.trackPoints, tool.snapFrom?.() ?? null, angles, this.worldTolerance(TRACK_PX)) : null;
  }

  private restOn(rest: Rest): void {
    if (this.dwell && sameRest(this.dwell.rest, rest)) return;
    this.clearDwell();
    const dwell = { rest, timer: 0, done: false };
    dwell.timer = window.setTimeout(() => {
      dwell.done = true;
      this.dwellDue(rest);
    }, TRACK_DWELL_MS);
    this.dwell = dwell;
  }

  private clearDwell(): void {
    if (this.dwell) clearTimeout(this.dwell.timer);
    this.dwell = null;
  }

  /** The rest lasted: what was rested on is acquired, or released when it was. */
  private dwellDue(rest: Rest): void {
    const held = this.acquired.findIndex((a) =>
      rest.kind === 'point' ? a.kind === 'point' && a.at.x === rest.p.x && a.at.y === rest.p.y : a.kind === 'edge' && a.dir.x === rest.dir.x && a.dir.y === rest.dir.y,
    );
    if (held >= 0) this.acquired.splice(held, 1);
    else if (rest.kind === 'point') {
      const extensions = rest.end !== null ? this.picker.extensionsAt(rest.end, rest.p) : [];
      if (!rest.track && !extensions.length) return;
      this.acquire({ kind: 'point', at: rest.p, extensions });
    } else this.acquire({ kind: 'edge', at: rest.at, dir: rest.dir });
    this.requestOverlay();
  }

  private acquire(a: Acquired): void {
    this.acquired.push(a);
    if (this.acquired.length > MAX_TRACK_POINTS) this.acquired.shift();
  }

  /** The acquired ends' extensions, for the snap (Uzantı). */
  private extensions(): Extension[] {
    return this.acquired.flatMap((a) => (a.kind === 'point' ? a.extensions : []));
  }

  /** The acquired edges' directions, for the snap (Paralel). */
  private parallels(): Vec2[] {
    return this.acquired.flatMap((a) => (a.kind === 'edge' ? [a.dir] : []));
  }

  /** The acquired extensions `p` lies on, each with how far along from its end. */
  private extensionsThrough(p: Vec2): { x: Extension; d: number }[] {
    return this.extensions().flatMap((x) => {
      const d = extensionAlong(x, p);
      return d === null ? [] : [{ x, d }];
    });
  }

  /** The acquired direction whose line through `from` holds `p`. */
  private parallelThrough(p: Vec2, from: Vec2): Vec2 | null {
    return this.parallels().find((u) => Math.abs((p.x - from.x) * u.y - (p.y - from.y) * u.x) <= ON) ?? null;
  }

  /**
   * The point `distance` along what the cursor is on (a typed distance): the tracking lock's single line from its
   * origin, or the one extension snapped to from its end, or the parallel snapped to from the last point toward the
   * cursor (docs/adr/0163 §2); else null.
   */
  trackAlong(distance: number): Vec2 | null {
    if (this.track) return alongTrack(this.track, distance);
    const s = this.snap;
    if (s?.kind === 'extension') {
      const on = this.extensionsThrough(s.point);
      return on.length === 1 ? extensionAt(on[0].x, distance) : null;
    }
    if (s?.kind === 'parallel') {
      const from = this.ctx.tools.active.snapFrom?.() ?? null;
      const u = from && this.parallelThrough(s.point, from);
      if (!from || !u) return null;
      const side = (s.point.x - from.x) * u.x + (s.point.y - from.y) * u.y < 0 ? -1 : 1;
      return { x: from.x + u.x * side * distance, y: from.y + u.y * side * distance };
    }
    return null;
  }

  /** Acquired points, tracking points and ends (read-only view, for tests and the UI). */
  get trackPoints(): readonly Vec2[] {
    return this.acquired.flatMap((a) => (a.kind === 'point' ? [a.at] : []));
  }

  /**
   * The extensions or the parallel the snap lies on, as dashed guides, and what the snap marker says then: “Uzantı
   * 12.063 m”, “Uzantı: kesişim” (docs/adr/0163 §2). The desktop's is `App::snap_guides`.
   */
  private snapGuides(): { paths: Vec2[][]; lines: { through: Vec2; dir: Vec2 }[]; label: string | null } {
    const s = this.snap;
    const none = { paths: [], lines: [], label: null };
    if (s?.kind === 'extension') {
      const on = this.extensionsThrough(s.point);
      const label = on.length === 1 ? `Uzantı ${this.ctx.format.length(on[0].d)}` : on.length ? 'Uzantı: kesişim' : null;
      return { paths: on.map(({ x, d }) => guide(x, d)), lines: [], label };
    }
    if (s?.kind === 'parallel') {
      const from = this.ctx.tools.active.snapFrom?.() ?? null;
      const dir = from && this.parallelThrough(s.point, from);
      return { paths: [], lines: from && dir ? [{ through: from, dir }] : [], label: null };
    }
    return none;
  }

  /**
   * Right button: a quick click is Enter for a running command (the select
   * tool opens its menu); holding it opens the command menu; Shift opens
   * the one-shot snap menu at once.
   */
  private rightPress: { timer: number; opened: boolean } | null = null;

  private onRightDown(e: PointerEvent): void {
    const emit = (kind: ViewportMenuKind) =>
      this.events.emit('contextmenu', { clientX: e.clientX, clientY: e.clientY, world: this.pointer(e).raw, screen: this.screenOf(e), kind });
    if (e.shiftKey) {
      this.rightPress = null;
      return emit('snap');
    }
    const running = this.ctx.tools.activeId.value !== 'select';
    const press = { timer: 0, opened: false };
    press.timer = window.setTimeout(() => {
      press.opened = true;
      emit(running ? 'command' : 'select');
    }, RIGHT_HOLD_MS);
    this.rightPress = press;
  }

  private onRightUp(e: PointerEvent): void {
    const press = this.rightPress;
    this.rightPress = null;
    if (!press) return;
    clearTimeout(press.timer);
    if (press.opened) return;
    const tool = this.ctx.tools.active;
    if (tool.id !== 'select' && tool.confirm) return tool.confirm();
    this.events.emit('contextmenu', { clientX: e.clientX, clientY: e.clientY, world: this.pointer(e).raw, screen: this.screenOf(e), kind: 'select' });
  }

  private bindInput(): void {
    const el = this.overlay;
    const d = this.d;
    d.add(
      listen<PointerEvent>(el, 'pointerdown', (e) => {
        // We focus the canvas ourselves; the browser's own mousedown focus
        // would come later and steal focus from a field a tool just opened
        // (the text tool's in-place editor).
        e.preventDefault();
        el.focus({ preventScroll: true });
        el.setPointerCapture(e.pointerId);
        if (e.button === 1) {
          e.preventDefault();
          this.panFrom = { x: e.clientX, y: e.clientY };
          this.panKept = false;
          el.dataset.panning = '';
          return;
        }
        if (e.button === 2) return this.onRightDown(e);
        // Recompute the snap here: a click can arrive without a preceding move
        // (pen, touch, fast clicks), and a stale snap would place the point elsewhere.
        this.updateSnap(this.screenOf(e));
        // An edge awaited for a lock takes the press (docs/adr/0166 §3).
        if (e.button === 0 && this.ctx.tools.pickPress(this.pointer(e))) return;
        this.ctx.tools.active.pointerDown?.(this.pointer(e));
        if (e.button === 0 && this.snapOverride.value) this.snapOverride.set(null);
      }),
    );
    d.add(
      listen<PointerEvent>(el, 'pointermove', (e) => {
        const t0 = import.meta.env.DEV ? performance.now() : 0;
        if (this.panFrom) {
          // The start of a pan keeps the view it leaves: at its first move, so a middle click that goes nowhere keeps nothing.
          if (!this.panKept) {
            this.panKept = true;
            this.navigation.remember();
          }
          this.camera.panBy(e.clientX - this.panFrom.x, e.clientY - this.panFrom.y);
          this.panFrom = { x: e.clientX, y: e.clientY };
        }
        const r = el.getBoundingClientRect();
        this.screenCursor = { x: e.clientX - r.left, y: e.clientY - r.top };
        this.moveKeys = { shift: e.shiftKey, ctrl: e.ctrlKey || e.metaKey, alt: e.altKey };
        const s0 = import.meta.env.DEV ? performance.now() : 0;
        this.updateSnap(this.screenCursor);
        const s1 = import.meta.env.DEV ? performance.now() : 0;
        const p = this.pointer(e);
        this.cursorWorld.set(p.world);
        // The magnifier looks where the pointer is, its own place, not the snap's (docs/adr/0181 §2).
        if (this.nav?.pointer(this.screenCursor, this.camera.screenToWorld(this.screenCursor))) this.requestLens();
        const m0 = import.meta.env.DEV ? performance.now() : 0;
        if (!this.panFrom) this.ctx.tools.active.pointerMove?.(p);
        const m1 = import.meta.env.DEV ? performance.now() : 0;
        this.requestOverlay();
        if (import.meta.env.DEV && this.probe) this.probe.moves.push({ snap: s1 - s0, tool: m1 - m0, total: performance.now() - t0 });
      }),
    );
    d.add(
      listen<PointerEvent>(el, 'pointerup', (e) => {
        if (e.button === 1 && this.panFrom) {
          this.panFrom = null;
          delete el.dataset.panning;
          return;
        }
        if (e.button === 2) return this.onRightUp(e);
        if (e.button !== 0) return;
        if (this.ctx.tools.pickRelease()) return;
        this.updateSnap(this.screenOf(e));
        this.ctx.tools.active.pointerUp?.(this.pointer(e));
      }),
    );
    d.add(
      listen<PointerEvent>(el, 'pointerleave', () => {
        this.screenCursor = null;
        this.snap = null;
        this.track = null;
        this.clearDwell();
        this.cursorWorld.set(null);
        if (this.ctx.tools.activeId.value === 'select') this.ctx.selection.hover.set(null);
        this.requestOverlay();
      }),
    );
    d.add(
      listen<WheelEvent>(
        el,
        'wheel',
        (e) => {
          e.preventDefault();
          const step = e.deltaMode === 1 ? e.deltaY * 0.05 : e.deltaY * 0.0015;
          const r = el.getBoundingClientRect();
          // The first step after a pause keeps the view it leaves; a run of steps is one pass (docs/adr/0141).
          this.navigation.wheel();
          this.camera.zoomAt(Math.exp(-step), { x: e.clientX - r.left, y: e.clientY - r.top });
        },
        { passive: false },
      ),
    );
    d.add(
      listen<MouseEvent>(el, 'auxclick', (e) => {
        if (e.button === 1 && e.detail === 2) this.zoomExtents();
      }),
    );
    d.add(
      // The right button is handled on press/release (see onRightDown); the
      // browser menu never shows. Its timing differs per OS (down vs up).
      listen<MouseEvent>(el, 'contextmenu', (e) => e.preventDefault()),
    );
  }

  private resize(): void {
    const w = this.host.clientWidth;
    const h = this.host.clientHeight;
    // HiDPI (Uygulama ayarları → Çizim motoru) is the drawing's: the screen's pixel ratio, or one pixel
    // per CSS pixel. The overlay above it is the interface and always has the screen's.
    const screen = window.devicePixelRatio || 1;
    const dpr = this.ctx.prefs.hiDpi.value ? screen : 1;
    if (w === this.size.w && h === this.size.h && dpr === this.dpr && screen === this.overlayDpr) return;
    this.size = { w, h };
    this.dpr = dpr;
    this.overlayDpr = screen;
    this.overlay.width = Math.round(w * screen);
    this.overlay.height = Math.round(h * screen);
    this.backend?.resize(w, h, dpr);
    this.camera.setSize(w, h);
    this.nav?.layout(w, h);
    // Resizing a canvas clears it. Waiting for the next animation frame would
    // let the browser paint the empty (black) buffer in between — visible as
    // flashing while a panel splitter is dragged. ResizeObserver callbacks run
    // after layout and before paint, so drawing here keeps every frame filled.
    this.glQueued = true;
    this.baseDirty = true;
    this.frame();
  }

  private size = { w: -1, h: -1 };

  // ── Frame ───────────────────────────────────────────────────────────

  private schedule(): void {
    if (this.frameQueued) return;
    this.frameQueued = true;
    requestAnimationFrame(() => this.frame());
  }

  // ── Construction lines (xline/ray) ──────────────────────────────────

  private constructionLayers = new Set<string>();
  private clipBox: Bounds | null = null;
  private clipScale = 0;

  /**
   * World box infinite lines are clipped to: three view sizes around the
   * camera, renewed when the view leaves it or the zoom changes 2× or more.
   */
  private constructionClip(): Bounds {
    const v = this.camera.visibleBounds();
    const c = this.clipBox;
    const s = this.camera.scale;
    if (!c || v.minX < c.minX || v.maxX > c.maxX || v.minY < c.minY || v.maxY > c.maxY || s > this.clipScale * 2 || s < this.clipScale / 2) {
      const w = v.maxX - v.minX;
      const h = v.maxY - v.minY;
      this.clipBox = { minX: v.minX - w, minY: v.minY - h, maxX: v.maxX + w, maxY: v.maxY + h };
      this.clipScale = s;
    }
    return this.clipBox!;
  }

  /** Rebuild layers (and highlights) holding construction lines when their clip box is stale. */
  private refreshConstructionClip(): void {
    const sel = [...this.ctx.selection.ids.value, this.ctx.selection.hover.value].some((id) => {
      const e = id === null ? undefined : this.ctx.doc.get(id);
      return !!e && isConstruction(e);
    });
    if (!this.constructionLayers.size && !sel) return;
    const before = this.clipBox;
    if (this.constructionClip() === before) return;
    this.constructionLayers.forEach((id) => this.dirtyLayers.add(id));
    this.highlightDirty = true;
  }

  private frame(): void {
    this.frameQueued = false;
    const t0 = performance.now();
    let t1 = t0;
    let t2 = t0;
    if (this.backend && this.glQueued) {
      this.glQueued = false;
      this.refreshConstructionClip();
      this.syncLayers();
      t1 = performance.now();
      this.renderGl();
      t2 = performance.now();
    }
    this.drawOverlay();
    this.nav?.drawOverview();
    const t3 = performance.now();
    this.stats = { build: t1 - t0, render: t2 - t1, overlay: t3 - t2 };
    if (import.meta.env.DEV && this.probe) this.probe.frames.push({ at: t0, ...this.stats, ...this.probe.overlay });
  }

  /**
   * The scale paper-mm symbol sizes are compiled at: the project's plot
   * scale, or with screen-sized symbols the view's own scale, in quarter
   * octave steps so a zoom does not rebuild every layer at every wheel tick.
   */
  private symbolScale(): number {
    return symbolScaleOf(this.ctx.prefs.symbolSize.value, this.ctx.doc.settings.plotScale.value, this.camera.scale);
  }

  private syncLayers(): void {
    const { doc } = this.ctx;
    const backend = this.backend!;
    // A layer gone from the tree (Katmanlar → Sil) is among the dirty ones even when all are: its buffers go too.
    const ids = this.allDirty ? [...new Set([...doc.layers.leaves().map((l) => l.id), ...this.dirtyLayers])] : [...this.dirtyLayers];
    this.allDirty = false;
    this.dirtyLayers.clear();
    // What the drawing shows changed: Genel bakış draws its picture again a moment later (docs/adr/0181 §1).
    if (ids.length) this.nav?.invalidate();
    const plotScale = this.symbolScale();
    this.builtSymbolScale = plotScale;
    const style = {
      origin: doc.origin,
      palette: this.palette,
      plotScale,
      screen: this.ctx.prefs.symbolSize.value === 'screen',
      hairlines: !this.ctx.prefs.lineWeights.value,
      library: this.ctx.styles.library,
      layerName: (id: string) => doc.layers.get(id)?.name ?? id,
      geometry: this.picker,
      clip: this.constructionClip(),
    };
    for (const id of ids) {
      const node = doc.layers.get(id);
      if (!node || node.type !== 'layer') {
        backend.remove(id);
        this.constructionLayers.delete(id);
        continue;
      }
      const list = doc.byLayer(id);
      if (list.some(isConstruction)) this.constructionLayers.add(id);
      else this.constructionLayers.delete(id);
      backend.upload(buildStyledLayer(id, list, node.style, style));
    }
    if (this.highlightDirty) {
      this.highlightDirty = false;
      this.uploadHighlight();
    }
  }

  private uploadHighlight(): void {
    const { doc, selection } = this.ctx;
    const accent = parseHex(this.palette.accent);
    const base = { color: 'fg', lineType: 'continuous' as const, lineWeight: 0.25 };
    const sel = [...selection.ids.value].map((id) => doc.get(id)).filter((e): e is Entity => !!e);
    this.backend!.upload(
      buildSceneLayer('__sel', sel, base, {
        origin: doc.origin,
        palette: this.palette,
        geometry: this.picker,
        overrideColor: accent,
        overrideFill: withAlpha(accent, 0.13),
        overrideDash: [6, 3],
        pointStyle: { size: 15, shape: 'ring' },
        clip: this.constructionClip(),
      }),
    );
    const hoverId = selection.hover.value;
    const hover = hoverId !== null && !selection.has(hoverId) ? doc.get(hoverId) : undefined;
    this.backend!.upload(
      buildSceneLayer('__hover', hover ? [hover] : [], base, {
        origin: doc.origin,
        palette: this.palette,
        geometry: this.picker,
        overrideColor: withAlpha(accent, 0.85),
        // Hover is outline-only: a fill flickers across large areas as the cursor moves.
        overrideFill: null,
        overrideDash: null,
        pointStyle: { size: 15, shape: 'ring' },
        clip: this.constructionClip(),
      }),
    );
  }

  private renderGl(): void {
    const { doc, settings } = this.ctx;
    const origin = doc.origin;
    const cam = this.camera;
    const view = {
      center: { x: cam.center.x - origin.x, y: cam.center.y - origin.y },
      scale: cam.scale,
      width: cam.width,
      height: cam.height,
      dpr: this.dpr,
    };
    const showGrid = settings.grid.value;
    if (showGrid) {
      const grid = gridExtent(cam.visibleBounds(), cam.scale, origin, this.palette, this.grid);
      if (grid !== this.grid) this.backend!.upload(buildGrid(grid));
      this.grid = grid;
    } else if (this.grid) {
      this.backend!.remove('__grid');
      this.grid = null;
    }
    const order = doc.layers
      .leaves()
      .filter((l) => doc.layers.isVisible(l.id))
      .map((l) => l.id)
      .reverse();
    this.nav?.order(order.join());
    // The magnifier keeps looking under the pointer when the view moves beneath it (a wheel zoom).
    if (this.screenCursor) this.nav?.pointer(null, cam.screenToWorld(this.screenCursor));
    this.backend!.render({
      view,
      // 1:N on a 96 dpi screen (as in the status bar); rule scale ranges use it.
      scaleDenominator: 1 / (cam.scale * 0.00026458),
      clearColor: this.palette.background,
      order,
      underlays: showGrid ? ['__grid'] : [],
      overlays: ['__hover', '__sel'],
      keepBase: !this.baseDirty,
      lens: this.nav?.lens(origin) ?? null,
    });
    this.baseDirty = false;
  }

  /**
   * The labels, as a picture of the view and a margin around it (LABEL_MARGIN of its size on each side):
   * a pointer move copies the picture, a pan that stays inside the margin copies it shifted (a pan used to
   * draw every label again in every frame), anything else (a zoom, a pan past the margin, a change of the
   * drawing or of the labels' look, a new size) draws the picture again. After a pan the view rests on a
   * whole-pixel shift of the picture, so the letters stay sharp; a fractional one is drawn again.
   */
  private drawCachedLabels(g: CanvasRenderingContext2D): void {
    const cam = this.camera;
    const dpr = this.dpr;
    const mx = Math.ceil(cam.width * LABEL_MARGIN);
    const my = Math.ceil(cam.height * LABEL_MARGIN);
    const w = Math.max(1, Math.round((cam.width + 2 * mx) * dpr));
    const h = Math.max(1, Math.round((cam.height + 2 * my) * dpr));
    const key = `${w}x${h}|${dpr}|${cam.scale}|${this.labelEpoch}|${this.ctx.doc.revision}`;
    const moving = cam.changed.value !== this.labelCamera;
    this.labelCamera = cam.changed.value;
    let cache = this.labelCache;
    if (!cache) cache = this.labelCache = { canvas: document.createElement('canvas'), key: '', center: { x: 0, y: 0 } };
    // Where the picture's top left corner falls on the screen, in device pixels.
    let sx = (-mx + (cache.center.x - cam.center.x) * cam.scale) * dpr;
    let sy = (-my - (cache.center.y - cam.center.y) * cam.scale) * dpr;
    const inside = sx <= 0 && sy <= 0 && sx >= -2 * mx * dpr && sy >= -2 * my * dpr;
    const whole = Math.abs(sx - Math.round(sx)) < 1e-3 && Math.abs(sy - Math.round(sy)) < 1e-3;
    if (cache.key !== key || !inside || (!moving && !whole)) {
      if (cache.canvas.width !== w || cache.canvas.height !== h) [cache.canvas.width, cache.canvas.height] = [w, h];
      const view = this.labelView;
      view.center = { ...cam.center };
      view.scale = cam.scale;
      view.width = cam.width + 2 * mx;
      view.height = cam.height + 2 * my;
      const lg = cache.canvas.getContext('2d')!;
      lg.setTransform(dpr, 0, 0, dpr, 0, 0);
      lg.clearRect(0, 0, view.width, view.height);
      drawLabels(lg, this.ctx.doc, view, this.palette, this.picker.labels(view.visibleBounds(), view.scale, this.editingId), (l, look) => this.dimensionText(l, look), (b) => this.picker.blockPieces(b));
      cache.key = key;
      cache.center = view.center;
      sx = -mx * dpr;
      sy = -my * dpr;
    }
    // The labels are the drawing's text: drawn at the drawing's pixel ratio, and scaled onto the overlay
    // when HiDPI is off (then they are as soft as the drawing, the rest of the overlay stays sharp).
    const x = moving ? sx : Math.round(sx);
    const y = moving ? sy : Math.round(sy);
    const k = this.overlayDpr / dpr;
    g.save();
    g.setTransform(1, 0, 0, 1, 0, 0);
    if (k === 1) g.drawImage(cache.canvas, x, y);
    else g.drawImage(cache.canvas, x * k, y * k, cache.canvas.width * k, cache.canvas.height * k);
    g.restore();
  }

  private drawOverlay(): void {
    const g = this.g;
    const cam = this.camera;
    g.setTransform(this.overlayDpr, 0, 0, this.overlayDpr, 0, 0);
    g.clearRect(0, 0, cam.width, cam.height);
    // Büyüteç (docs/adr/0181 §5): its rectangle is the lens's own; the rest of the overlay stays out of it.
    const lens = this.nav?.lensCamera() ?? null;
    g.save();
    if (lens) {
      const [x, y, w, h] = lens.rect;
      g.beginPath();
      g.rect(0, 0, cam.width, cam.height);
      g.rect(x, y, w, h);
      g.clip('evenodd');
    }
    this.drawMainOverlay(g);
    g.restore();
    if (lens) this.drawLensOverlay(g, lens.rect, lens.camera);
  }

  /** The drawing's text and marks in the magnifier, through its camera: no picture kept, its rectangle is small. */
  private drawLensOverlay(g: CanvasRenderingContext2D, rect: readonly [number, number, number, number], lens: Camera): void {
    const pal = this.palette;
    const [x, y, w, h] = rect;
    g.save();
    g.beginPath();
    g.rect(x, y, w, h);
    g.clip();
    g.translate(x, y);
    drawLabels(g, this.ctx.doc, lens, pal, this.picker.labels(lens.visibleBounds(), lens.scale, this.editingId), (l, look) => this.dimensionText(l, look), (b) => this.picker.blockPieces(b));
    if (this.paragraphPreview) paragraphRecords(g, pal, lens, this.paragraphPreview.records, this.paragraphPreview.text, this.paragraphPreview.runs, this.paragraphPreview.face);
    const selected = this.ctx.selection.ids.value;
    if (selected.size <= 150) drawGrips(g, this.picker.grips(selected), lens, pal, this.ctx.tools.active.activeGrip?.() ?? null);
    drawMarkedVertices(g, this.ctx.selection.vertices.value, lens, pal);
    this.ctx.tools.active.draw?.(g, lens);
    if (this.snap) drawSnap(g, this.snap, lens, pal);
    // The pointer's place: a small cross in the middle, arms of 9 px kept 3 px clear of it (the desktop's too).
    const [cx, cy] = [Math.round(w / 2) + 0.5, Math.round(h / 2) + 0.5];
    g.strokeStyle = pal.fg;
    g.globalAlpha = 0.85;
    g.lineWidth = 1;
    g.beginPath();
    for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
      g.moveTo(cx + 3 * dx, cy + 3 * dy);
      g.lineTo(cx + 12 * dx, cy + 12 * dy);
    }
    g.stroke();
    g.restore();
  }

  /** The overlay of the drawing area through its own camera: text, grips, the tool's draft, snaps, marks, the cross-hair. */
  private drawMainOverlay(g: CanvasRenderingContext2D): void {
    const cam = this.camera;
    const pal = this.palette;
    const l0 = import.meta.env.DEV ? performance.now() : 0;
    this.drawCachedLabels(g);
    if (this.paragraphPreview) paragraphRecords(g, pal, cam, this.paragraphPreview.records, this.paragraphPreview.text, this.paragraphPreview.runs, this.paragraphPreview.face);
    const l1 = import.meta.env.DEV ? performance.now() : 0;
    const selected = this.ctx.selection.ids.value;
    if (selected.size <= 150) drawGrips(g, this.picker.grips(selected), cam, pal, this.ctx.tools.active.activeGrip?.() ?? null);
    drawMarkedVertices(g, this.ctx.selection.vertices.value, cam, pal);
    const mark = this.ctx.selection.mark.value;
    if (mark) drawSearchMark(g, mark, this.ctx.format.point(mark), cam, pal);
    const d0 = import.meta.env.DEV ? performance.now() : 0;
    this.ctx.tools.active.draw?.(g, cam);
    // The digitizing locks over the tool's preview (docs/adr/0166 §6).
    this.ctx.tools.drawLocks(g, cam, pal, this.panFrom ? null : this.screenCursor);
    if (import.meta.env.DEV && this.probe) this.probe.overlay = { labels: l1 - l0, tool: performance.now() - d0 };
    // Tracking and the snap additions' guides under the snap marker, as the desktop draws them.
    const guides = this.snapGuides();
    const edges = this.acquired.flatMap((a) => (a.kind === 'edge' ? [{ at: a.at, dir: a.dir }] : []));
    drawObjectTracking(g, { points: this.trackPoints, edges, paths: guides.paths, lines: guides.lines }, this.snap ? null : this.track, cam, pal, (m) => this.ctx.format.length(m));
    if (this.snap) drawSnap(g, this.snap, cam, pal, guides.label ?? undefined);
    // A CBS project's grid north and scale bar, a CAD project's coordinate axes (docs/adr/0165 §5).
    if (this.ctx.format.axes === 'cad') drawUcsIcon(g, cam, pal);
    else {
      drawNorthArrow(g, cam, pal);
      drawScaleBar(g, cam, pal);
    }
    // An edge awaited for a lock is picked, whatever the tool's look (docs/adr/0166 §3).
    const look = this.ctx.tools.lockPick.value ? 'pick' : this.ctx.tools.active.cursor;
    if (this.screenCursor && !this.panFrom) drawCrosshair(g, this.screenCursor, look, pal, this.ctx.prefs.crosshair.value);
  }
}

/** An extension from its end to `d` along it, to draw: the segment, or an arc's points at most 5° apart. */
function guide(x: Extension, d: number): Vec2[] {
  if (x.kind === 'line') return [x.end, { x: x.end.x + x.dir.x * d, y: x.end.y + x.dir.y * d }];
  const turn = d / x.r;
  const steps = Math.max(1, Math.ceil(turn / ((5 * Math.PI) / 180)));
  return Array.from({ length: steps + 1 }, (_, i) => {
    const a = x.a0 + Math.sign(x.sweep) * turn * (i / steps);
    return { x: x.c.x + x.r * Math.cos(a), y: x.c.y + x.r * Math.sin(a) };
  });
}
