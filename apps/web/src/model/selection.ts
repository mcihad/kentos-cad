import { Signal } from '../core/signal';
import type { Vec2 } from './geometry';
import type { CoreArea, CoreEdge } from './ops/topologyRules';

const sameSet = (a: ReadonlySet<number>, b: ReadonlySet<number>) => a.size === b.size && [...a].every((x) => b.has(x));

/**
 * Sıradakini seç's chip (docs/adr/0187 §1): the objects a click could mean, the most specific first, which of them is
 * chosen, and where the click was (the drawing's coordinates). It holds while the selection is the one it made.
 */
export interface Cycle {
  at: Vec2;
  candidates: readonly number[];
  index: number;
}

/**
 * A topology finding shown over the drawing (docs/adr/0202 §5): its regions filled, its edges drawn bold, its place
 * marked with the problem's name; not part of the drawing.
 */
export interface ProblemMark {
  readonly at: Vec2;
  readonly label: string;
  readonly regions: readonly CoreArea[];
  readonly edges: readonly CoreEdge[];
}

export class Selection {
  readonly ids = new Signal<ReadonlySet<number>>(new Set(), sameSet);
  readonly hover = new Signal<number | null>(null);
  /** The vertices Köşe tablosu's selected rows name (docs/adr/0172 §3): ringed in the drawing. */
  readonly vertices = new Signal<readonly Vec2[]>([]);
  /** The place Koordinata git marked (docs/adr/0178 §6): a cross and ring over the drawing, not part of it. */
  readonly mark = new Signal<Vec2 | null>(null);
  /** The Topoloji tab's finding shown over the drawing (docs/adr/0202 §5). */
  readonly problem = new Signal<ProblemMark | null>(null);
  /** The last selection that held something and was replaced or cleared (Önceki seçim, docs/adr/0187 §3). */
  readonly previous = new Signal<readonly number[]>([]);
  /** Sıradakini seç's chip: null while the selection is not a click's among several objects. */
  readonly cycle = new Signal<Cycle | null>(null);

  get size(): number {
    return this.ids.value.size;
  }

  has(id: number): boolean {
    return this.ids.value.has(id);
  }

  /** Selects exactly these; the selection it replaces becomes the previous one. */
  set(ids: Iterable<number>): void {
    this.replace(new Set(ids));
  }

  add(ids: Iterable<number>): void {
    this.cycle.set(null);
    this.ids.set(new Set([...this.ids.value, ...ids]));
  }

  toggle(id: number): void {
    this.cycle.set(null);
    const next = new Set(this.ids.value);
    next.has(id) ? next.delete(id) : next.add(id);
    this.ids.set(next);
  }

  /** Drop ids that no longer exist. */
  retain(exists: (id: number) => boolean): void {
    const next = [...this.ids.value].filter(exists);
    if (next.length !== this.ids.value.size) {
      this.cycle.set(null);
      this.ids.set(new Set(next));
    }
  }

  /** Selects nothing; the selection it clears becomes the previous one. */
  clear(): void {
    this.replace(new Set());
  }

  /**
   * Sıradakini seç: the chip's `index`th candidate takes the place of the one chosen, the rest of the selection kept
   * (docs/adr/0187 §1). The previous selection stays what it was.
   */
  cycleTo(index: number): void {
    const c = this.cycle.value;
    if (!c || !c.candidates.length) return;
    const i = ((index % c.candidates.length) + c.candidates.length) % c.candidates.length;
    const next = new Set(this.ids.value);
    next.delete(c.candidates[c.index]);
    next.add(c.candidates[i]);
    this.ids.set(next);
    this.cycle.set({ ...c, index: i });
  }

  private replace(next: Set<number>): void {
    this.cycle.set(null);
    if (sameSet(next, this.ids.value)) return;
    if (this.ids.value.size) this.previous.set([...this.ids.value]);
    this.ids.set(next);
  }
}
