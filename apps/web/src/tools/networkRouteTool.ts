import { fixed } from '../core/displayNumber';
import type { Vec2 } from '../model/geometry';
import { lineGeometry, refusalWords, type NetworkRoute } from '../model/networkAnswers';
import type { NetworkReorder } from '../io/network/protocol';
import type { ViewTransform } from '../viewport/Camera';
import { drawTag } from './preview';
import { NetworkTool, networkOptions, REACH_PX } from './networkTool';
import { ROUTE_LAYER, writeResults } from './resultLayer';
import type { OptionChoice, ToolPointer } from './Tool';

/**
 * En kısa yol (docs/adr/0209 §5): stops clicked in turn; from the second on, the route through them shows, and the
 * way from the last stop to the cursor follows the pointer (the last stop's search tree is kept in the worker, the way
 * is read from it on each move). Sıra (S) puts the stops in the best order (the first kept, or the first and the last).
 * Enter writes the route to Rota (opened in that step when missing) as one polyline with its network, cost, stops,
 * order, length and each cost's total. The desktop's `kentos_interaction::network::route`.
 */

export const LABEL = 'En kısa yol';

const REORDERS: readonly { value: NetworkReorder; name: string }[] = [
  { value: 'none', name: 'yok' },
  { value: 'keepFirst', name: 'ilk sabit' },
  { value: 'keepFirstLast', name: 'ilk ve son sabit' },
];
const reorderName = (v: NetworkReorder) => REORDERS.find((r) => r.value === v)?.name ?? 'yok';

type Route = Extract<NetworkRoute, { order: readonly number[] }>;

export class NetworkRouteTool extends NetworkTool {
  readonly id = 'netRoute';
  protected readonly prefers = 'road' as const;
  protected readonly label = LABEL;
  private route: { value: Route; paths: Float64Array } | null = null;
  private refusal: string | null = null;
  /** Whether the worker keeps a search tree from the last stop. */
  private tree = false;
  private hover: Vec2 | null = null;
  private band: { cost: number; paths: Float64Array } | null = null;

  protected ask(gen: number): void {
    const id = this.networkId()!;
    const stops = this.stops();
    const barriers = this.barriers();
    const cost = this.costIndex();
    const reach = this.reach();
    this.route = null;
    this.refusal = null;
    this.tree = false;
    this.band = null;
    const fail = (e: Error) => gen === this.gen && this.fail(e);
    if (stops.length) {
      this.ctx.networks.ask<boolean>(id, { kind: 'treeFrom', at: stops[stops.length - 1], reach, cost, barriers }).then(({ value }) => {
        if (gen !== this.gen) return;
        this.tree = value;
        this.follow();
      }, fail);
    }
    if (stops.length < 2) return;
    this.ctx.networks.ask<NetworkRoute>(id, { kind: 'route', stops, barriers, reach, cost, reorder: networkOptions.reorder }).then(({ value, paths }) => {
      if (gen !== this.gen) return;
      if ('error' in value) {
        this.refusal = refusalWords(value, `${REACH_PX} piksel`);
        this.ctx.log.warn(`${LABEL}: ${this.refusal}`);
      } else this.route = { value, paths };
      this.refresh();
    }, fail);
  }

  pointerMove(p: ToolPointer): void {
    this.hover = p.world;
    this.follow();
  }

  /** The way from the last stop to the cursor, asked one at a time (a newer one waiting takes the place of the older). */
  private follow(): void {
    const id = this.networkId();
    if (!id || !this.tree || !this.hover) return;
    const gen = this.gen;
    this.ctx.networks.pathTo(id, [this.hover.x, this.hover.y], this.reach()).then((a) => {
      if (!a || gen !== this.gen) return;
      this.band = a.value ? { cost: a.value.cost, paths: a.paths } : null;
      this.ctx.view.requestOverlay();
    }, () => {});
  }

