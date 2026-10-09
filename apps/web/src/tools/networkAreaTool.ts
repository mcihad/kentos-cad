import { fixed } from '../core/displayNumber';
import type { EntityGeometry } from '../model/entities';
import { lineGeometry, refusalWords, type NetworkServiceArea } from '../model/networkAnswers';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { NetworkTool, networkOptions, REACH_PX } from './networkTool';
import { AREA_LAYER, AREA_LINES_LAYER, writeResults, type ResultWrite } from './resultLayer';

/**
 * Hizmet alanı (docs/adr/0209 §6): facilities clicked; the network within each break of the chosen cost shows at once
 * (its lines, asked first) and then its areas (the lines' buffers, Kenar payı wide; each band paler than the one inside
 * it). Aralıklar (R) are rising numbers in the cost's unit; Yön (Y) from or to the facilities; Biçim (B) discs or rings;
 * Birleşik (İ) one area a band for all facilities or one each; Çizgiler (Ç) writes the lines too. Enter writes the areas
 * to Hizmet alanı (and the lines to Hizmet alanı çizgileri) in one step. The desktop's
 * `kentos_interaction::network::service_area`.
 */

export const LABEL = 'Hizmet alanı';
const MAX_BREAKS = 10;

type Areas = Extract<NetworkServiceArea, { lines: unknown }>;
type Asking = 'breaks' | 'trim';

/** A number as a label shows it: the display rule's, without trailing zeros (“0–300”). */
const plain = (v: number): string => fixed(v, 6).replace(/\.?0+$/, '');

export class NetworkAreaTool extends NetworkTool {
  readonly id = 'netServiceArea';
  protected readonly prefers = 'road' as const;
  protected readonly label = LABEL;
  private asking: Asking | null = null;
  private answer_: { value: Areas; paths: Float64Array; fills: Float64Array } | null = null;
  private refusal: string | null = null;

  /** The kind of the cost used: 0 the length, 1 a speed, 2 a field (the breaks are kept for each). */
  private costKind(): number {
    const c = this.costIndex();
    if (c === 0) return 0;
    return this.network()?.costs?.[c - 1]?.kind === 'speed' ? 1 : 2;
  }

  /** The breaks of the cost used, in its unit: the ones last typed for its kind, else the kind's. */
  private breaks(): number[] {
    const kind = this.costKind();
    return networkOptions.breaks[kind] ?? [[500, 1000, 1500], [5, 10, 15], [10, 20, 30]][kind];
  }

  protected ask(gen: number): void {
    const id = this.networkId()!;
    const facilities = this.stops();
    this.answer_ = null;
    this.refusal = null;
    if (!facilities.length) return;
    const O = networkOptions;
    const q = {
      kind: 'area' as const,
      facilities,
      breaks: this.breaks(),
      reach: this.reach(),
      cost: this.costIndex(),
      toward: O.toward,
      separate: O.separate,
      barriers: this.barriers(),
      trim: O.trim,
      rings: O.rings,
    };
    const fail = (e: Error) => gen === this.gen && this.fail(e);
    const take = ({ value, paths, fills }: { value: NetworkServiceArea; paths: Float64Array; fills: Float64Array }) => {
      if (gen !== this.gen) return;
      if ('error' in value) {
        if (this.refusal === null) this.ctx.log.warn(`${LABEL}: ${refusalWords(value, `${REACH_PX} piksel`)}`);
        this.refusal = refusalWords(value, `${REACH_PX} piksel`);
        this.answer_ = null;
      } else if (!this.answer_ || fills.length) this.answer_ = { value, paths, fills };
      this.refresh();
    };
    // The lines first (they come at once), then the areas.
    this.ctx.networks.ask<NetworkServiceArea>(id, { ...q, areas: false }).then(take, fail);
    this.ctx.networks.ask<NetworkServiceArea>(id, { ...q, areas: true }).then(take, fail);
  }

  protected refresh(): void {
    this.prompt.set(`${LABEL}: ${this.step()}`);
    this.ctx.view.requestOverlay();
  }

  /** The breaks as written, in the cost's unit. */
  private breaksText(): string {
    const c = this.costIndex();
    const unit = c === 0 ? this.ctx.format.lengthUnitLabel : this.network()?.costs?.[c - 1]?.kind === 'speed' ? 'dk' : (this.network()?.costs?.[c - 1]?.unit ?? '');
    const shown = c === 0 ? this.breaks().map((b) => this.ctx.format.fromMetres(b)) : this.breaks();
    return `${shown.map(plain).join(' ')}${unit ? ` ${unit}` : ''}`;
  }

  private step(): string {
    if (!this.networkId()) return this.noNetwork();
    if (this.asking === 'breaks') return `aralıkları artan sayılarla yazın, aralarında boşluk (ör. 5 10 15; en çok ${MAX_BREAKS}) [şimdi: ${this.breaksText()}]`;
    if (this.asking === 'trim') return `alanın çizgilerden payını yazın (şimdi: ${this.ctx.format.length(networkOptions.trim)})`;
    const O = networkOptions;
    const on = (b: boolean) => (b ? 'açık' : 'kapalı');
    const parts = [
      ...this.commonParts(),
      `Aralıklar (R): ${this.breaksText()}`,
      `Yön (Y): ${O.toward ? 'tesise' : 'tesisten'}`,
      `Biçim (B): ${O.rings ? 'halka' : 'disk'}`,
      `Birleşik (İ): ${on(!O.separate)}`,
      `Kenar payı (K): ${this.ctx.format.length(O.trim)}`,
      `Çizgiler (Ç): ${on(O.lines)}`,
      ...(this.answer_ ? ['Uygula (Enter)'] : []),
    ];
    const n = this.stops().length;
    let now: string;
    if (this.problem) now = this.problem;
    else if (n === 0) now = 'tesise tıklayın ya da Y,X yazın';
    else if (this.refusal) now = this.refusal;
    else if (this.answer_) {
      const a = this.answer_.value;
      const reached = a.areas.filter((x) => x.shape).length;
      now = `${n} tesis; ${a.lines.length} çizgi${this.answer_.fills.length ? `, ${reached} alan` : ', alanlar hesaplanıyor'}; başka tesise tıklayın ya da Enter ile yazın`;
    } else now = `${n} tesis; hesaplanıyor`;
    return `${now} [${parts.join(' / ')}]`;
  }

