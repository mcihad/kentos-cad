import type { AppContext } from '../app/context';
import { DisposableStore, listen } from '../core/disposable';
import type { Bounds, Vec2 } from '../model/geometry';
import { parseHex, resolveColor, type CanvasPalette } from '../render/color';
import type { LensFrame } from '../render/types';
import { Camera } from './Camera';
import { cards, fit, nextSide, OVERVIEW, toWorld, viewFrame, ZOOMS, type Cards, type Fit, type Side, type Zoom } from './navigation';
import type { PickIndex } from './picking';
import type { ViewNavigation } from './viewHistory';

/**
 * Genel bakış and Büyüteç over the drawing (docs/adr/0181; the desktop's `navigation_cards.rs`): two cards at the
 * places `navigation.ts` gives them. The overview shows the core's picture of the whole drawing
 * (`PickIndex.overviewPicture`), drawn again 250 ms after the drawing, its layers or the palette change, never for a
 * view change, and the view's frame over it; a press moves the view there, a drag pans, the wheel zooms, a double
 * click shows everything. The magnifier is a frame and a title row over the rectangle the drawing backend draws again
 * through a second camera (`lens`), and the 2D overlay draws the drawing's text and marks in it (`lensCamera`).
 */

/** How long after a change the overview's picture is drawn again. */
const REDRAW_MS = 250;
/** The title row's height, CSS px (times the interface's scale, rounded). */
const HEADER = 24;

/** What the cards need of the drawing area. */
export interface NavigationHost {
  readonly ctx: AppContext;
  readonly camera: Camera;
  readonly navigation: ViewNavigation;
  readonly picker: PickIndex;
  palette(): CanvasPalette;
  /** The drawing's pixel ratio. */
  dpr(): number;
  /** The drawing is to be drawn again (the lens moved, its zoom or side changed). */
  requestRender(): void;
  zoomExtents(): void;
}

const hex = (c: string): string => {
  const [r, g, b] = parseHex(c);
  return `#${[r, g, b].map((v) => Math.round(v * 255).toString(16).padStart(2, '0')).join('')}`;
};

export class NavigationCards {
  private readonly d = new DisposableStore();
  private readonly overviewEl: HTMLElement;
  private readonly picture: HTMLCanvasElement;
  private readonly empty: HTMLElement;
  private readonly lensEl: HTMLElement;
  /** The magnifier's window says where it looks before the pointer has been on the drawing. */
  private readonly hint: HTMLElement;
  private readonly zoomButtons = new Map<Zoom, HTMLButtonElement>();
  private laid: Cards | null = null;
  private area = { width: 0, height: 0 };
  private side: Side = 'right';
  /** Where the magnifier looks: the pointer's last place on the drawing. */
  private at: Vec2 | null = null;
  /** The overview's picture, and the fit and extent it was drawn at; null for an empty drawing. */
  private shot: { canvas: HTMLCanvasElement; fit: Fit; extent: Bounds } | null = null;
  private timer = 0;
  private pressed = false;
  /** The visible layers' order the picture was drawn with: hiding a layer changes no layer's buffers. */
  private orderKey = '';

  private readonly host: NavigationHost;

