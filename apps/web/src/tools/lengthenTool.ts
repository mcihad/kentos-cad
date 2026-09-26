import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { lengthenEntity, lengthOf, lengthToward, nearEnd } from '../model/ops/lengthen';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { EdgePickTool } from './edgeTools';
import { editGeometry, uidOf, writeEdit } from './editCommand';
import { drawTag, strokeGeometry } from './preview';
import type { ToolPointer } from './Tool';

type Mode = 'dynamic' | 'delta' | 'percent' | 'total';
const MODE_NAME: Record<Mode, string> = { dynamic: 'dinamik', delta: 'fark', percent: 'yüzde', total: 'toplam' };

/**
 * Uzat-kısalt (LENGTHEN): click near the end to change. Dynamic (default)
 * then follows the mouse — past the end the end segment continues, inside
 * the path it is cut back — and a typed number is the new total length.
 * Fark, yüzde and toplam apply at once to every end clicked.
 */
export class LengthenTool extends EdgePickTool {
  readonly id = 'lengthen';
  private static mode: Mode = 'dynamic';
  private static values = { delta: 1, percent: 100, total: 10 };
  private ask: Exclude<Mode, 'dynamic'> | null = null;
  private target: { e: Entity; atEnd: boolean } | null = null;
  private mouse: Vec2 | null = null;

  protected override editable = (e: Entity) => lengthOf(e) !== null && !this.ctx.doc.layers.isLocked(e.layerId);

  override get snaps(): boolean {
    return !!this.target;
  }

  protected refresh(): void {
    const f = this.ctx.format;
    const v = LengthenTool.values;
    let step: string;
    if (this.ask === 'delta') step = 'uzunluk farkını yazın (eksi kısaltır)';
    else if (this.ask === 'percent') step = 'yeni uzunluğu eski uzunluğun yüzdesi olarak yazın (100 = aynı)';
    else if (this.ask === 'total') step = 'yeni toplam uzunluğu yazın';
    else if (this.target) step = 'yeni ucu fareyle gösterin ya da toplam uzunluğu yazın';
    else
      step = `değiştirilecek ucun yakınına tıklayın [kip: ${MODE_NAME[LengthenTool.mode]}; Dinamik (D) / Fark (F): ${f.length(v.delta)} / Yüzde (Y): ${v.percent} / Toplam (T): ${f.length(v.total)}]`;
    this.prompt.set(`Uzat-kısalt: ${step}`);
    this.ctx.view.requestOverlay();
  }

  private lengthFor(e: Entity): number {
    const L = lengthOf(e) ?? 0;
    const v = LengthenTool.values;
    switch (LengthenTool.mode) {
      case 'delta':
        return L + v.delta;
      case 'percent':
        return (L * v.percent) / 100;
      default:
        return v.total;
    }
  }

  override pointerMove(p: ToolPointer): void {
    this.mouse = p.world;
    if (this.target) return this.ctx.view.requestOverlay();
    super.pointerMove(p);
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (this.target) {
      const L = lengthToward(this.target.e, this.target.atEnd, p.world);
      if (L !== null) this.apply(this.target.e, this.target.atEnd, L);
      this.target = null;
      return this.refresh();
    }
    const e = this.ctx.view.pickEdge(p.screen, this.editable);
    if (!e) return this.ctx.log.warn('Düzenlenebilir bir çizgiye, yaya ya da açık çoklu çizgiye tıklayın.');
    const atEnd = nearEnd(e, p.raw);
    if (LengthenTool.mode === 'dynamic') {
      this.target = { e, atEnd };
      this.ctx.selection.hover.set(null);
      return this.refresh();
    }
    this.apply(e, atEnd, this.lengthFor(e));
  }

  private apply(e: Entity, atEnd: boolean, length: number): void {
    const r = lengthenEntity(e, atEnd, length);
    if ('error' in r) return this.ctx.log.warn(r.error);
    const before = lengthOf(e) ?? 0;
    // The whole geometry is written, through the edit command (docs/adr/0047): the result may drop or change bulges.
    if (!writeEdit(this.ctx, 'lengthen', [{ kind: 'update', uid: uidOf(this.ctx, e), geometry: editGeometry(r.geometry) }])) return;
    const f = this.ctx.format;
    this.ctx.log.success(`Uzunluk ${f.length(before)} → ${f.length(length)}.`);
    this.hover = null;
  }

  input(text: string): boolean {
    const t = text.trim().toLocaleUpperCase('tr-TR');
    const modes: Record<string, Mode> = { D: 'dynamic', F: 'delta', Y: 'percent', T: 'total' };
    if (modes[t] && !this.target) {
      LengthenTool.mode = modes[t];
      this.ask = t === 'D' ? null : (modes[t] as Exclude<Mode, 'dynamic'>);
      this.refresh();
      return true;
    }
    const n = parseNumber(text);
    if (n === null) return false;
    if (this.ask) {
      if (this.ask !== 'delta' && !(n > 0)) {
        this.ctx.log.warn('Değer sıfırdan büyük olmalı.');
        return true;
      }
      LengthenTool.values[this.ask] = n;
      this.ask = null;
      this.refresh();
      return true;
    }
    if (this.target) {
      this.apply(this.target.e, this.target.atEnd, n);
      this.target = null;
      this.refresh();
      return true;
    }
    return false;
  }

  confirm(): void {
    if (this.target || this.ask) {
      this.target = null;
      this.ask = null;
      return this.refresh();
    }
    this.ctx.tools.exit();
  }

  cancel(): boolean {
    if (!this.target && !this.ask) return false;
    this.target = null;
    this.ask = null;
    this.refresh();
    return true;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    const show = (e: Entity, atEnd: boolean, length: number | null, at: Vec2) => {
      if (length === null) return;
      const r = lengthenEntity(e, atEnd, length);
      if ('error' in r) return;
      strokeGeometry(g, view, r.geometry, { color: pal.accent, width: 2 });
      drawTag(g, view.worldToScreen(at), [`${f.length(lengthOf(e) ?? 0)} → ${f.length(length)}`], pal.accent, pal.labelHalo);
    };
    if (this.target && this.mouse) return show(this.target.e, this.target.atEnd, lengthToward(this.target.e, this.target.atEnd, this.mouse), this.mouse);
    if (this.hover && LengthenTool.mode !== 'dynamic') show(this.hover.entity, nearEnd(this.hover.entity, this.hover.world), this.lengthFor(this.hover.entity), this.hover.world);
  }
}
