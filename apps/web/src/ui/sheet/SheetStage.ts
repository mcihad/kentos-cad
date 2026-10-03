import type { AppContext } from '../../app/context';
import type { Sheet } from '../../contracts/generated/sheet/Sheet';
import { Signal, watchAll } from '../../core/signal';
import { mm } from '../../product/sheet/adapter';
import type { BookText } from '../../product/sheet/engine';
import { SELECT } from '../../product/sheet/state';
import { boundsOf, readPaperColors, type PaperColors } from '../../render/sheet/paperPainter';
import { fitPage, nextStep, PX_PER_MM, realSize, zoomAt, zoomPercent, type PaperViewport } from '../../render/sheet/paperView';
import { paintRuler, readRulerColors, type RulerColors } from '../../render/sheet/rulerPainter';
import { AddTool } from '../../tools/sheet/addTool';
import { SelectTool } from '../../tools/sheet/selectTool';
import type { PaperTool, PaperToolHost, ToolOverlay } from '../../tools/sheet/tool';
import { Component } from '../Component';
import { h } from '../dom';
import { icon } from '../icons';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import type { SheetHost } from './host';
import { StageInput, type StageParts } from './stageInput';
import { chosenFrames, paintStage } from './stagePaint';

/**
 * The desk (docs/sheet/design.md §11): the sheet as the engine plans it on
 * the theme's surface, millimetre rulers above and beside it, the zoom box in
 * its corner. Ctrl+wheel (or a pinch) zooms about the pointer, the wheel
 * scrolls (Shift across), the middle button, Space held or the El tool pans;
 * Ctrl+0 fits the page, Ctrl+1 shows it at real size. A press on the paper
 * goes to the tool in hand (tools/sheet/: Seç, or a tool that adds an item);
 * a drag out of a ruler makes a guide, a guide dragged back onto one goes.
 * While a drag lasts the paper shows the tool's copy of the book. Each sheet
 * keeps its own view while the window is open; the canvases have device
 * pixels and follow a change of pixel ratio.
 */
export class SheetStage extends Component implements StageParts {
  readonly el: HTMLElement;
  /** The pointer on the paper, mm from its top left corner; null off the desk. */
  readonly cursor = new Signal<{ x: number; y: number } | null>(null);
  /** The zoom as a percentage of real size. */
  readonly zoom = new Signal(100);
  readonly ctx: AppContext;
  readonly host: SheetHost;
  readonly paper: HTMLCanvasElement;
  readonly top: HTMLCanvasElement;
  readonly left: HTMLCanvasElement;
  readonly select: SelectTool;
  private readonly zoomValue: HTMLButtonElement;
  private readonly empty: HTMLElement;
  private readonly views = new Map<string, PaperViewport>();
  private colors: PaperColors | null = null;
  private rulerColors: RulerColors | null = null;
  private frame = 0;
  private space = false;
  private dpr = Math.max(1, window.devicePixelRatio || 1);
  private preview: BookText | null = null;
  private overlay: ToolOverlay | null = null;
  private current: PaperTool;
  private readonly input: StageInput;

