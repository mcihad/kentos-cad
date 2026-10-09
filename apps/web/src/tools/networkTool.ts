import type { AppContext } from '../app/context';
import type { NetworkDef } from '../contracts/generated/NetworkDef';
import type { NetworkKind } from '../contracts/generated/NetworkKind';
import { fixed } from '../core/displayNumber';
import { Signal } from '../core/signal';
import type { Vec2 } from '../model/geometry';
import type { NetworkPlace } from '../model/networkAnswers';
import { LENGTH_COST } from '../model/networkRules';
import type { NetworkReorder, NetworkTraceKind, XY } from '../io/network/protocol';
import type { ViewTransform } from '../viewport/Camera';
import { pointFromText } from './tracking';
import { strokePaths, tint } from './preview';
import type { OptionChoice, Tool, ToolPointer } from './Tool';

/**
 * What the network tools share (docs/adr/0209 §10): Ağ (A) and, where it counts, Maliyet (M); Engel (E: the next click
 * puts a barrier); the points clicked or typed sit on the network (found within 20 screen pixels, kept where they sit);
 * Esc takes the newest point back, with none it leaves. A project without a network is told to define one (Ağlar, G).
 * The network is built in the network worker (app/networks.ts) as the tool starts; answers that come after the tool
 * moved on are dropped. The desktop's `kentos_interaction::network`.
 */

/** How far a click looks for the network, screen pixels (docs/adr/0209 §4). */
export const REACH_PX = 20;

/** The session's options (the desktop's `Memory::network_*`). */
export const networkOptions = {
  /** The network last chosen; none: the first of the tool's kind. */
  network: null as string | null,
  /** The cost last chosen, by name. */
  cost: LENGTH_COST as string,
  reorder: 'none' as NetworkReorder,
  /** Hizmet alanı's breaks for each kind of cost (length, speed, field), in its unit; none: the kind's own. */
  breaks: [null, null, null] as (number[] | null)[],
  toward: false,
  rings: false,
  separate: false,
  /** Kenar payı, metres. */
  trim: 50,
  lines: false,
  trace: 'connected' as NetworkTraceKind,
};

export type NetworkPoint = { readonly kind: 'stop' | 'barrier'; readonly at: Vec2 };

export abstract class NetworkTool implements Tool {
  abstract readonly id: string;
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = true;
  protected readonly ctx: AppContext;
  /** The points in the order given; Esc takes the last back. */
  protected points: NetworkPoint[] = [];
  protected barrierNext = false;
  /** Bumped whenever what an answer was asked for changes: an older answer is dropped. */
  protected gen = 0;
  /** Bumped when the tool leaves or another network is chosen: a point found for the old one is dropped. */
  private session = 0;
  /** The network's words when it cannot be built or asked. */
  protected problem: string | null = null;
  /** The kind of network the tool takes first. */
  protected abstract readonly prefers: NetworkKind;
  protected abstract readonly label: string;
  /** Whether the tool asks with a cost (Şebeke izleme does not). */
  protected readonly costs: boolean = true;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  activate(): void {
    // The network is built as the tool starts; what building says, the service says once (app/networks.ts).
    const id = this.networkId();
    if (id) this.ctx.networks.ready(id).catch((e: Error) => this.fail(e));
    this.changed();
  }

  deactivate(): void {
    this.gen++;
    this.session++;
  }

  protected fail(e: Error): void {
    this.problem = e.message;
    this.ctx.log.warn(e.message);
    this.refresh();
  }

  /** The networks of the project. */
  protected list(): readonly NetworkDef[] {
    return this.ctx.networks.list();
  }

  /** The network used: the one last chosen, else the first of the tool's kind, else the first. */
  protected networkId(): string | null {
    const list = this.list();
    const chosen = networkOptions.network;
    if (chosen && list.some((n) => n.id === chosen)) return chosen;
    return (list.find((n) => n.kind === this.prefers) ?? list[0])?.id ?? null;
  }

  protected network(): NetworkDef | undefined {
    const id = this.networkId();
    return id ? this.ctx.networks.find(id) : undefined;
  }

  /** The network's costs' names, Uzunluk first. */
  protected costNames(): string[] {
    return [LENGTH_COST, ...(this.network()?.costs ?? []).map((c) => c.name)];
  }

  /** The cost used: the one last chosen when this network has it, else Uzunluk. */
  protected costIndex(): number {
    return Math.max(0, this.costNames().indexOf(networkOptions.cost));
  }

