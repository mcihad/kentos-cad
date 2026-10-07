import type { ArrangeMode } from '../contracts/generated/ArrangeMode';
import type { Entity } from '../model/entities';
import type { Bounds, Vec2 } from '../model/geometry';
import { translation } from '../model/geom/affine';
import { aligns, arrangeAt, arrangeBoxes, arrangeMoves, arrangeUnion, eastward } from '../model/ops/arrange';
import type { ViewTransform } from '../viewport/Camera';
import { MAX_GHOSTS, SelectionFirstTool } from './modifyTools';
import { strokePath, strokePaths } from './preview';
import type { ToolPointer } from './Tool';

/**
 * Hizala ve dağıt (docs/adr/0194 §1; the desktop's `kentos_interaction::align_distribute`): on the modify tools'
 * base, the objects first (or the selection); the method's letter (Sola S, Ortala O, Sağa A, Üste Ü, Ortaya R, Alta T,
 * Yatay dağıt Y, Dikey dağıt D) switches it, while picking too (the ribbon's methods type it). An alignment waits for
 * its reference: a click on an object takes its box, a click elsewhere or a typed point that point, Enter the
 * selection's box; a spread writes on Enter or a click. The preview: every selected object's outline where it would
 * go, dashed, and the reference's line. The boxes and the moves are the core's (`ops::arrange`); written through
 * `cad.entities.transform`'s `arrange`, one undo step named after the method; the tool leaves, the selection stays.
 * Nothing that would not move is written.
 */

/** The methods' chips: their words and letters, in the tool's order. */
const CHIPS: readonly [ArrangeMode, string, string][] = [
  ['left', 'Sola', 'S'],
  ['center', 'Ortala', 'O'],
  ['right', 'Sağa', 'A'],
  ['top', 'Üste', 'Ü'],
  ['middle', 'Ortaya', 'R'],
  ['bottom', 'Alta', 'T'],
  ['horizontal', 'Yatay dağıt', 'Y'],
  ['vertical', 'Dikey dağıt', 'D'],
];

/** A move shorter than this is no move: the selection is already where it would go. */
const STILL = 1e-9;

const DONE: Record<ArrangeMode, (n: number) => string> = {
  left: (n) => `${n} nesne sola hizalandı.`,
  center: (n) => `${n} nesne ortalandı.`,
  right: (n) => `${n} nesne sağa hizalandı.`,
  top: (n) => `${n} nesne üste hizalandı.`,
  middle: (n) => `${n} nesne ortaya hizalandı.`,
  bottom: (n) => `${n} nesne alta hizalandı.`,
  horizontal: (n) => `${n} nesne yatay olarak eşit aralıkla dağıtıldı.`,
  vertical: (n) => `${n} nesne dikey olarak eşit aralıkla dağıtıldı.`,
};

/** What an alignment meets. */
type Reference = { box: Bounds } | { point: Vec2 } | 'selection';

export class AlignDistributeTool extends SelectionFirstTool {
  readonly id = 'alignDistribute';
  protected readonly label = 'Hizala ve dağıt';
  /** The method, kept for this run only, so the ribbon's methods each start the tool the way they say. */
  private mode: ArrangeMode = 'left';
  /** The selection off locked layers and their boxes, measured once it is confirmed. */
  private ids: number[] = [];
  private boxes: Bounds[] = [];
  /** The reference the preview shows (an alignment), or none yet. */
  private shown: Reference | null = null;

  protected begin(): void {
    const { doc } = this.ctx;
    const open = this.targets().filter((e) => !doc.layers.isLocked(e.layerId));
    this.ids = open.map((e) => e.id);
    this.boxes = open.length ? arrangeBoxes(open, doc.blocks.value, doc.settings.drawingFont.value) : [];
    this.shown = 'selection';
  }

  private chips(): string {
    return `[${CHIPS.map(([m, word, key]) => `${word} (${key})${m === this.mode ? ': açık' : ''}`).join(' / ')}]`;
  }

  protected stagePrompt(): string {
    return aligns(this.mode) ? 'başvuru nesnesine ya da noktaya tıklayın; Enter seçimin sınırına göre hizalar' : 'Enter ile eşit aralıkla dağıtın';
  }

  protected override refresh(): void {
    const n = this.ctx.selection.size;
    const step = this.picking ? this.pickStep(n) : this.stagePrompt();
    this.prompt.set(`${this.label}: ${step} ${this.chips()}`);
    this.ctx.view.requestOverlay();
  }

