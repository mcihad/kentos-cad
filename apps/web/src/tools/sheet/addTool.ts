import type { Item } from '../../contracts/generated/sheet/Item';
import type { RectUm } from '../../contracts/generated/sheet/RectUm';
import { errorText } from '../../product/sheet/engine';
import { boxUm, DRAG_PX, toUm, type PaperPoint, type PaperTool, type PaperToolHost, type PointerMods } from './tool';

/**
 * A tool of the mode's profile that adds an item (docs/sheet/design.md
 * §11a: Harita, Metin, Lejant, Antet …, each with its ready looks): a drag
 * on the paper gives the new item its frame, a click puts one of the
 * kind's usual size at the point (a sheet frame takes the margins' box).
 * The item itself is the engine's (`newItem`: the mode's preset, the
 * project's plot scale for a map, which looks where the drawing area
 * looks); a scale bar, a north arrow, a legend and an overview read the
 * sheet's first map. One undo step; the new item is chosen and the paper
 * goes back to Seç.
 */

/** A click's frame, millimetres, by the kind the tool adds: the interface's own sizes, not a rule of the sheet. */
const CLICK_SIZE: Record<string, readonly [number, number]> = {
  map: [120, 80],
  text: [60, 12],
  legend: [50, 60],
  scaleBar: [60, 14],
  northArrow: [16, 22],
  table: [90, 40],
  coordinateList: [60, 50],
  titleBlock: [120, 40],
  picture: [40, 30],
  shape: [40, 30],
  line: [60, 4],
};

/** Items that read a map: the new one is linked to the sheet's first. */
const READS_MAP = new Set(['scaleBar', 'northArrow', 'legend']);

export class AddTool implements PaperTool {
  private readonly host: PaperToolHost;
  private readonly tool: string;
  private readonly preset: string | undefined;
  private readonly label: string;
  private readonly kind: string;
  private from: PaperPoint | null = null;
  private to: PaperPoint | null = null;

  constructor(host: PaperToolHost, tool: { tool: string; preset?: string; label: string; kind: string }) {
    this.host = host;
    this.tool = tool.tool;
    this.preset = tool.preset;
    this.label = tool.label;
    this.kind = tool.kind;
  }

  down(p: PaperPoint): boolean {
    this.from = this.to = p;
    return true;
  }

  move(p: PaperPoint): void {
    if (!this.from) return;
    this.to = p;
    if (this.dragged()) this.host.overlay({ frame: boxUm(this.from, p) });
  }

  up(p: PaperPoint, _m: PointerMods): void {
    const from = this.from;
    this.from = this.to = null;
    this.host.overlay(null);
    if (!from) return;
    this.place(this.dragged(from, p) ? boxUm(from, p) : this.clickFrame(p));
  }

  hover(): void {
    this.host.cursor('crosshair');
  }

  cancel(): void {
    this.from = this.to = null;
    this.host.overlay(null);
  }

  private dragged(a = this.from, b = this.to): boolean {
    return !!a && !!b && Math.max(Math.abs(b.x - a.x), Math.abs(b.y - a.y)) * this.host.scale() >= DRAG_PX * 2;
  }

  private sheet() {
    const id = this.host.sheet();
    return this.host.book().book.sheets.find((s) => s.id === id);
  }

  /** A click's frame: the kind's usual size with its top left corner at the point; a sheet frame, the margins' box. */
  private clickFrame(p: PaperPoint): RectUm {
    const sheet = this.sheet();
    if (this.kind === 'border' && sheet) {
      const { size, margins: m } = sheet.page;
      return { left: m.left, top: m.top, width: size.width - m.left - m.right, height: size.height - m.top - m.bottom };
    }
    const [w, h] = CLICK_SIZE[this.kind] ?? [40, 30];
    return { left: toUm(p.x), top: toUm(p.y), width: toUm(w), height: toUm(h) };
  }

  /** A name the sheet does not have yet: the tool's, then “2”, “3” … */
  private nameFor(items: readonly Item[]): string {
    const names = new Set(items.map((i) => i.name));
    if (!names.has(this.label)) return this.label;
    for (let n = 2; ; n++) if (!names.has(`${this.label} ${n}`)) return `${this.label} ${n}`;
  }

  private place(frame: RectUm): void {
    const sheet = this.sheet();
    if (!sheet) return;
    const id = this.host.newId();
    const maps = sheet.items.filter((i) => i.kind.type === 'map');
    const link = READS_MAP.has(this.kind) || this.tool === 'overviewMap' ? maps[0]?.id : undefined;
    let item: Item;
    try {
      item = this.host.engine.newItem(this.host.workspace(), this.host.capabilities(), {
        tool: this.tool,
        ...(this.preset ? { preset: this.preset } : {}),
        id,
        name: this.nameFor(sheet.items),
        frame: { ...frame, width: Math.max(1000, frame.width), height: Math.max(1000, frame.height) },
        ...(link ? { link } : {}),
      });
    } catch (e) {
      this.host.say(`${this.label} eklenemedi: ${errorText(e)}`);
      return;
    }
    // A new map looks where the drawing area looks (its centre can be taken again with “Görünümden al”).
    if (item.kind.type === 'map' && item.kind.view.type === 'fixed' && !item.kind.view.center) item = { ...item, kind: { ...item.kind, view: { ...item.kind.view, center: this.host.mapCenter() } } };
    if (this.host.apply([{ op: 'addItems', to: { kind: 'sheet', id: sheet.id }, items: [item] }], `Ekle: ${item.name}`)) {
      this.host.state.select([id]);
      this.host.done();
    }
  }
}
