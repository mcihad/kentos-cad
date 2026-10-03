import type { Handle } from '../../contracts/generated/sheet/Handle';
import type { HandleHit } from '../../contracts/generated/sheet/HandleHit';
import type { Item } from '../../contracts/generated/sheet/Item';
import type { Op } from '../../contracts/generated/sheet/Op';
import type { RectUm } from '../../contracts/generated/sheet/RectUm';
import { bookText, errorText, type SnapDrag } from '../../product/sheet/engine';
import { HANDLE, ROTATE_OFFSET } from '../../render/sheet/paperPainter';
import { boxUm, DRAG_PX, toleranceUm, toUm, type PaperPoint, type PaperTool, type PaperToolHost, type PointerMods } from './tool';

/**
 * Seç (docs/sheet/design.md §5, §11): a click takes the item under the
 * pointer as the engine finds it (`hitTest`: a turned frame by its outline,
 * a line by its stroke, a group whole), Shift adds and Ctrl flips; a drag on
 * a chosen item moves the choice, snapped by the engine's `SnapSession`
 * (guides, items, the page and its margins, the grid, equal spacing; the
 * lines, distances and marks come with it); a drag on a handle resizes the
 * one chosen item (Shift keeps its proportions, Alt from its centre), on
 * the round handle above it turns it (Shift in 15° steps, a quarter turn
 * within 2°, the engine's `snapRotation`); a drag on empty paper chooses
 * with a box, left to right what lies inside, right to left what it
 * touches. While a drag lasts the paper shows the engine's operation
 * applied to a copy of the book; the release applies it once.
 */

type Drag =
  | { kind: 'press'; at: PaperPoint; hit: string | null; mods: PointerMods; wasChosen: boolean }
  | { kind: 'move'; at: PaperPoint; ids: string[]; snap: SnapDrag; op: Op | null }
  | { kind: 'resize'; id: string; handle: Handle; snap: SnapDrag; op: Op | null }
  | { kind: 'rotate'; item: Item; from: number; op: Op | null }
  | { kind: 'marquee'; at: PaperPoint; to: PaperPoint; add: boolean };

const CURSOR: Record<HandleHit, string> = { n: 'ns-resize', s: 'ns-resize', e: 'ew-resize', w: 'ew-resize', ne: 'nesw-resize', sw: 'nesw-resize', nw: 'nwse-resize', se: 'nwse-resize', rotate: 'grab' };

export class SelectTool implements PaperTool {
  private readonly host: PaperToolHost;
  private drag: Drag | null = null;
  /** The item under the pointer, for the painter's thin outline. */
  hovered: string | null = null;

  constructor(host: PaperToolHost) {
    this.host = host;
  }

  private items(): Item[] {
    const id = this.host.sheet();
    return this.host.book().book.sheets.find((s) => s.id === id)?.items ?? [];
  }

  /** The chosen item whose handle is under the point, if exactly one unlocked item is chosen. */
  private handleAt(p: PaperPoint): { id: string; handle: HandleHit } | null {
    const chosen = [...this.host.state.selection.value];
    if (chosen.length !== 1) return null;
    const item = this.items().find((i) => i.id === chosen[0]);
    if (!item || item.locked || item.kind.type === 'group') return null;
    const px = 1000 / this.host.scale();
    try {
      const h = this.host.engine.hitTest(this.host.book(), this.host.sheet(), { type: 'handle', item: item.id, at: [toUm(p.x), toUm(p.y)], tolerance: Math.round((HANDLE / 2 + 2) * px), rotateOffset: Math.round(ROTATE_OFFSET * px) });
      return h.handle ? { id: item.id, handle: h.handle } : null;
    } catch {
      return null;
    }
  }

  /** What a click takes at the point: the outermost group of the topmost item that is not locked. */
  private takenAt(p: PaperPoint): string | null {
    try {
      const hits = this.host.engine.hitTest(this.host.book(), this.host.sheet(), { type: 'point', at: [toUm(p.x), toUm(p.y)], tolerance: Math.round((2 * 1000) / this.host.scale()) });
      const locked = new Set(this.items().filter((i) => i.locked).map((i) => i.id));
      return hits.hits.find((h) => !locked.has(h.top) && !locked.has(h.item))?.top ?? null;
    } catch (e) {
      this.host.say(`Seçilemedi: ${errorText(e)}`);
      return null;
    }
  }

  down(p: PaperPoint, m: PointerMods): boolean {
    const handle = this.handleAt(p);
    if (handle) {
      const item = this.items().find((i) => i.id === handle.id)!;
      if (handle.handle === 'rotate') {
        this.drag = { kind: 'rotate', item, from: this.angle(item.frame, p), op: null };
        this.host.cursor('grabbing');
      } else {
        const snap = this.session([item.id]);
        if (!snap) return false;
        this.drag = { kind: 'resize', id: item.id, handle: handle.handle, snap, op: null };
      }
      return true;
    }
    const hit = this.takenAt(p);
    const sel = this.host.state.selection.value;
    this.drag = { kind: 'press', at: p, hit, mods: m, wasChosen: !!hit && sel.has(hit) };
    if (hit) {
      // A press on an item not chosen chooses it at once (it is about to be dragged); one chosen keeps the choice.
      if (m.ctrl) this.host.state.select([hit], 'toggle');
      else if (m.shift) this.host.state.select([hit], 'add');
      else if (!sel.has(hit)) this.host.state.select([hit], 'replace');
    }
    return true;
  }