  constructor(parent: HTMLElement, host: NavigationHost) {
    this.host = host;
    const { ui } = host.ctx;
    const head = (title: string, close: () => void, ...extra: HTMLElement[]) => {
      const el = document.createElement('div');
      el.className = 'nav-card__head';
      const name = document.createElement('span');
      name.className = 'nav-card__title';
      name.textContent = title;
      const x = document.createElement('button');
      x.type = 'button';
      x.className = 'nav-card__close';
      x.textContent = '×';
      x.title = 'Kapat';
      x.setAttribute('aria-label', `${title}: kapat`);
      x.addEventListener('click', close);
      el.append(name, ...extra, x);
      return el;
    };

    this.overviewEl = document.createElement('div');
    this.overviewEl.className = 'nav-card nav-card--overview';
    this.picture = document.createElement('canvas');
    this.picture.className = 'nav-card__picture';
    this.picture.title = 'Görünümü taşımak için basın ya da sürükleyin; tekerlek yakınlaştırır, çift tık tümünü gösterir';
    this.empty = document.createElement('div');
    this.empty.className = 'nav-card__empty';
    this.empty.textContent = 'Çizimde gösterilecek nesne yok.';
    this.overviewEl.append(
      head('Genel bakış', () => ui.overview.set(false)),
      this.picture,
      this.empty,
    );

    this.lensEl = document.createElement('div');
    this.lensEl.className = 'nav-card nav-card--lens';
    const zooms = document.createElement('span');
    zooms.className = 'nav-card__zooms';
    for (const z of ZOOMS) {
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'nav-card__zoom';
      b.textContent = `${z}×`;
      b.title = `${z} kat büyüt`;
      b.addEventListener('click', () => ui.magnifierZoom.set(z));
      this.zoomButtons.set(z, b);
      zooms.append(b);
    }
    const window_ = document.createElement('div');
    window_.className = 'nav-card__window';
    this.hint = document.createElement('div');
    this.hint.className = 'nav-card__empty nav-card__hint';
    this.hint.textContent = 'İmleci çizimin üstüne getirin.';
    window_.append(this.hint);
    this.lensEl.append(head('Büyüteç', () => ui.magnifier.set(false), zooms), window_);
    parent.append(this.overviewEl, this.lensEl);

    const d = this.d;
    d.add(() => {
      clearTimeout(this.timer);
      this.overviewEl.remove();
      this.lensEl.remove();
    });
    const shown = () => {
      this.layout();
      if (ui.overview.value) this.invalidate(0);
      host.requestRender();
    };
    d.add(ui.overview.subscribe(shown));
    d.add(ui.magnifier.subscribe(shown));
    d.add(
      ui.magnifierZoom.subscribe(() => {
        this.showZoom();
        host.requestRender();
      }),
    );
    this.bindOverview();
    this.showZoom();
  }

  dispose(): void {
    this.d.dispose();
  }

  get overviewShown(): boolean {
    return this.host.ctx.ui.overview.value;
  }

  get lensShown(): boolean {
    return this.host.ctx.ui.magnifier.value;
  }

  /** The magnifier's zoom: the kept one, at one of its steps. */
  get zoom(): Zoom {
    const kept = this.host.ctx.ui.magnifierZoom.value;
    return ZOOMS.reduce((a, b) => (Math.abs(Math.log2(b / kept)) < Math.abs(Math.log2(a / kept)) ? b : a));
  }

  private showZoom(): void {
    const z = this.zoom;
    for (const [n, b] of this.zoomButtons) b.setAttribute('aria-pressed', String(n === z));
  }

  private header(): number {
    const s = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--ui-scale')) || 1;
    return Math.round(HEADER * s);
  }

  /** The cards at their places in a drawing area of `width` × `height` CSS px (null: as it is). */
  layout(width = this.area.width, height = this.area.height): void {
    this.area = { width, height };
    const header = this.header();
    const laid = cards(this.area, header, this.host.ctx.format.axes !== 'cad', this.overviewShown);
    this.laid = laid;
    const place = (el: HTMLElement, r: readonly number[] | null) => {
      el.hidden = !r;
      if (!r) return;
      Object.assign(el.style, { left: `${r[0]}px`, top: `${r[1]}px`, width: `${r[2]}px`, height: `${r[3]}px` });
      el.style.setProperty('--nav-head', `${header}px`);
    };
    place(this.overviewEl, laid.overview?.card ?? null);
    place(this.lensEl, this.lensShown ? laid[this.side].card : null);
    this.hint.hidden = !!this.at;
    const dpr = this.host.dpr();
    this.picture.width = Math.floor(OVERVIEW.width * dpr + 0.5);
    this.picture.height = Math.floor(OVERVIEW.height * dpr + 0.5);
    this.drawOverview();
  }

  // ── Genel bakış ──────────────────────────────────────────────────────

