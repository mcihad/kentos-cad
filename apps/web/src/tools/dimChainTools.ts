import { lookOfDimension } from '../model/annotationStyles';
import type { DimensionEntity, Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { layoutDimension, type DimensionGeom } from '../model/geom/dimension';
import { baselineDimension, continueDimension } from '../model/ops/construct';
import type { ViewTransform } from '../viewport/Camera';
import { ringMark } from './constructPreview';
import { PointInputTool } from './drawTools';
import { drawTag, strokeLayout } from './preview';
import type { ToolPointer } from './Tool';

/**
 * Zincir ölçü and Baz ölçü (docs/adr/0140): more dimensions along the one just drawn.
 * They start from the last aligned or linear dimension drawn in this session (by any
 * of the dimension tools) or ask to click one; each click then adds one dimension, one
 * undo step each, and right click or Enter ends.
 *
 * - Zincir: every new point is measured from the point before it, on the base's line
 *   (the first from the base's second point).
 * - Baz: every new point is measured from the base's first point; the lines lie one
 *   above the other, the spacing between them three times the base's text height, the
 *   level growing by one per dimension.
 *
 * The geometry of each new dimension is the shared core's (`continueDimension`,
 * `baselineDimension`); it is written through `cad.entities.create` as the Ölçülendirme
 * tool writes, the undo step named after the tool.
 */

/** The persistent id of the newest aligned or linear dimension written, for Zincir and Baz ölçü to start from. */
let lastDimensionUid: string | null = null;

/** Remembers a dimension just written (the dimension tools call it); an angular, radius or diameter one is no base. */
export function rememberDimension(uid: string | undefined, style: DimensionGeom['style']): void {
  if (uid !== undefined && (style === undefined || style === 'aligned' || style === 'linear')) lastDimensionUid = uid;
}

/** Forgets it (a new document). */
export function forgetDimension(): void {
  lastDimensionUid = null;
}

/** Whether a dimension can be a base: measured along a line. */
const isBase = (e: Entity | undefined): e is DimensionEntity => e?.kind === 'dimension' && (e.style === undefined || e.style === 'aligned' || e.style === 'linear');

/** A base's geometry and its style and look: the next ones are written in them (AutoCAD's DIMCONTINUEMODE 1, docs/adr/0183 §4). */
const geomOf = (e: DimensionEntity): DimensionGeom => ({
  a: e.a,
  b: e.b,
  offset: e.offset,
  height: e.height,
  ...(e.style && { style: e.style }),
  ...(e.angle !== undefined && { angle: e.angle }),
  ...lookOfDimension(e),
});

abstract class DimensionAlongTool extends PointInputTool {
  protected abstract readonly verb: string;
  /** The undo step's name, as the create command has it. */
  protected abstract readonly operation: 'dimensionChain' | 'dimensionBaseline';
  /** The dimension the next ones follow. */
  protected base: DimensionGeom | null = null;
  /** Picking another base by clicking a dimension. */
  private picking = false;
  /** Dimensions written since the base was taken. */
  protected count = 0;
  /** Chain: where the next one starts. */
  protected from: Vec2 | null = null;

  /** The new dimension for a point, or null when it makes none. */
  protected abstract next(p: Vec2): DimensionGeom | null;
  /** After one was written. */
  protected abstract advance(p: Vec2): void;
  protected abstract start(base: DimensionGeom): void;

  override get snaps(): boolean {
    return !this.picking && !!this.base;
  }

  override activate(): void {
    const e = lastDimensionUid !== null ? this.ctx.doc.byUid(lastDimensionUid) : undefined;
    if (isBase(e)) this.take(geomOf(e), 'Son çizilen ölçüden devam ediliyor.');
    else if (lastDimensionUid !== null) this.ctx.log.warn('Son çizilen ölçü artık çizimde yok; devam edilecek ölçüye tıklayın.');
    this.picking = !this.base;
    super.activate();
  }

  private take(base: DimensionGeom, said: string): void {
    this.base = base;
    this.count = 0;
    this.picking = false;
    this.start(base);
    this.ctx.log.info(said);
  }

  protected override get last(): Vec2 | null {
    return this.from;
  }

  protected promptFor(): string {
    if (this.picking || !this.base) return `devam edilecek hizalı ya da doğrusal ölçüye tıklayın${this.base ? ' [Vazgeç (Esc)]' : ''}`;
    return 'sonraki ölçünün noktasını belirtin [Ölçü seç (S) / Bitir (Enter)]';
  }

  protected onPoint(p: Vec2): void {
    if (!this.base) return;
    const g = this.next(p);
    const l = g && layoutDimension(g);
    if (!g || !l) return void this.ctx.log.warn('Bu noktadan ölçü oluşmuyor; nokta ölçülen noktayla çakışıyor. Başka bir yer gösterin.');
    const { style, ...rest } = g;
    const out = this.writeObjects([{ kind: 'dimension', ...rest, ...(style && style !== 'aligned' && { style }) }], this.operation);
    if (!out) return;
    rememberDimension(this.ctx.doc.uidOf(out.ids[0]), style);
    this.count++;
    this.ctx.log.success(`${this.verb} eklendi: ${this.ctx.view.dimensionText(l, lookOfDimension(g))}`);
    this.advance(p);
  }

  override pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (!this.picking && this.base) return super.pointerDown(p);
    const e = this.ctx.view.pick(p.screen) ?? undefined;
    if (!isBase(e)) return void this.ctx.log.warn('Hizalı ya da doğrusal bir ölçüye tıklayın; açı, yarıçap ve çap ölçüleri buna uymaz.');
    this.ctx.selection.hover.set(null);
    this.take(geomOf(e), 'Ölçü seçildi; sonraki noktaları gösterin.');
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
  }

  override pointerMove(p: ToolPointer): void {
    if (!this.picking && this.base) return super.pointerMove(p);
    this.ctx.selection.hover.set(this.ctx.view.pick(p.screen)?.id ?? null);
    this.hover = p.world;
  }

  override acceptPoint(p: Vec2): boolean {
    if (this.picking || !this.base) return false;
    return super.acceptPoint(p);
  }

  protected override option(key: string): boolean {
    if (key !== 'S' || this.picking) return false;
    this.picking = true;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    if (this.picking || !this.base) return false;
    return super.input(text);
  }

  /** Right click or Enter ends: what was added is said once. */
  override confirm(): void {
    if (this.count) this.ctx.log.info(`${this.verb}: ${this.count} ölçü eklendi.`);
    this.count = 0;
    this.picking = false;
    this.base = null;
    this.ctx.tools.exit();
  }

  /** Esc: back from picking another base to the one in use; else the tool leaves. */
  cancel(): boolean {
    if (this.picking && this.base) {
      this.picking = false;
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    return false;
  }

  override undoStep(): boolean {
    if (this.undoLastMade()) {
      this.count = Math.max(0, this.count - 1);
      this.revert();
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    return false;
  }

  /** After a dimension was taken back by Ctrl+Z. */
  protected abstract revert(): void;

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    if (this.picking || !this.base || !this.hover) return;
    const start = this.startPoint();
    if (start) ringMark(g, view, start, pal.snap);
    const d = this.next(this.hover);
    const l = d && layoutDimension(d);
    if (!l) return;
    strokeLayout(g, view, l, pal.accent);
    drawTag(g, view.worldToScreen(this.hover), [this.ctx.view.dimensionText(l, lookOfDimension(d))], pal.accent, pal.labelHalo);
    this.drawTracking(g, view);
  }

  /** Where the next dimension is measured from, marked. */
  protected abstract startPoint(): Vec2 | null;
}

export class DimContinueTool extends DimensionAlongTool {
  readonly id = 'dimContinue';
  protected readonly label = 'Zincir ölçü';
  protected readonly verb = 'Zincir ölçü';
  protected readonly operation = 'dimensionChain';
  /** The chain's points, the base's second point first; Ctrl+Z takes the last back. */
  private chain: Vec2[] = [];

  protected start(base: DimensionGeom): void {
    this.chain = [base.b];
    this.from = base.b;
  }
  protected next(p: Vec2): DimensionGeom | null {
    return this.base && this.from ? continueDimension(this.base, this.from, p) : null;
  }
  protected advance(p: Vec2): void {
    this.chain.push(p);
    this.from = p;
  }
  protected revert(): void {
    if (this.chain.length > 1) this.chain.pop();
    this.from = this.chain.at(-1) ?? null;
  }
  protected startPoint(): Vec2 | null {
    return this.from;
  }
}

export class DimBaselineTool extends DimensionAlongTool {
  readonly id = 'dimBaseline';
  protected readonly label = 'Baz ölçü';
  protected readonly verb = 'Baz ölçü';
  protected readonly operation = 'dimensionBaseline';
  /** The level the next line takes: one beyond the base's for the first, growing by one each. */
  private level = 1;

  protected start(base: DimensionGeom): void {
    this.level = 1;
    this.from = base.a;
  }
  /** Three text heights between the lines. */
  private spacing(): number {
    return 3 * (this.base?.height ?? 0);
  }
  protected next(p: Vec2): DimensionGeom | null {
    return this.base ? baselineDimension(this.base, p, this.level, this.spacing()) : null;
  }
  protected advance(): void {
    this.level++;
  }
  protected revert(): void {
    this.level = Math.max(1, this.level - 1);
  }
  protected startPoint(): Vec2 | null {
    return this.base?.a ?? null;
  }
}