  /** A value of cost `c` in its unit (metres, minutes, the field's unit). */
  protected costText(v: number | null, c = this.costIndex()): string {
    if (v === null || !Number.isFinite(v)) return '—';
    if (c === 0) return this.ctx.format.length(v);
    const cost = this.network()?.costs?.[c - 1];
    if (cost?.kind === 'speed') return `${fixed(v, 1)} dk`;
    return cost?.unit ? `${fixed(v, 2)} ${cost.unit}` : fixed(v, 2);
  }

  /** How far a point looks for the network, metres. */
  protected reach(): number {
    return REACH_PX / this.ctx.view.camera.scale;
  }

  protected stops(): XY[] {
    return this.points.filter((p) => p.kind === 'stop').map((p) => [p.at.x, p.at.y] as const);
  }

  protected barriers(): XY[] {
    return this.points.filter((p) => p.kind === 'barrier').map((p) => [p.at.x, p.at.y] as const);
  }

  /** The options shared by the network tools, as the prompt shows them. */
  protected commonParts(): string[] {
    const n = this.network();
    return [`Ağ (A): ${n?.name ?? 'yok'}`, ...(this.costs ? [`Maliyet (M): ${this.costNames()[this.costIndex()]}`] : []), this.barrierNext ? 'Engel (E): sonraki tıklama' : 'Engel (E)'];
  }

  /** The prompt's step and option when the project has no network (the desktop's `no_network_words`). */
  protected noNetwork(): string {
    return 'projede ağ tanımlı değil; Ağlar ile yol ya da şebeke ağını tanımlayın [Ağlar (G)]';
  }

  /** Something the answers depend on changed: ask again and show. */
  protected changed(): void {
    this.gen++;
    this.problem = null;
    if (this.networkId()) this.ask(this.gen);
    this.refresh();
  }

  /** Asks the network for the answers the points need, dropping them when `gen` is not current when they come. */
  protected abstract ask(gen: number): void;