  constructor(ctx: AppContext, host: SheetHost) {
    super();
    this.ctx = ctx;
    this.host = host;
    this.paper = h('canvas', { class: 'sheet-stage__paper', tabindex: '0', role: 'application', 'aria-label': 'Pafta: kâğıt ve öğeler' });
    this.top = h('canvas', { class: 'sheet-stage__ruler', 'aria-label': 'Yatay cetvel: aşağı sürükleyince yatay kılavuz' });
    this.left = h('canvas', { class: 'sheet-stage__ruler', 'aria-label': 'Düşey cetvel: sağa sürükleyince düşey kılavuz' });
    const minus = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Uzaklaştır' }, icon('zoomOut', 16));
    const plus = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Yakınlaştır' }, icon('zoomIn', 16));
    this.zoomValue = h('button', { class: 'sheet-zoom__value', type: 'button', 'aria-haspopup': 'menu', 'aria-label': 'Yakınlaştırma' }, '%100');
    minus.addEventListener('click', () => this.step(-1));
    plus.addEventListener('click', () => this.step(1));
    this.zoomValue.addEventListener('click', () => PopupMenu.open(this.zoomItems(), this.zoomValue.getBoundingClientRect(), { owner: this.zoomValue, minWidth: 200 }));
    this.d.add(tooltip(minus, () => ({ title: 'Uzaklaştır', shortcut: ctx.keymap.chordFor('sheet.zoomOut') }), 'top'));
    this.d.add(tooltip(plus, () => ({ title: 'Yakınlaştır', shortcut: ctx.keymap.chordFor('sheet.zoomIn') }), 'top'));
    this.d.add(tooltip(this.zoomValue, () => ({ title: 'Yakınlaştırma', description: 'Gerçek boya göre: %100’de kâğıdın bir milimetresi ekranda bir milimetredir. Ctrl+tekerlek yakınlaştırır.' }), 'top'));
    this.empty = h(
      'div',
      { class: 'sheet-stage__empty', hidden: true },
      h('div', null, h('b', null, 'Bu pafta boş'), 'Şeridin Pafta sekmesindeki Ekle panelinden harita, metin, lejant, antet ekleyin; ya da Şablondan ile hazır bir düzenle başlayın.'),
    );
    this.el = h(
      'div',
      { class: 'sheet-stage' },
      h('div', { class: 'sheet-stage__corner', title: 'Cetvel birimi: milimetre' }, 'mm'),
      this.top,
      this.left,
      this.paper,
      this.empty,
      h('div', { class: 'sheet-zoom' }, minus, this.zoomValue, plus),
    );
    this.select = new SelectTool(this.toolHost());
    this.current = this.select;
    const { state } = host;
    this.d.add(watchAll([state.book, state.open, state.selection, host.profile, host.painted], () => this.request()));
    this.d.add(state.open.subscribe(() => this.showSheet()));
    this.d.add(state.tool.subscribe(() => this.takeTool()));
    this.d.add(ctx.prefs.theme.subscribe(() => this.restyle()));
    this.d.add(ctx.prefs.accent.subscribe(() => this.restyle()));
    const ro = new ResizeObserver(() => this.resized());
    ro.observe(this.paper);
    this.d.add(() => ro.disconnect());
    this.watchPixelRatio();
    this.input = new StageInput(this, this.d);
    this.d.add(() => cancelAnimationFrame(this.frame));
  }

  /** A paper point (mm) in the window's coordinates (CSS px), for scripts that point at an item; null with no sheet. */
  clientPoint(p: { x: number; y: number }): { x: number; y: number } | null {
    const s = this.shown();
    if (!s) return null;
    const r = this.paper.getBoundingClientRect();
    const v = this.viewOf(s);
    return { x: r.left + v.x + p.x * v.scale, y: r.top + v.y + p.y * v.scale };
  }

  /** Gives the keyboard to the paper (after a click on a tab or a tool). */
  focus(): void {
    this.paper.focus({ preventScroll: true });
  }

  /** The tool in hand. */
  tool(): PaperTool {
    return this.current;
  }

  /** The engine's sheet in front, as the paper shows it now (a drag's copy while one lasts). */
  shown(): Sheet | null {
    const id = this.host.state.open.value;
    const book = this.preview ?? this.host.book();
    return (id && book?.book.sheets.find((s) => s.id === id)) || null;
  }

  private paperSize(s: Sheet) {
    return { width: mm(s.page.size.width), height: mm(s.page.size.height) };
  }

  /** The view of the sheet in front; one fitted to the page when the sheet is first shown. */
  viewOf(sheet: Sheet): PaperViewport {
    let v = this.views.get(sheet.id);
    if (!v) {
      v = fitPage(this.paperSize(sheet), this.area());
      if (this.paper.clientWidth) this.views.set(sheet.id, v);
    }
    return v;
  }

  setView(v: PaperViewport): void {
    const sheet = this.shown();
    if (!sheet) return;
    this.views.set(sheet.id, v);
    this.request();
  }

  private area(): { width: number; height: number } {
    return { width: this.paper.clientWidth || 800, height: this.paper.clientHeight || 600 };
  }

  fitPage(): void {
    const s = this.shown();
    if (s) this.setView(fitPage(this.paperSize(s), this.area()));
  }