  protected override option(key: string): 'ask' | 'show' | null {
    const O = networkOptions;
    switch (key) {
      case 'R':
        this.asking = 'breaks';
        return 'show';
      case 'K':
        this.asking = 'trim';
        return 'show';
      case 'Y':
        O.toward = !O.toward;
        return 'ask';
      case 'B':
        O.rings = !O.rings;
        return 'ask';
      case 'İ':
        O.separate = !O.separate;
        return 'ask';
      case 'Ç':
        O.lines = !O.lines;
        return 'show';
      default:
        return super.option(key);
    }
  }

  /** A value typed for what is asked: a wrong one is said and asked again. */
  protected override answer(text: string): boolean {
    if (!this.asking) return false;
    const refused = (why: string) => this.ctx.log.warn(`${why}; “${text}” yazıldı.`);
    if (this.asking === 'breaks') {
      const values = text.split(/[\s;]+/).filter(Boolean).map(parseNumber);
      const ok = values.length > 0 && values.length <= MAX_BREAKS && values.every((v, i) => v !== null && v > 0 && Number.isFinite(v) && (i === 0 || v > values[i - 1]!));
      if (!ok) {
        refused(`Aralıklar artan, sıfırdan büyük sayılar olmalı (en çok ${MAX_BREAKS})`);
        return true;
      }
      const c = this.costIndex();
      networkOptions.breaks[this.costKind()] = values.map((v) => (c === 0 ? this.ctx.format.toMetres(v!) : v!));
    } else {
      const v = parseNumber(text);
      if (v === null || !(v > 0) || !Number.isFinite(v) || this.ctx.format.toMetres(v) > 10000) {
        refused('Kenar payı sıfırdan büyük bir uzunluk olmalı (en çok 10 km)');
        return true;
      }
      networkOptions.trim = this.ctx.format.toMetres(v);
    }
    this.asking = null;
    this.changed();
    return true;
  }

  override input(text: string): boolean {
    // While a value is asked, everything typed is that value (Aralıklar has spaces).
    if (this.asking) return this.answer(text.trim());
    return super.input(text);
  }

  takesWords(): boolean {
    return this.asking === 'breaks';
  }

  /** Enter: a value being asked ends; with areas, they are written and the facilities go; with none the tool leaves. */
  confirm(): void {
    if (this.asking) {
      this.asking = null;
      return this.refresh();
    }
    if (!this.points.length) return this.ctx.tools.exit();
    const a = this.answer_;
    if (!a || !a.fills.length) return this.ctx.log.warn(`${LABEL}: ${this.refusal ?? 'alanlar henüz hesaplanmadı; biraz bekleyip yeniden deneyin.'}`);
    const n = this.network()!;
    const c = this.costIndex();
    const cost = this.costNames()[c];
    const breaks = this.breaks();
    const decimals = this.ctx.doc.settings.lengthDecimals.value;
    const number = (v: number) => fixed(v, c === 0 ? decimals : 2);
    const all = this.stops().map((_, i) => i + 1).join(', ');
    const facility = (f: number | null | undefined) => (f === null || f === undefined ? all : String(f + 1));
    const writes: ResultWrite[] = [
      {
        layer: AREA_LAYER,
        objects: a.value.areas
          .filter((x) => x.shape)
          .map((x) => ({
            geometry: x.shape as EntityGeometry,
            attrs: { Ağ: n.name, Maliyet: cost, Tesis: facility(x.facility), Başlangıç: number(networkOptions.rings && x.band > 0 ? breaks[x.band - 1] : 0), Bitiş: number(breaks[x.band]) },
          })),
      },
    ];
    if (networkOptions.lines)
      writes.push({
        layer: AREA_LINES_LAYER,
        objects: a.value.lines.map((l) => ({
          geometry: lineGeometry(l.line),
          attrs: { Ağ: n.name, Maliyet: cost, Tesis: facility(l.facility), Aralık: `${plain(l.band > 0 ? breaks[l.band - 1] : 0)}–${plain(breaks[l.band])}` },
        })),
      });
    const ids = writeResults(this.ctx, LABEL, writes);
    if (!ids) return;
    this.ctx.log.success(`${LABEL}: ${writes[0].objects.length} alan${networkOptions.lines ? ` ve ${a.value.lines.length} çizgi` : ''} yazıldı.`);
    this.points = [];
    this.changed();
  }

  /** Esc: a value being asked goes first. */
  override cancel(): boolean {
    if (this.asking) {
      this.asking = null;
      this.refresh();
      return true;
    }
    return super.cancel();
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const a = this.answer_;
    if (a) {
      this.drawFills(g, view, a.fills, this.breaks().length);
      this.drawPaths(g, view, a.paths, pal.accent, 1.5);
    }
    this.drawPoints(g, view, true);
  }
}
