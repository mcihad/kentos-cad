import type { AppContext } from '../../app/context';
import type { Sheet } from '../../contracts/generated/sheet/Sheet';
import { listen, type DisposableStore } from '../../core/disposable';
import type { Signal } from '../../core/signal';
import { panBy, toPaper, wheelFactor, zoomAt, type PaperViewport } from '../../render/sheet/paperView';
import { GuideDrag } from '../../tools/sheet/guideTool';
import type { SelectTool } from '../../tools/sheet/selectTool';
import type { PaperPoint, PaperTool, PaperToolHost, PointerMods } from '../../tools/sheet/tool';
import { PopupMenu } from '../widgets/PopupMenu';
import type { SheetHost } from './host';
import { itemItems, paperItems } from './menus';

/**
 * The desk's pointer and wheel (docs/sheet/design.md §5, §11), apart from the
 * desk itself (SheetStage.ts): a press on the paper goes to the tool in hand
 * (tools/sheet/), and its drag and release to the same tool; the middle
 * button, Space held or the El tool pan; Ctrl+wheel zooms about the pointer,
 * the wheel scrolls (Shift across); a double click on empty paper and a
 * double middle press fit the page; a right click gives the item's or the
 * paper's menu. A guide is moved on the paper and dragged out of a ruler;
 * dropped back on one it goes.
 */

/** What the input reaches of the desk. */
export interface StageParts {
  readonly ctx: AppContext;
  readonly host: SheetHost;
  readonly paper: HTMLCanvasElement;
  readonly top: HTMLCanvasElement;
  readonly left: HTMLCanvasElement;
  readonly cursor: Signal<{ x: number; y: number } | null>;
  readonly select: SelectTool;
  tool(): PaperTool;
  toolHost(): PaperToolHost;
  shown(): Sheet | null;
  viewOf(s: Sheet): PaperViewport;
  setView(v: PaperViewport): void;
  fitPage(): void;
  focus(): void;
  request(): void;
  /** Space held or the El tool in hand: a press pans. */
  handHeld(): boolean;
  /** The pointer's look: `panning` while a pan is dragged. */
  syncCursor(css: string, panning: boolean): void;
}

const mods = (e: PointerEvent | MouseEvent): PointerMods => ({ shift: e.shiftKey, ctrl: e.ctrlKey || e.metaKey, alt: e.altKey });

export class StageInput {
  private readonly s: StageParts;
  private pan: { x: number; y: number; id: number } | null = null;
  private pressed: PaperTool | null = null;
  private guide: GuideDrag | null = null;

  constructor(s: StageParts, d: DisposableStore) {
    this.s = s;
    this.bindPointer(d);
    this.bindRulers(d);
  }

  get panning(): boolean {
    return !!this.pan;
  }

  /** Esc: a drag in progress is dropped (true); false when there was none. */
  cancel(): boolean {
    const busy = !!this.pressed || !!this.guide;
    this.pressed?.cancel();
    this.guide?.cancel();
    this.pressed = null;
    this.guide = null;
    return busy;
  }

  private paperPoint(e: { clientX: number; clientY: number }): PaperPoint | null {
    const sheet = this.s.shown();
    if (!sheet) return null;
    const r = this.s.paper.getBoundingClientRect();
    return toPaper(this.s.viewOf(sheet), { x: e.clientX - r.left, y: e.clientY - r.top });
  }

  /** Whether the pointer is over a ruler (above or left of the paper's canvas): a guide dropped there goes. */
  private overRuler(e: { clientX: number; clientY: number }): boolean {
    const r = this.s.paper.getBoundingClientRect();
    return e.clientY < r.top || e.clientX < r.left;
  }

