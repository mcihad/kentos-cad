import type { DisplayList } from '../../contracts/generated/sheet/DisplayList';
import type { Item } from '../../contracts/generated/sheet/Item';
import type { Sheet } from '../../contracts/generated/sheet/Sheet';
import { fixed } from '../../core/displayNumber';
import { mm } from '../../product/sheet/adapter';
import { paintList, type PaintSources } from '../../render/sheet/painter';
import { boundsOf, paintAids, paintChoice, paintDesk, paintMarquee, paintPaperEdge, type FrameMm, type PaperColors } from '../../render/sheet/paperPainter';
import type { PaperViewport } from '../../render/sheet/paperView';
import { badge, paintGaps, paintGuideDrag, paintNewFrame, paintSnapLines, paintSpacing } from '../../render/sheet/snapPainter';
import type { ToolOverlay } from '../../tools/sheet/tool';

/**
 * One picture of the desk (docs/sheet/design.md §11): the desk and the
 * paper, the engine's plan of the sheet (or of the drag's copy), the paper's
 * edge, the design aids (margins, grid, guides), the item under the pointer,
 * the choice and its handles, and the tool's aids (smart guides, distances,
 * equal spacing, a box, a new frame, a guide, an angle). Apart from the
 * component so the workspace stays the size of what it does.
 */

export interface StageScene {
  readonly sheet: Sheet;
  readonly list: DisplayList | null;
  readonly view: PaperViewport;
  readonly chosen: ReadonlySet<string>;
  readonly hover: string | null;
  readonly overlay: ToolOverlay | null;
  /** The round handle is offered (one unlocked item chosen, the select tool in hand). */
  readonly handles: 'none' | 'resize' | 'rotate';
}

/** An item's frame in millimetres and degrees. */
export const frameOf = (i: Item): FrameMm => ({ frame: { left: mm(i.frame.left), top: mm(i.frame.top), width: mm(i.frame.width), height: mm(i.frame.height) }, rotation: i.rotation / 1000 });

/** The chosen items' frames (a group's as its children's union, which the engine keeps in its frame). */
export function chosenFrames(sheet: Sheet, chosen: ReadonlySet<string>): FrameMm[] {
  return sheet.items.filter((i) => chosen.has(i.id)).map(frameOf);
}

const num = (v: number) => fixed(v, 1);

export function paintStage(g: CanvasRenderingContext2D, size: { width: number; height: number }, dpr: number, s: StageScene, src: PaintSources, c: PaperColors): void {
  const p = s.sheet.page;
  const paper = { width: mm(p.size.width), height: mm(p.size.height), margins: { left: mm(p.margins.left), top: mm(p.margins.top), right: mm(p.margins.right), bottom: mm(p.margins.bottom) } };
  paintDesk(g, size.width, size.height, dpr, s.view, paper, c);
  if (s.list) {
    // The plan inside the paper only, in device pixels: micrometres times the zoom.
    g.save();
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.beginPath();
    g.rect(s.view.x, s.view.y, paper.width * s.view.scale, paper.height * s.view.scale);
    g.clip();
    paintList(g, s.list, { scale: (s.view.scale * dpr) / 1000, x: s.view.x * dpr, y: s.view.y * dpr, minLinePx: 0.75, minTextPx: 1.2, placeholderFont: c.font }, src);
    g.restore();
  }
  paintPaperEdge(g, dpr, s.view, paper, c);
  paintAids(g, dpr, s.view, paper, { guides: s.sheet.guides, grid: s.sheet.snapGrid.visible ? mm(s.sheet.snapGrid.spacing) : null }, c);
  const hover = s.hover && !s.chosen.has(s.hover) ? s.sheet.items.find((i) => i.id === s.hover) : undefined;
  paintChoice(g, dpr, s.view, { hover: hover ? frameOf(hover) : null, chosen: chosenFrames(s.sheet, s.chosen), handles: s.handles }, c);
  const o = s.overlay;
  if (!o) return;
  if (o.snap) {
    paintSnapLines(g, dpr, s.view, o.snap.lines, c);
    paintSpacing(g, dpr, s.view, o.snap.spacing, c);
    paintGaps(g, dpr, s.view, o.snap.gaps, c);
  }
  if (o.resize) {
    paintSnapLines(g, dpr, s.view, o.resize.lines, c);
    // The items whose width or height the new size matches: their box, and the size.
    const matched = o.resize.sizes.map((m) => s.sheet.items.find((i) => i.id === m.item)).filter((i): i is Item => !!i);
    paintChoice(g, dpr, s.view, { hover: null, chosen: matched.map(frameOf), handles: 'none' }, c);
    const box = boundsOf(chosenFrames(s.sheet, s.chosen));
    if (box && o.resize.sizes.length) {
      const m = o.resize.sizes[0];
      badge(g, { x: s.view.x + (box.left + box.width / 2) * s.view.scale, y: s.view.y + (box.top + box.height) * s.view.scale + 16 }, `${m.axis === 'x' ? 'Aynı genişlik' : 'Aynı yükseklik'} · ${num(mm(m.size))} mm`, c);
    }
  }
  if (o.marquee) paintMarquee(g, dpr, s.view, { left: mm(o.marquee.box.left), top: mm(o.marquee.box.top), width: mm(o.marquee.box.width), height: mm(o.marquee.box.height) }, o.marquee.crossing, c);
  if (o.frame) paintNewFrame(g, dpr, s.view, o.frame, c, num);
  if (o.guide) paintGuideDrag(g, dpr, s.view, o.guide, size, c, num);
  if (o.angle) {
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    badge(g, { x: s.view.x + o.angle.at.x * s.view.scale + 24, y: s.view.y + o.angle.at.y * s.view.scale - 16 }, `${fixed(o.angle.degrees, 1)}°`, c);
  }
}
