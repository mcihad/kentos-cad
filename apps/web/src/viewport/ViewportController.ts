import type { AppContext } from '../app/context';
import { DisposableStore, listen } from '../core/disposable';
import { Emitter } from '../core/emitter';
import { Signal } from '../core/signal';
import type { Entity } from '../model/entities';
import { dimensionLabel, type DimensionLayout } from '../model/geom/dimension';
import type { Affine } from '../model/geom/affine';
import type { Edge } from '../model/geom/intersect';
import type { ExtendResult, TrimResult } from '../model/ops/trim';
import type { Bounds, Vec2 } from '../model/geometry';
import { parseHex, readCanvasPalette, withAlpha, type CanvasPalette } from '../render/color';
import { createBackend } from '../render/createBackend';
import { buildGrid, gridExtent, type GridExtent } from '../render/grid';
import { Atlas } from '../render/atlas';
import { buildSceneLayer } from '../render/sceneBuilder';
import { buildStyledLayer } from '../render/styledLayer';
import type { BackendKind, RenderBackend } from '../render/types';
import { webgpuSupported } from '../render/webgpu/support';
import type { ToolPointer } from '../tools/Tool';
import { Camera } from './Camera';
import { drawCrosshair, drawGrips, drawLabels, drawNorthArrow, drawObjectTracking, drawScaleBar, drawSnap, midGripVisible } from './overlay';
import { alongTrack, trackAngles, trackPoint, type TrackHit } from './objectTracking';
import { PickIndex, type SnapHit, type SnapKind } from './picking';

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
  commit(text: string): void;
  cancel(): void;
}

/** Holding the right button this long opens the command menu instead of confirming. */
/** The label picture reaches this share of the view beyond each edge, so a pan copies it instead of drawing the labels. */
const LABEL_MARGIN = 0.35;
const RIGHT_HOLD_MS = 300;
/** Resting on a snap this long acquires (or releases) it as a tracking point. */
const TRACK_DWELL_MS = 350;
const MAX_TRACK_POINTS = 3;
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
}

/**
 * Owns the GPU canvas, the overlay canvas and the camera. Translates DOM
 * input into tool calls and keeps GPU buffers in sync with the document.
 */