  /** A method's letter, while picking too; else a point (the base's). */
  override input(text: string): boolean {
    const key = text.trim().toLocaleUpperCase('tr-TR');
    const chip = CHIPS.find(([, , k]) => k === key);
    if (chip) {
      this.mode = chip[0];
      this.shown = 'selection';
      this.refresh();
      return true;
    }
    return super.input(text);
  }

  /** A click on an object takes its box; elsewhere the base's point (snapped). A spread writes on any click. */
  override pointerDown(p: ToolPointer): void {
    if (this.picking || p.button !== 0) return super.pointerDown(p);
    if (!aligns(this.mode)) return void this.write('selection');
    const hit = this.ctx.view.pick(p.screen);
    if (hit) return void this.write({ box: this.boxOf(hit) });
    super.pointerDown(p);
  }

  override pointerMove(p: ToolPointer): void {
    super.pointerMove(p);
    if (this.picking) return;
    const hit = aligns(this.mode) ? this.ctx.view.pick(p.screen) : null;
    this.shown = hit ? { box: this.boxOf(hit) } : { point: p.world };
  }

  protected point(p: Vec2): void {
    this.write(aligns(this.mode) ? { point: p } : 'selection');
  }

  override confirm(): void {
    if (this.picking) return super.confirm();
    this.write('selection');
  }

  private boxOf(e: Entity): Bounds {
    const i = this.ids.indexOf(e.id);
    if (i >= 0) return this.boxes[i];
    return arrangeBoxes([e], this.ctx.doc.blocks.value, this.ctx.doc.settings.drawingFont.value)[0];
  }

  /** The easting or northing an alignment meets: the box's, else the point's, else the selection's box's. */
  private at(r: Reference): number | null {
    if (r === 'selection') {
      const u = this.boxes.length ? arrangeUnion(this.boxes) : null;
      return u ? arrangeAt(u, this.mode) : null;
    }
    if ('box' in r) return arrangeAt(r.box, this.mode);
    return eastward(this.mode) ? r.point.x : r.point.y;
  }

  /** Writes the moves through the command; the tool leaves once written. */
  private write(r: Reference): void {
    const { log } = this.ctx;
    const at = aligns(this.mode) ? this.at(r) : null;
    if (aligns(this.mode) && at === null) return;
    if (this.ids.length && (aligns(this.mode) || this.ids.length >= 3)) {
      const moves = arrangeMoves(this.boxes, this.mode, at);
      if (moves.every((d) => Math.abs(d.x) <= STILL && Math.abs(d.y) <= STILL)) {
        log.info(aligns(this.mode) ? 'Seçilenler zaten hizalı; bir şey değişmedi.' : 'Seçilenler zaten eşit aralıklı; bir şey değişmedi.');
        return this.ctx.tools.exit();
      }
    }
    const n = this.transformSelection({ kind: 'arrange', mode: this.mode, ...(at !== null && { at }) }, false);
    if (n === null) return this.refresh();
    log.success(DONE[this.mode](n));
    this.ctx.tools.exit();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    super.draw(g, view);
    if (this.picking || !this.ids.length) return;
    const r = this.shown;
    const at = aligns(this.mode) ? (r ? this.at(r) : null) : null;
    if (aligns(this.mode) && at === null) return;
    const pal = this.ctx.view.palette;
    const moves = arrangeMoves(this.boxes, this.mode, at);
    this.ids.slice(0, MAX_GHOSTS + 1).forEach((id, i) => {
      strokePaths(g, view, this.ctx.view.ghosts([id], [translation(moves[i].x, moves[i].y)], MAX_GHOSTS), { color: pal.accent, dash: [4, 3] });
    });
    const u = arrangeUnion(this.boxes);
    if (at === null || !u) return;
    const span = r && r !== 'selection' && 'box' in r ? (arrangeUnion([u, r.box]) ?? u) : u;
    const pad = Math.max(span.maxX - span.minX, span.maxY - span.minY) * 0.05;
    const line: Vec2[] = eastward(this.mode)
      ? [
          { x: at, y: span.minY - pad },
          { x: at, y: span.maxY + pad },
        ]
      : [
          { x: span.minX - pad, y: at },
          { x: span.maxX + pad, y: at },
        ];
    strokePath(g, view, line, { color: pal.accent });
    if (r && r !== 'selection' && 'box' in r) {
      const b = r.box;
      strokePath(
        g,
        view,
        [
          { x: b.minX, y: b.minY },
          { x: b.maxX, y: b.minY },
          { x: b.maxX, y: b.maxY },
          { x: b.minX, y: b.maxY },
        ],
        { color: pal.accent, closed: true, dash: [4, 3] },
      );
    }
  }
}