  protected refresh(): void {
    this.prompt.set(`${LABEL}: ${this.step()}`);
    this.ctx.view.requestOverlay();
  }

  private step(): string {
    if (!this.networkId()) return this.noNetwork();
    const parts = [...this.commonParts(), `Sıra (S): ${reorderName(networkOptions.reorder)}`, ...(this.route ? ['Uygula (Enter)'] : [])];
    const n = this.stops().length;
    let now: string;
    if (this.problem) now = this.problem;
    else if (n === 0) now = 'ilk durağa tıklayın ya da Y,X yazın';
    else if (n === 1) now = 'sonraki durağa tıklayın; imlece kadar yol izlenir';
    else if (this.refusal) now = this.refusal;
    else if (this.route) now = `${n} durak; ${this.totals(this.route.value)}; sonraki durağa tıklayın ya da Enter ile yazın`;
    else now = `${n} durak; yol aranıyor`;
    return `${now} [${parts.join(' / ')}]`;
  }

  /** The route's costs in words: Uzunluk first, then each other the network has. */
  private totals(r: Route): string {
    return this.costNames()
      .map((name, c) => `${name} ${this.costText(r.totals[c] ?? null, c)}`)
      .join(', ');
  }

  protected override option(key: string): 'ask' | 'show' | null {
    if (key === 'S') {
      const at = REORDERS.findIndex((r) => r.value === networkOptions.reorder);
      networkOptions.reorder = REORDERS[(at + 1) % REORDERS.length].value;
      return 'ask';
    }
    return super.option(key);
  }

  override optionChoices(key: string): readonly OptionChoice[] | null {
    if (key === 'S') return REORDERS.map((r) => ({ label: r.name, typed: r.name, checked: r.value === networkOptions.reorder }));
    return super.optionChoices(key);
  }

  override chooseOption(key: string, typed: string): boolean {
    if (key === 'S') {
      const r = REORDERS.find((x) => x.name === typed.trim());
      if (!r) return false;
      networkOptions.reorder = r.value;
      this.changed();
      return true;
    }
    return super.chooseOption(key, typed);
  }

  /** Enter: with a route, it is written and the stops go; with no stops the tool leaves. */
  confirm(): void {
    if (!this.points.length) return this.ctx.tools.exit();
    const r = this.route?.value;
    if (!r) return this.ctx.log.warn(`${LABEL}: ${this.refusal ?? 'yazılacak bir yol yok; en az iki durak verin.'}`);
    const n = this.network()!;
    const decimals = this.ctx.doc.settings.lengthDecimals.value;
    const attrs: Record<string, string> = { Ağ: n.name, Maliyet: this.costNames()[this.costIndex()], Duraklar: String(this.stops().length) };
    if (networkOptions.reorder !== 'none') attrs['Sıra'] = r.order.map((i) => i + 1).join(', ');
    this.costNames().forEach((name, c) => {
      const v = r.totals[c];
      attrs[name] = v === null || v === undefined ? '' : fixed(v, c === 0 ? decimals : 2);
    });
    const ids = writeResults(this.ctx, LABEL, [{ layer: ROUTE_LAYER, objects: [{ geometry: lineGeometry(r.line), attrs }] }]);
    if (!ids) return;
    this.ctx.log.success(`${LABEL}: ${this.totals(r)}; “${ROUTE_LAYER.name}” katmanına yazıldı.`);
    this.points = [];
    this.changed();
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    if (this.route) this.drawPaths(g, view, this.route.paths, pal.accent, 3);
    if (this.band) this.drawPaths(g, view, this.band.paths, pal.accent, 2, [7, 5]);
    this.drawPoints(g, view, true);
    if (this.band && this.hover) {
      const lines = [`+${this.costText(this.band.cost)}`];
      if (this.route) lines.push(`Toplam ${this.costText(this.route.value.cost + this.band.cost)}`);
      drawTag(g, view.worldToScreen(this.hover), lines, pal.accent, pal.labelHalo);
    }
  }
}