  /** What the drawing shows changed: the picture is drawn again `after` ms later. */
  invalidate(after = REDRAW_MS): void {
    if (!this.overviewShown) return;
    clearTimeout(this.timer);
    this.timer = window.setTimeout(() => this.redraw(), after);
  }

  /** The visible layers' order of a frame: a change (a layer hidden or shown) draws the picture again. */
  order(key: string): void {
    if (key === this.orderKey) return;
    this.orderKey = key;
    this.invalidate();
  }

  /** The core's picture of the drawing, each layer in its colour. */
  private redraw(): void {
    const { ctx, picker } = this.host;
    const pal = this.host.palette();
    const colors: Record<string, string> = {};
    for (const l of ctx.doc.layers.leaves()) colors[l.id] = hex(resolveColor(l.style.color, pal));
    const dpr = this.host.dpr();
    const extent = picker.overviewExtent();
    const pixels = extent ? picker.overviewPicture(OVERVIEW.width, OVERVIEW.height, dpr, colors) : new Uint8Array();
    if (!extent || !pixels.length) this.shot = null;
    else {
      const canvas = this.shot?.canvas ?? document.createElement('canvas');
      canvas.width = Math.floor(OVERVIEW.width * dpr + 0.5);
      canvas.height = Math.floor(OVERVIEW.height * dpr + 0.5);
      canvas.getContext('2d')!.putImageData(new ImageData(new Uint8ClampedArray(pixels), canvas.width, canvas.height), 0, 0);
      this.shot = { canvas, fit: fit(extent, OVERVIEW), extent };
    }
    this.drawOverview();
  }

  /** The picture and the view's frame over it: every frame, cheap (one copy, one rectangle). */
  drawOverview(): void {
    if (!this.overviewShown) return;
    const g = this.picture.getContext('2d');
    if (!g) return;
    const pal = this.host.palette();
    const dpr = this.host.dpr();
    const [r, gr, b] = pal.background;
    g.setTransform(1, 0, 0, 1, 0, 0);
    g.fillStyle = `rgb(${Math.round(r * 255)}, ${Math.round(gr * 255)}, ${Math.round(b * 255)})`;
    g.fillRect(0, 0, this.picture.width, this.picture.height);
    this.empty.hidden = !!this.shot;
    if (!this.shot) return;
    g.drawImage(this.shot.canvas, 0, 0);
    const cam = this.host.camera;
    const v = viewFrame(this.shot.fit, OVERVIEW, cam.center, 1 / cam.scale, { width: cam.width, height: cam.height });
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.strokeStyle = pal.accent;
    g.lineWidth = 1.5;
    if (v.cross) {
      const x = (v.frame[0] + v.frame[2]) / 2;
      const y = (v.frame[1] + v.frame[3]) / 2;
      g.beginPath();
      g.moveTo(x - 6, y);
      g.lineTo(x + 6, y);
      g.moveTo(x, y - 6);
      g.lineTo(x, y + 6);
      g.stroke();
      return;
    }
    const [u0, v0, u1, v1] = v.frame;
    g.fillStyle = pal.accent;
    g.globalAlpha = 0.14;
    g.fillRect(u0, v0, u1 - u0, v1 - v0);
    g.globalAlpha = 1;
    g.strokeRect(u0, v0, u1 - u0, v1 - v0);
  }

  /** A drawing point in the page's client pixels over the overview's picture (the traces press there); null when it shows nothing. */
  overviewClient(p: Vec2): Vec2 | null {
    if (!this.shot || !this.overviewShown) return null;
    const r = this.picture.getBoundingClientRect();
    const f = this.shot.fit;
    return { x: r.left + OVERVIEW.width / 2 + (p.x - f.cx) * f.k, y: r.top + OVERVIEW.height / 2 - (p.y - f.cy) * f.k };
  }

  /** What the overview shows: its extent; null when it is closed or shows nothing. */
  get overviewExtent(): Bounds | null {
    return this.overviewShown ? (this.shot?.extent ?? null) : null;
  }