  private bindPointer(d: DisposableStore): void {
    const { s } = this;
    const c = s.paper;
    d.add(
      listen<PointerEvent>(c, 'pointerdown', (e) => {
        const p = this.paperPoint(e);
        if (!p || !s.host.engine()) return;
        s.focus();
        if (e.button === 1 || (e.button === 0 && s.handHeld())) {
          e.preventDefault();
          c.setPointerCapture(e.pointerId);
          this.pan = { x: e.clientX, y: e.clientY, id: e.pointerId };
          s.syncCursor('', true);
          return;
        }
        if (e.button !== 0) return;
        // A guide on the paper is moved before an item under it is taken.
        const guide = s.tool() === s.select ? GuideDrag.at(s.toolHost(), p) : null;
        if (guide) {
          this.guide = guide;
          c.setPointerCapture(e.pointerId);
          return;
        }
        if (s.tool().down(p, mods(e))) {
          this.pressed = s.tool();
          c.setPointerCapture(e.pointerId);
        }
        s.request();
      }),
    );
    d.add(
      listen<PointerEvent>(c, 'pointermove', (e) => {
        const sheet = s.shown();
        if (!sheet) return;
        if (this.pan && this.pan.id === e.pointerId) {
          const dx = e.clientX - this.pan.x;
          const dy = e.clientY - this.pan.y;
          this.pan = { ...this.pan, x: e.clientX, y: e.clientY };
          s.setView(panBy(s.viewOf(sheet), dx, dy));
        }
        const p = this.paperPoint(e)!;
        s.cursor.set(p);
        if (this.guide) this.guide.move(p, this.overRuler(e));
        else if (this.pressed) this.pressed.move(p, mods(e));
        else if (!this.pan) s.tool().hover?.(p);
        s.request();
      }),
    );
    const end = (e: PointerEvent, cancelled: boolean) => {
      if (this.pan && this.pan.id === e.pointerId) {
        this.pan = null;
        s.syncCursor('', false);
      }
      const p = this.paperPoint(e);
      const g = this.guide;
      this.guide = null;
      if (g) {
        if (cancelled || !p) g.cancel();
        else g.up(p, this.overRuler(e));
      }
      const t = this.pressed;
      this.pressed = null;
      if (t) {
        if (cancelled || !p) t.cancel();
        else t.up(p, mods(e));
      }
      s.request();
    };
    d.add(listen<PointerEvent>(c, 'pointerup', (e) => end(e, false)));
    d.add(listen<PointerEvent>(c, 'pointercancel', (e) => end(e, true)));
    d.add(
      listen<PointerEvent>(c, 'pointerleave', () => {
        if (this.pressed || this.guide) return;
        s.cursor.set(null);
        s.tool().hover?.(null);
        s.request();
      }),
    );
    d.add(
      listen<WheelEvent>(
        c,
        'wheel',
        (e) => {
          const sheet = s.shown();
          if (!sheet) return;
          e.preventDefault();
          const v = s.viewOf(sheet);
          if (e.ctrlKey || e.metaKey) {
            const r = c.getBoundingClientRect();
            s.setView(zoomAt(v, wheelFactor(e.deltaY, e.deltaMode), { x: e.clientX - r.left, y: e.clientY - r.top }));
            return;
          }
          const k = e.deltaMode === 1 ? 33 : e.deltaMode === 2 ? 400 : 1;
          // A mouse wheel with Shift scrolls across; a trackpad gives both directions itself.
          const dx = (e.shiftKey && !e.deltaX ? e.deltaY : e.deltaX) * k;
          const dy = (e.shiftKey && !e.deltaX ? 0 : e.deltaY) * k;
          s.setView(panBy(v, -dx, -dy));
        },
        { passive: false },
      ),
    );
    d.add(
      listen<MouseEvent>(c, 'dblclick', () => {
        if (!s.select.hovered && s.tool() === s.select) s.fitPage();
      }),
    );
    // A double press of the middle button fits the page, as it shows all on the drawing (DESIGN.md §9).
    let lastMiddle = 0;
    d.add(
      listen<MouseEvent>(c, 'auxclick', (e) => {
        if (e.button !== 1) return;
        const now = performance.now();
        if (now - lastMiddle < 350) s.fitPage();
        lastMiddle = now;
      }),
    );
    d.add(
      listen<MouseEvent>(c, 'contextmenu', (e) => {
        e.preventDefault();
        const p = this.paperPoint(e);
        if (!p || !s.host.engine()) return;
        s.select.hover(p);
        const hit = s.select.hovered;
        if (hit && !s.host.state.selection.value.has(hit)) s.host.state.select([hit]);
        PopupMenu.open(hit ? itemItems(s.ctx, s.host) : paperItems(s.ctx), { x: e.clientX, y: e.clientY }, { placement: 'point', minWidth: 220 });
      }),
    );
  }

  /** Guides out of the rulers: the top ruler gives a level guide, the left one an upright one. */
  private bindRulers(d: DisposableStore): void {
    const { s } = this;
    for (const [ruler, axis] of [
      [s.top, 'y'],
      [s.left, 'x'],
    ] as const) {
      const over = (e: PointerEvent) => {
        const r = ruler.getBoundingClientRect();
        return e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
      };
      d.add(
        listen<PointerEvent>(ruler, 'pointerdown', (e) => {
          if (e.button !== 0 || !s.shown() || !s.host.engine()) return;
          e.preventDefault();
          ruler.setPointerCapture(e.pointerId);
          this.guide = GuideDrag.fromRuler(s.toolHost(), axis);
        }),
      );
      d.add(
        listen<PointerEvent>(ruler, 'pointermove', (e) => {
          const p = this.paperPoint(e);
          if (this.guide && p) this.guide.move(p, over(e));
        }),
      );
      const end = (e: PointerEvent) => {
        const g = this.guide;
        this.guide = null;
        const p = this.paperPoint(e);
        if (!g) return;
        if (p && e.type === 'pointerup') g.up(p, over(e));
        else g.cancel();
      };
      d.add(listen<PointerEvent>(ruler, 'pointerup', end));
      d.add(listen<PointerEvent>(ruler, 'pointercancel', end));
    }
  }
}