  realSize(): void {
    const s = this.shown();
    if (s) this.setView(realSize(this.paperSize(s), this.area(), this.viewOf(s)));
  }

  /** The chosen items fitted in the desk, `pad` CSS px clear round them. */
  zoomChoice(pad = 72): void {
    const s = this.shown();
    const box = s ? boundsOf(chosenFrames(s, this.host.state.selection.value)) : null;
    if (!s || !box) return;
    const fit = fitPage({ width: Math.max(box.width, 1), height: Math.max(box.height, 1) }, this.area(), pad);
    this.setView({ scale: fit.scale, x: fit.x - box.left * fit.scale, y: fit.y - box.top * fit.scale });
  }

  /** One zoom step in (1) or out (−1), about the desk's centre. */
  step(dir: 1 | -1): void {
    const s = this.shown();
    if (!s) return;
    const v = this.viewOf(s);
    const a = this.area();
    this.setView(zoomAt(v, (nextStep(v.scale / PX_PER_MM, dir) * PX_PER_MM) / v.scale, { x: a.width / 2, y: a.height / 2 }));
  }

  private zoomItems(): MenuItem[] {
    const now = this.zoom.value;
    const to = (share: number) => {
      const s = this.shown();
      if (!s) return;
      const v = this.viewOf(s);
      const a = this.area();
      this.setView(zoomAt(v, (share * PX_PER_MM) / v.scale, { x: a.width / 2, y: a.height / 2 }));
    };
    return [
      { label: 'Sayfayı sığdır', icon: 'sheetZoomPage', shortcut: this.ctx.keymap.chordFor('sheet.zoomPage'), run: () => this.fitPage() },
      { label: 'Gerçek boyut', icon: 'sheetZoomReal', shortcut: this.ctx.keymap.chordFor('sheet.zoomReal'), hint: '%100', run: () => this.realSize() },
      { label: 'Seçime yakınlaş', icon: 'zoomSelection', disabled: !this.host.state.selection.value.size, run: () => this.zoomChoice() },
      { kind: 'separator' },
      ...[0.25, 0.5, 1, 2, 4].map((s): MenuItem => ({ label: `%${Math.round(s * 100)}`, radio: true, checked: now === Math.round(s * 100), run: () => to(s) })),
    ];
  }

  /** Space held: a press on the paper pans (the key layer tells it, app/sheet/keys.ts). */
  setSpace(held: boolean): void {
    if (this.space === held) return;
    this.space = held;
    this.syncCursor('', this.input.panning);
  }

  handHeld(): boolean {
    return this.space || this.host.state.tool.value.kind === 'hand';
  }

  /** Esc: a drag in progress is dropped (true); false when there was none. */
  cancelDrag(): boolean {
    return this.input.cancel();
  }

  private showSheet(): void {
    this.cancelDrag();
    this.select.hovered = null;
    this.request();
  }

  private takeTool(): void {
    this.current.cancel();
    const t = this.host.state.tool.value;
    this.current = t.kind === 'add' && this.host.engine() ? new AddTool(this.toolHost(), { tool: t.tool, preset: t.preset, label: t.label, kind: t.item }) : this.select;
    this.syncCursor(t.kind === 'add' ? 'crosshair' : '', false);
    this.request();
  }

  private restyle(): void {
    this.colors = null;
    this.rulerColors = null;
    this.request();
  }

  private resized(): void {
    const s = this.shown();
    if (s && !this.views.has(s.id) && this.paper.clientWidth) this.views.set(s.id, fitPage(this.paperSize(s), this.area()));
    this.paintNow();
  }

  /** Follows the screen's pixel ratio (the window moved to another screen, the browser zoomed). */
  private watchPixelRatio(): void {
    let mq: MediaQueryList | null = null;
    const follow = () => {
      mq?.removeEventListener('change', changed);
      mq = matchMedia(`(resolution: ${this.dpr}dppx)`);
      mq.addEventListener('change', changed);
    };
    const changed = () => {
      this.dpr = Math.max(1, window.devicePixelRatio || 1);
      follow();
      this.paintNow();
    };
    follow();
    this.d.add(() => mq?.removeEventListener('change', changed));
  }