export class ViewportController {
  readonly camera = new Camera();
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
  /** Entity whose text is being edited inline (hidden from the overlay). */
  private editingId: number | null = null;
  private snap: SnapHit | null = null;
  private panFrom: Vec2 | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    this.picker = new PickIndex(ctx.doc);
    this.palette = readCanvasPalette();
  }

  async mount(host: HTMLElement): Promise<void> {
    this.host = host;
    this.overlay = document.createElement('canvas');
    this.overlay.className = 'viewport__overlay';
    this.overlay.tabIndex = 0;
    this.overlay.setAttribute('aria-label', 'Çizim alanı');
    host.append(this.overlay);
    this.g = this.overlay.getContext('2d')!;

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
    else this.zoomExtents();
    document.fonts?.ready.then(() => this.requestOverlay());
  }

  dispose(): void {
    this.d.dispose();
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

  /** The object snap the marker shows now, if any (the interaction traces read it; docs/adr/0029). */
  get currentSnap(): SnapHit | null {
    return this.snap;
  }

  pick(screen: Vec2): Entity | null {
    return this.picker.hit(this.camera.screenToWorld(screen), this.ctx.prefs.pickAperture.value / this.camera.scale);
  }

  pickRect(r: Bounds, crossing: boolean): number[] {
    return this.picker.inRect(r, crossing);
  }

  /** Edge-only pick for modify tools (ignores polygon interiors, points and text). */
  pickEdge(screen: Vec2, filter?: (e: Entity) => boolean): Entity | null {
    return this.picker.hitEdge(this.camera.screenToWorld(screen), this.ctx.prefs.pickAperture.value / this.camera.scale, filter);
  }

  /** Smallest closed shape around a world point (hatch boundary). */
  enclosingRing(world: Vec2): { entity: Entity; ring: Vec2[] } | null {
    return this.picker.enclosing(world);
  }

  /** A dimension's measured value as drawn: prefix and value in project units (length without unit). */
  dimensionText(l: Pick<DimensionLayout, 'prefix' | 'unit' | 'value'>): string {
    const f = this.ctx.format;
    return dimensionLabel(undefined, l, { length: (m) => f.length(m, false), angle: (a) => f.angle(a) });
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
    let bestD = 6; // px
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

  zoomExtents(): void {
    const b = this.picker.extent();
    if (b) this.camera.fit(b);
  }

  zoomToSelection(): void {
    const b = this.picker.extent(this.ctx.selection.ids.value);
    if (b) this.camera.fit(b, 96);
  }

  /** The box around these objects (all of them without `ids`), from the geometry store; null when empty. */
  extent(ids?: Iterable<number>): Bounds | null {
    return this.picker.extent(ids);
  }

  zoomBy(factor: number): void {
    this.camera.zoomAt(factor, { x: this.camera.width / 2, y: this.camera.height / 2 });
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
    d.add(doc.events.on('attrs', stale));
    d.add(doc.layers.events.on('state', stale));
    d.add(doc.layers.events.on('structure', stale));
    d.add(
      doc.events.on('changed', ({ layerIds }) => {
        layerIds.forEach((id) => this.dirtyLayers.add(id));
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
    return kinds;
  }

  private updateSnap(screen: Vec2): void {
    const tool = this.ctx.tools.active;
    const override = this.snapOverride.value;
    // A one-shot snap works even with running snaps off (F3), and only for its own kind.
    const on = tool.snaps && (this.ctx.settings.snap.value || !!override);
    const kinds = override ? new Set<SnapKind>([override]) : this.snapKinds();
    this.snap = on ? this.picker.snap(this.camera.screenToWorld(screen), this.ctx.prefs.snapAperture.value / this.camera.scale, kinds, tool.snapFrom?.() ?? null) : null;
    this.updateTracking(screen);
  }

  // ── Object tracking ─────────────────────────────────────────────────

  private acquired: Vec2[] = [];
  private track: TrackHit | null = null;
  /** Snap point being rested on; `done` once it has toggled, so resting longer does not toggle back. */
  private dwell: { x: number; y: number; timer: number; done: boolean } | null = null;

  private updateTracking(screen: Vec2): void {
    const { settings, prefs } = this.ctx;
    const tool = this.ctx.tools.active;
    if (!tool.snaps || tool.id === 'select' || !settings.tracking.value) {
      this.track = null;
      return this.clearDwell();
    }
    const s = this.snap;
    if (s && TRACKABLE.has(s.kind)) this.restOn(s.point);
    else this.clearDwell();
    const angles = trackAngles(settings.polar.value ? prefs.polarIncrement.value : null);
    this.track = s ? null : trackPoint(this.camera.screenToWorld(screen), this.acquired, tool.snapFrom?.() ?? null, angles, this.worldTolerance(TRACK_PX));
  }

  private restOn(p: Vec2): void {
    if (this.dwell && this.dwell.x === p.x && this.dwell.y === p.y) return;
    this.clearDwell();
    const dwell = { x: p.x, y: p.y, timer: 0, done: false };
    dwell.timer = window.setTimeout(() => {
      dwell.done = true;
      this.toggleTrackPoint(p);
    }, TRACK_DWELL_MS);
    this.dwell = dwell;
  }

  private clearDwell(): void {
    if (this.dwell) clearTimeout(this.dwell.timer);
    this.dwell = null;
  }

  private toggleTrackPoint(p: Vec2): void {
    const i = this.acquired.findIndex((q) => q.x === p.x && q.y === p.y);
    if (i >= 0) this.acquired.splice(i, 1);
    else {
      this.acquired.push(p);
      if (this.acquired.length > MAX_TRACK_POINTS) this.acquired.shift();
    }
    this.requestOverlay();
  }

  /** Point `distance` along the active tracking line (typed distance while tracking), else null. */
  trackAlong(distance: number): Vec2 | null {
    return this.track ? alongTrack(this.track, distance) : null;
  }

  /** Acquired tracking points (read-only view, for tests and the UI). */
  get trackPoints(): readonly Vec2[] {
    return this.acquired;
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
          el.dataset.panning = '';
          return;
        }
        if (e.button === 2) return this.onRightDown(e);
        // Recompute the snap here: a click can arrive without a preceding move
        // (pen, touch, fast clicks), and a stale snap would place the point elsewhere.
        this.updateSnap(this.screenOf(e));
        this.ctx.tools.active.pointerDown?.(this.pointer(e));
        if (e.button === 0 && this.snapOverride.value) this.snapOverride.set(null);
      }),
    );
    d.add(
      listen<PointerEvent>(el, 'pointermove', (e) => {
        const t0 = import.meta.env.DEV ? performance.now() : 0;
        if (this.panFrom) {
          this.camera.panBy(e.clientX - this.panFrom.x, e.clientY - this.panFrom.y);
          this.panFrom = { x: e.clientX, y: e.clientY };
        }
        const r = el.getBoundingClientRect();
        this.screenCursor = { x: e.clientX - r.left, y: e.clientY - r.top };
        const s0 = import.meta.env.DEV ? performance.now() : 0;
        this.updateSnap(this.screenCursor);
        const s1 = import.meta.env.DEV ? performance.now() : 0;
        const p = this.pointer(e);
        this.cursorWorld.set(p.world);
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
    if (this.ctx.prefs.symbolSize.value !== 'screen') return this.ctx.doc.settings.plotScale.value;
    const denominator = 1 / (this.camera.scale * 0.00026458);
    return 2 ** (Math.round(Math.log2(Math.max(denominator, 1)) * 4) / 4);
  }

  private syncLayers(): void {
    const { doc } = this.ctx;
    const backend = this.backend!;
    const ids = this.allDirty ? doc.layers.leaves().map((l) => l.id) : [...this.dirtyLayers];
    this.allDirty = false;
    this.dirtyLayers.clear();
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
    this.backend!.render({
      view,
      // 1:N on a 96 dpi screen (as in the status bar); rule scale ranges use it.
      scaleDenominator: 1 / (cam.scale * 0.00026458),
      clearColor: this.palette.background,
      order,
      underlays: showGrid ? ['__grid'] : [],
      overlays: ['__hover', '__sel'],
      keepBase: !this.baseDirty,
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
      drawLabels(lg, this.ctx.doc, view, this.palette, this.picker.labels(view.visibleBounds(), view.scale, this.editingId), (l) => this.dimensionText(l));
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
    const pal = this.palette;
    g.setTransform(this.overlayDpr, 0, 0, this.overlayDpr, 0, 0);
    g.clearRect(0, 0, cam.width, cam.height);
    const l0 = import.meta.env.DEV ? performance.now() : 0;
    this.drawCachedLabels(g);
    const l1 = import.meta.env.DEV ? performance.now() : 0;
    const selected = this.ctx.selection.ids.value;
    if (selected.size <= 150) drawGrips(g, this.picker.grips(selected), cam, pal, this.ctx.tools.active.activeGrip?.() ?? null);
    const d0 = import.meta.env.DEV ? performance.now() : 0;
    this.ctx.tools.active.draw?.(g, cam);
    if (import.meta.env.DEV && this.probe) this.probe.overlay = { labels: l1 - l0, tool: performance.now() - d0 };
    if (this.snap) drawSnap(g, this.snap, cam, pal);
    drawObjectTracking(g, this.acquired, this.snap ? null : this.track, cam, pal, (m) => this.ctx.format.length(m));
    drawNorthArrow(g, cam, pal);
    drawScaleBar(g, cam, pal);
    if (this.screenCursor && !this.panFrom) drawCrosshair(g, this.screenCursor, this.ctx.tools.active.cursor, pal, this.ctx.prefs.crosshair.value);
  }
}