  /** The prompt and the overlay. */
  protected abstract refresh(): void;

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    this.place(p.world);
  }

  /** A clicked or typed point: put on the network as a stop (a facility, a start) or, after E, a barrier. */
  protected place(at: Vec2): void {
    const id = this.networkId();
    if (!id) return this.ctx.log.warn(`${this.label}: projede ağ tanımlı değil; Ağlar ile yol ya da şebeke ağını tanımlayın`);
    const kind = this.barrierNext ? 'barrier' : 'stop';
    this.barrierNext = false;
    const session = this.session;
    this.ctx.networks
      .ask<NetworkPlace | null>(id, { kind: 'locate', at: [at.x, at.y], reach: this.reach() })
      .then(({ value }) => {
        if (session !== this.session) return;
        if (!value) return this.ctx.log.warn(`${this.label}: tıklanan yerin ${REACH_PX} piksel yakınında ağ yok; ağın çizgisine yakın tıklayın.`);
        this.points.push({ kind, at: { x: value.x, y: value.y } });
        this.changed();
      }, (e: Error) => this.fail(e));
    this.refresh();
  }

  input(text: string): boolean {
    const t = text.trim();
    const pt = pointFromText(this.ctx, t, this.points.at(-1)?.at ?? null, null);
    if (pt) {
      this.place(pt);
      return true;
    }
    if (this.answer(t)) return true;
    const did = this.option(t.toLocaleUpperCase('tr-TR'));
    if (!did) return false;
    if (did === 'ask') this.changed();
    else this.refresh();
    return true;
  }

  /** A value typed for what the tool asks; false when it asks nothing. */
  protected answer(_text: string): boolean {
    return false;
  }

  /**
   * A key's option: `ask` when the answers depend on it (asked again), `show` when only the prompt does, null when it is
   * not one.
   */
  protected option(key: string): 'ask' | 'show' | null {
    switch (key) {
      case 'A': {
        const list = this.list();
        if (!list.length) return null;
        const at = list.findIndex((n) => n.id === this.networkId());
        networkOptions.network = list[(at + 1) % list.length].id;
        this.started();
        return 'ask';
      }
      case 'M': {
        if (!this.costs) return null;
        const names = this.costNames();
        networkOptions.cost = names[(this.costIndex() + 1) % names.length];
        return 'ask';
      }
      case 'E':
        this.barrierNext = !this.barrierNext;
        return 'show';
      case 'G':
        this.ctx.commands.execute('network.manage');
        return 'show';
      default:
        return null;
    }
  }

  /** Another network was chosen: its points go, and it is built as it is chosen. */
  private started(): void {
    this.points = [];
    this.session++;
    const id = this.networkId();
    if (id) this.ctx.networks.ready(id).catch((e: Error) => this.fail(e));
  }

  optionChoices(key: string): readonly OptionChoice[] | null {
    if (key === 'A') {
      const id = this.networkId();
      return [...this.list().map((n) => ({ label: n.name, typed: n.name, checked: n.id === id })), { label: 'Ağlar…', typed: '', checked: false, command: 'network.manage' }];
    }
    if (key === 'M' && this.costs) {
      const at = this.costIndex();
      return this.costNames().map((name, i) => ({ label: name, typed: name, checked: i === at }));
    }
    return null;
  }

  chooseOption(key: string, typed: string): boolean {
    const t = typed.trim();
    if (key === 'A') {
      const n = this.list().find((x) => x.name === t);
      if (!n) return false;
      if (n.id !== this.networkId()) {
        networkOptions.network = n.id;
        this.started();
      }
    } else if (key === 'M' && this.costs) {
      if (!this.costNames().includes(t)) return false;
      networkOptions.cost = t;
    } else return false;
    this.changed();
    return true;
  }

  /** Esc: E waiting, then the newest point, go first; with none the tool leaves. */
  cancel(): boolean {
    if (this.barrierNext) {
      this.barrierNext = false;
      this.refresh();
      return true;
    }
    if (!this.points.length) return false;
    this.points.pop();
    this.changed();
    return true;
  }

  undoStep(): boolean {
    return this.cancel();
  }

  get pointCount(): number {
    return this.points.length;
  }

  /** The points: stops numbered in rings, barriers as crosses. */
  protected drawPoints(g: CanvasRenderingContext2D, view: ViewTransform, numbered: boolean): void {
    const pal = this.ctx.view.palette;
    let n = 0;
    g.save();
    g.font = `600 11px ${pal.font}`;
    g.textAlign = 'center';
    g.textBaseline = 'middle';
    for (const p of this.points) {
      const s = view.worldToScreen(p.at);
      if (p.kind === 'barrier') {
        g.strokeStyle = pal.danger;
        g.lineWidth = 2.5;
        g.beginPath();
        g.moveTo(s.x - 6, s.y - 6);
        g.lineTo(s.x + 6, s.y + 6);
        g.moveTo(s.x + 6, s.y - 6);
        g.lineTo(s.x - 6, s.y + 6);
        g.stroke();
        continue;
      }
      n++;
      g.fillStyle = pal.accent;
      g.beginPath();
      g.arc(s.x, s.y, numbered ? 8 : 5, 0, Math.PI * 2);
      g.fill();
      g.strokeStyle = pal.labelHalo;
      g.lineWidth = 1.5;
      g.stroke();
      if (numbered) {
        g.fillStyle = pal.labelHalo;
        g.fillText(String(n), s.x, s.y + 0.5);
      }
    }
    g.restore();
  }

  /** Paths as the worker sent them (`flags, n, x0, y0, …`). */
  protected drawPaths(g: CanvasRenderingContext2D, view: ViewTransform, paths: Float64Array, color: string, width: number, dash?: number[]): void {
    if (paths.length) strokePaths(g, view, paths, { color, width, dash });
  }

  /** A service area's fills (`band, rings, n0, x0, y0, …`), each band paler than the one inside it. */
  protected drawFills(g: CanvasRenderingContext2D, view: ViewTransform, fills: Float64Array, bands: number): void {
    const pal = this.ctx.view.palette;
    g.save();
    for (let i = 0; i < fills.length; ) {
      const band = fills[i];
      const rings = fills[i + 1];
      i += 2;
      g.beginPath();
      for (let r = 0; r < rings; r++) {
        const n = fills[i++];
        for (let k = 0; k < n; k++) {
          const s = view.worldToScreen({ x: fills[i + 2 * k], y: fills[i + 2 * k + 1] });
          k ? g.lineTo(s.x, s.y) : g.moveTo(s.x, s.y);
        }
        g.closePath();
        i += 2 * n;
      }
      g.fillStyle = tint(pal.accent, 0.42 - (0.3 * band) / Math.max(1, bands - 1 || 1));
      g.fill('evenodd');
      g.strokeStyle = tint(pal.accent, 0.8);
      g.lineWidth = 1;
      g.stroke();
    }
    g.restore();
  }
}