  move(p: PaperPoint, m: PointerMods): void {
    const d = this.drag;
    if (!d) return;
    const scale = this.host.scale();
    switch (d.kind) {
      case 'press': {
        if (Math.hypot(p.x - d.at.x, p.y - d.at.y) * scale < DRAG_PX) return;
        if (!d.hit) {
          this.drag = { kind: 'marquee', at: d.at, to: p, add: d.mods.shift || d.mods.ctrl };
          this.move(p, m);
          return;
        }
        const chosen = this.items().filter((i) => this.host.state.selection.value.has(i.id));
        const locked = chosen.filter((i) => i.locked);
        if (!chosen.length || locked.length) {
          if (locked.length) this.host.say(`“${locked[0].name}” kilitli: taşımak için Öğeler listesinde kilidini açın.`);
          this.drag = null;
          return;
        }
        const snap = this.session(chosen.map((i) => i.id));
        if (!snap) {
          this.drag = null;
          return;
        }
        this.drag = { kind: 'move', at: d.at, ids: chosen.map((i) => i.id), snap, op: null };
        this.host.cursor('move');
        this.move(p, m);
        return;
      }
      case 'move': {
        const r = d.snap.query(toUm(p.x - d.at.x), toUm(p.y - d.at.y), m.alt ? 0 : toleranceUm(this.host));
        d.op = { op: 'moveItems', ids: d.ids, delta: r.delta };
        this.show(d.op);
        this.host.overlay({ snap: r });
        return;
      }
      case 'resize': {
        const r = d.snap.resize(d.handle, toUm(p.x), toUm(p.y), m.ctrl ? 0 : toleranceUm(this.host), m.shift);
        d.op = { op: 'resizeItem', id: d.id, handle: d.handle, to: r.to, keepAspect: m.shift, fromCenter: m.alt };
        this.show(d.op);
        this.host.overlay({ resize: r });
        return;
      }
      case 'rotate': {
        const turn = this.angle(d.item.frame, p) - d.from;
        const want = this.host.engine.snapRotation(((d.item.rotation + turn) % 360_000 + 360_000) % 360_000, m.shift);
        const angle = ((want - d.item.rotation) % 360_000 + 360_000) % 360_000;
        d.op = { op: 'rotateItems', ids: [d.item.id], angle };
        this.show(d.op);
        this.host.overlay({ angle: { at: p, degrees: want / 1000 } });
        return;
      }
      case 'marquee': {
        d.to = p;
        this.host.overlay({ marquee: { box: boxUm(d.at, d.to), crossing: d.to.x < d.at.x } });
        return;
      }
    }
  }

  up(p: PaperPoint, m: PointerMods): void {
    const d = this.drag;
    this.drag = null;
    if (!d) return;
    this.host.overlay(null);
    this.host.cursor('');
    switch (d.kind) {
      case 'press':
        // A click on empty paper lets the choice go (unless it meant to add); on one of several chosen, takes just it.
        if (!d.hit && !d.mods.shift && !d.mods.ctrl) this.host.state.clearSelection();
        else if (d.hit && d.wasChosen && !d.mods.shift && !d.mods.ctrl) this.host.state.select([d.hit], 'replace');
        return;
      case 'move':
      case 'resize':
      case 'rotate':
        if (d.kind !== 'rotate') d.snap.free();
        this.host.preview(null);
        if (d.op && !this.idle(d.op)) this.host.apply([d.op]);
        return;
      case 'marquee': {
        void m;
        const box = boxUm(d.at, p);
        try {
          const hits = this.host.engine.hitTest(this.host.book(), this.host.sheet(), { type: 'rect', rect: box, mode: p.x < d.at.x ? 'intersect' : 'contain' });
          const locked = new Set(this.items().filter((i) => i.locked).map((i) => i.id));
          const tops = [...new Set(hits.hits.map((h) => h.top))].filter((id) => !locked.has(id));
          this.host.state.select(tops, d.add ? 'add' : 'replace');
        } catch (e) {
          this.host.say(`Seçilemedi: ${errorText(e)}`);
        }
        return;
      }
    }
  }

  hover(p: PaperPoint | null): void {
    if (this.drag) return;
    if (!p) {
      this.hovered = null;
      this.host.cursor('');
      return;
    }
    const handle = this.handleAt(p);
    this.host.cursor(handle ? CURSOR[handle.handle] : '');
    this.hovered = handle ? null : this.takenAt(p);
  }

  cancel(): void {
    const d = this.drag;
    this.drag = null;
    if (d && (d.kind === 'move' || d.kind === 'resize')) d.snap.free();
    this.host.preview(null);
    this.host.overlay(null);
    this.host.cursor('');
  }

  /** Whether a drag is in progress (the workspace keeps its keys while it is). */
  get busy(): boolean {
    return !!this.drag && this.drag.kind !== 'press';
  }

  private session(ids: string[]): SnapDrag | null {
    try {
      return this.host.engine.snapDrag(this.host.book(), this.host.sheet(), ids);
    } catch (e) {
      this.host.say(`Sürüklenemedi: ${errorText(e)}`);
      return null;
    }
  }

  /** The operation applied to a copy of the book, shown while the drag lasts. */
  private show(op: Op): void {
    try {
      this.host.preview(bookText(this.host.engine.applyOp(this.host.book(), op).book));
    } catch {
      // A step the engine refuses (a resize past zero) shows the last one that it took.
    }
  }

  /** An operation that changes nothing (a drag that came back to where it started). */
  private idle(op: Op): boolean {
    if (op.op === 'moveItems') return !op.delta[0] && !op.delta[1];
    if (op.op === 'rotateItems') return !op.angle;
    return false;
  }

  /** The pointer's direction from a frame's centre, millidegrees clockwise from straight up. */
  private angle(f: RectUm, p: PaperPoint): number {
    const cx = (f.left + f.width / 2) / 1000;
    const cy = (f.top + f.height / 2) / 1000;
    return Math.round((Math.atan2(p.x - cx, -(p.y - cy)) * 180_000) / Math.PI);
  }
}
