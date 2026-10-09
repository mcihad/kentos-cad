import { refusalWords, type NetworkTrace } from '../model/networkAnswers';
import type { NetworkTraceKind } from '../io/network/protocol';
import type { ViewTransform } from '../viewport/Camera';
import { NetworkTool, networkOptions, REACH_PX } from './networkTool';
import type { OptionChoice } from './Tool';

/**
 * Şebeke izleme (docs/adr/0209 §8): starting points clicked (a broken pipe, a feeder); Tür (T) traces what is connected
 * (Bağlı), what the edges' directions lead to or from (Akış aşağı, Akış yukarı), or what closing the nearest open valves
 * isolates (Yalıtım: the valves to close, and what no source feeds once they are). The trace shows at once; Enter
 * selects its objects (and the valves) and says their length, counts and the valves' names. The desktop's
 * `kentos_interaction::network::trace`.
 */

export const LABEL = 'Şebeke izleme';

export const TRACE_KINDS: readonly { value: NetworkTraceKind; name: string }[] = [
  { value: 'connected', name: 'bağlı' },
  { value: 'downstream', name: 'akış aşağı' },
  { value: 'upstream', name: 'akış yukarı' },
  { value: 'isolation', name: 'yalıtım' },
];
const kindName = (v: NetworkTraceKind) => TRACE_KINDS.find((k) => k.value === v)?.name ?? 'bağlı';

type Traced = Extract<NetworkTrace, { objects: readonly number[] }>;

export class NetworkTraceTool extends NetworkTool {
  readonly id = 'netTrace';
  protected readonly prefers = 'utility' as const;
  protected readonly label = LABEL;
  protected override readonly costs = false;
  private traced: { value: Traced; paths: Float64Array } | null = null;
  private refusal: string | null = null;

  protected ask(gen: number): void {
    const id = this.networkId()!;
    const starts = this.stops();
    this.traced = null;
    this.refusal = null;
    if (!starts.length) return;
    this.ctx.networks.ask<NetworkTrace>(id, { kind: 'trace', starts, barriers: this.barriers(), reach: this.reach(), trace: networkOptions.trace }).then(
      ({ value, paths }) => {
        if (gen !== this.gen) return;
        if ('error' in value) {
          this.refusal = refusalWords(value, `${REACH_PX} piksel`);
          this.ctx.log.warn(`${LABEL}: ${this.refusal}`);
        } else this.traced = { value, paths };
        this.refresh();
      },
      (e: Error) => gen === this.gen && this.fail(e),
    );
  }

  protected refresh(): void {
    this.prompt.set(`${LABEL}: ${this.step()}`);
    this.ctx.view.requestOverlay();
  }

  private step(): string {
    if (!this.networkId()) return this.noNetwork();
    const parts = [...this.commonParts(), `Tür (T): ${kindName(networkOptions.trace)}`, ...(this.traced ? ['Seç (Enter)'] : [])];
    const n = this.stops().length;
    let now: string;
    if (this.problem) now = this.problem;
    else if (n === 0) now = 'izlemenin başlayacağı yere tıklayın ya da Y,X yazın';
    else if (this.refusal) now = this.refusal;
    else if (this.traced) now = `${this.summary(this.traced.value)}; başka başlangıca tıklayın ya da Enter ile seçin`;
    else now = `${n} başlangıç; izleniyor`;
    return `${now} [${parts.join(' / ')}]`;
  }

  /** The trace in words: length, objects, valves, and what is no longer fed. */
  private summary(t: Traced): string {
    const f = this.ctx.format;
    const words = [`${f.length(t.length)}, ${t.objects.length} nesne`];
    if (networkOptions.trace === 'isolation') {
      words.push(`${t.valves.length} vana kapatılır`);
      if (t.unfed.length) words.push(`beslemesiz kalan ${f.length(t.unfedLength)}, ${t.unfed.length} nesne`);
    }
    return words.join('; ');
  }

  protected override option(key: string): 'ask' | 'show' | null {
    if (key === 'T') {
      const at = TRACE_KINDS.findIndex((k) => k.value === networkOptions.trace);
      networkOptions.trace = TRACE_KINDS[(at + 1) % TRACE_KINDS.length].value;
      return 'ask';
    }
    return super.option(key);
  }

  override optionChoices(key: string): readonly OptionChoice[] | null {
    if (key === 'T') return TRACE_KINDS.map((k) => ({ label: k.name, typed: k.name, checked: k.value === networkOptions.trace }));
    return super.optionChoices(key);
  }

  override chooseOption(key: string, typed: string): boolean {
    if (key === 'T') {
      const k = TRACE_KINDS.find((x) => x.name === typed.trim());
      if (!k) return false;
      networkOptions.trace = k.value;
      this.changed();
      return true;
    }
    return super.chooseOption(key, typed);
  }

  /** A valve's name: its label, else its `Ad`, else its number. */
  private valveName(id: number): string {
    const e = this.ctx.doc.get(id);
    return e?.label?.trim() || e?.attrs['Ad']?.trim() || `#${id}`;
  }

  /** Enter: the trace's objects (and Yalıtım's valves) are selected and said; the tool leaves. With no start it leaves. */
  confirm(): void {
    const t = this.traced?.value;
    if (!t) {
      if (!this.points.length) return this.ctx.tools.exit();
      return this.ctx.log.warn(`${LABEL}: ${this.refusal ?? 'izlenen bir şey yok.'}`);
    }
    const isolation = networkOptions.trace === 'isolation';
    const ids = [...t.objects, ...(isolation ? t.valves.map((v) => v.id) : [])].filter((id) => this.ctx.doc.get(id));
    this.ctx.selection.set(ids);
    let said = `${LABEL} (${kindName(networkOptions.trace)}): ${this.summary(t)}`;
    if (isolation && t.valves.length) said += `; kapatılacak vanalar: ${t.valves.map((v) => this.valveName(v.id)).join(', ')}`;
    this.ctx.log.success(`${said}.`);
    // The starts go first: leaving takes a point back while the tool has one (ToolManager.exit asks `cancel` first).
    this.points = [];
    this.barrierNext = false;
    this.ctx.tools.exit();
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const t = this.traced;
    if (t) {
      // The trace's own lines first, then what is no longer fed (dashed, in the warning colour).
      const split = pathsSplit(t.paths, t.value.lines.length);
      this.drawPaths(g, view, split[0], pal.accent, 3);
      this.drawPaths(g, view, split[1], pal.danger, 2, [6, 4]);
      g.save();
      g.fillStyle = pal.danger;
      g.strokeStyle = pal.labelHalo;
      for (const v of t.value.valves) {
        const s = view.worldToScreen(v);
        g.fillRect(Math.round(s.x) - 5, Math.round(s.y) - 5, 10, 10);
        g.strokeRect(Math.round(s.x) - 5.5, Math.round(s.y) - 5.5, 11, 11);
      }
      g.restore();
    }
    this.drawPoints(g, view, false);
  }
}

/** Packed paths (`flags, n, x0, y0, …`) cut after the first `count`. */
function pathsSplit(paths: Float64Array, count: number): [Float64Array, Float64Array] {
  let i = 0;
  for (let k = 0; k < count && i < paths.length; k++) i += 2 + 2 * paths[i + 1];
  return [paths.subarray(0, i), paths.subarray(i)];
}