  private bindOverview(): void {
    const { camera, navigation } = this.host;
    const el = this.picture;
    const worldAt = (e: PointerEvent | MouseEvent): Vec2 | null => {
      if (!this.shot) return null;
      const r = el.getBoundingClientRect();
      return toWorld(this.shot.fit, OVERVIEW, e.clientX - r.left, e.clientY - r.top);
    };
    const go = (e: PointerEvent) => {
      const p = worldAt(e);
      if (p) camera.setView(p, camera.scale);
    };
    this.d.add(
      listen<PointerEvent>(el, 'pointerdown', (e) => {
        if (e.button !== 0 || !this.shot) return;
        e.preventDefault();
        el.setPointerCapture(e.pointerId);
        this.pressed = true;
        navigation.remember();
        go(e);
      }),
    );
    this.d.add(listen<PointerEvent>(el, 'pointermove', (e) => this.pressed && go(e)));
    const release = (e: PointerEvent) => {
      this.pressed = false;
      if (el.hasPointerCapture(e.pointerId)) el.releasePointerCapture(e.pointerId);
    };
    this.d.add(listen<PointerEvent>(el, 'pointerup', release));
    this.d.add(listen<PointerEvent>(el, 'pointercancel', release));
    this.d.add(listen<MouseEvent>(el, 'dblclick', () => this.host.zoomExtents()));
    this.d.add(
      listen<WheelEvent>(
        el,
        'wheel',
        (e) => {
          e.preventDefault();
          const step = e.deltaMode === 1 ? e.deltaY * 0.05 : e.deltaY * 0.0015;
          navigation.wheel();
          camera.zoomAt(Math.exp(-step), { x: camera.width / 2, y: camera.height / 2 });
        },
        { passive: false },
      ),
    );
  }

  // ── Büyüteç ──────────────────────────────────────────────────────────

  /**
   * The pointer at `screen` over the drawing (null: it left), on the drawing at `world`: the magnifier looks there,
   * and moves to the other side when the pointer nears its card. Whether the drawing is to be drawn again.
   */
  pointer(screen: Vec2 | null, world: Vec2 | null): boolean {
    if (!this.lensShown || !this.laid) return false;
    let changed = false;
    if (screen) {
      const side = nextSide(this.laid, this.side, screen);
      if (side !== this.side) {
        this.side = side;
        this.layout();
        changed = true;
      }
    }
    if (world && (!this.at || this.at.x !== world.x || this.at.y !== world.y)) {
      this.at = world;
      this.hint.hidden = true;
      changed = true;
    }
    return changed;
  }

  /** The magnifier's content rectangle, CSS px in the drawing area; null when it is closed or has looked nowhere yet. */
  private lensRect(): readonly [number, number, number, number] | null {
    if (!this.lensShown || !this.laid || !this.at) return null;
    return this.laid[this.side].content;
  }

  /** The magnifier's picture for the backend: its rectangle and camera, origin-relative. */
  lens(origin: Vec2): LensFrame | null {
    const r = this.lensRect();
    if (!r || !this.at) return null;
    const scale = this.host.camera.scale * this.zoom;
    return { rect: r, view: { center: { x: this.at.x - origin.x, y: this.at.y - origin.y }, scale, width: r[2], height: r[3], dpr: this.host.dpr() } };
  }

  /** The magnifier's rectangle and its camera for the 2D overlay (the drawing's text and marks there). */
  lensCamera(): { rect: readonly [number, number, number, number]; camera: Camera } | null {
    const r = this.lensRect();
    if (!r || !this.at) return null;
    // Its fields set as they are: the drawing's camera keeps its scale within its own limits, the backend's lens does not.
    const camera = new Camera();
    camera.width = r[2];
    camera.height = r[3];
    camera.center = { ...this.at };
    camera.scale = this.host.camera.scale * this.zoom;
    return { rect: r, camera };
  }

  /** What the magnifier shows: its zoom, side and centre; null when it is closed. */
  get lensState(): { zoom: Zoom; side: Side; center: Vec2 | null } | null {
    return this.lensShown ? { zoom: this.zoom, side: this.side, center: this.at } : null;
  }
}