  request(): void {
    if (this.frame) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      this.paintNow();
    });
  }

  /** Gives a canvas its CSS size in device pixels; true when it has a size. */
  private size(c: HTMLCanvasElement): boolean {
    const w = Math.round(c.clientWidth * this.dpr);
    const hh = Math.round(c.clientHeight * this.dpr);
    if (!w || !hh) return false;
    if (c.width !== w) c.width = w;
    if (c.height !== hh) c.height = hh;
    return true;
  }

  private paintNow(): void {
    cancelAnimationFrame(this.frame);
    this.frame = 0;
    const sheet = this.shown();
    const book = this.preview ?? this.host.book();
    if (!sheet || !book || !this.el.isConnected || !this.size(this.paper)) return;
    const colors = (this.colors ??= readPaperColors(this.el));
    const view = this.viewOf(sheet);
    const g = this.paper.getContext('2d');
    if (!g) return;
    const chosen = this.host.state.selection.value;
    const one = chosen.size === 1 ? sheet.items.find((i) => chosen.has(i.id)) : undefined;
    const handles = this.current !== this.select || !chosen.size ? 'none' : one && !one.locked && one.kind.type !== 'group' ? 'rotate' : 'resize';
    paintStage(g, { width: this.paper.clientWidth, height: this.paper.clientHeight }, this.dpr, { sheet, list: this.host.plan(book, sheet.id), view, chosen, hover: this.select.hovered, overlay: this.overlay, handles }, this.host.sources, colors);
    this.paintRulers(sheet, view);
    const pct = zoomPercent(view);
    this.zoom.set(pct);
    this.zoomValue.textContent = `%${pct}`;
    this.empty.hidden = sheet.items.length > 0;
  }

  private paintRulers(sheet: Sheet, view: PaperViewport): void {
    const rc = (this.rulerColors ??= readRulerColors(this.el));
    const box = boundsOf(chosenFrames(sheet, this.host.state.selection.value));
    const fontPx = parseFloat(getComputedStyle(this.el).fontSize) * 0.84 || 11;
    const cur = this.cursor.value;
    for (const [canvas, axis] of [
      [this.top, 'h'],
      [this.left, 'v'],
    ] as const) {
      if (!this.size(canvas)) continue;
      const rg = canvas.getContext('2d');
      if (!rg) continue;
      const horizontal = axis === 'h';
      paintRuler(
        rg,
        this.dpr,
        {
          axis,
          length: horizontal ? canvas.clientWidth : canvas.clientHeight,
          thickness: horizontal ? canvas.clientHeight : canvas.clientWidth,
          view,
          paperMm: horizontal ? mm(sheet.page.size.width) : mm(sheet.page.size.height),
          cursor: cur ? (horizontal ? cur.x : cur.y) : null,
          band: box ? (horizontal ? { from: box.left, to: box.left + box.width } : { from: box.top, to: box.top + box.height }) : null,
          fontPx,
        },
        rc,
      );
    }
  }

  // ── The tools ────────────────────────────────────────────────────

  toolHost(): PaperToolHost {
    const host = this.host;
    return {
      get engine() {
        return host.engine()!;
      },
      state: host.state,
      book: () => host.book()!,
      sheet: () => host.state.open.value ?? '',
      scale: () => {
        const s = this.shown();
        return s ? this.viewOf(s).scale : PX_PER_MM;
      },
      preview: (b) => {
        this.preview = b;
        this.request();
      },
      overlay: (o) => {
        this.overlay = o;
        this.request();
      },
      apply: (ops, label) => host.apply(ops, label),
      newId: () => host.newId(),
      workspace: () => host.workspace(),
      capabilities: () => host.capabilities(),
      mapCenter: () => host.mapPlace().center,
      say: (text) => this.ctx.log.warn(text),
      cursor: (css) => this.syncCursor(css, this.input.panning),
      done: () => host.state.tool.set(SELECT),
    };
  }

  syncCursor(css: string, panning: boolean): void {
    this.paper.style.cursor = panning ? 'grabbing' : this.handHeld() ? 'grab' : css;
  }
}
