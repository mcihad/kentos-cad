import { Signal } from '../core/signal';

const sameSet = (a: ReadonlySet<number>, b: ReadonlySet<number>) => a.size === b.size && [...a].every((x) => b.has(x));

export class Selection {
  readonly ids = new Signal<ReadonlySet<number>>(new Set(), sameSet);
  readonly hover = new Signal<number | null>(null);

  get size(): number {
    return this.ids.value.size;
  }

  has(id: number): boolean {
    return this.ids.value.has(id);
  }

  set(ids: Iterable<number>): void {
    this.ids.set(new Set(ids));
  }

  add(ids: Iterable<number>): void {
    this.ids.set(new Set([...this.ids.value, ...ids]));
  }

  toggle(id: number): void {
    const next = new Set(this.ids.value);
    next.has(id) ? next.delete(id) : next.add(id);
    this.ids.set(next);
  }

  /** Drop ids that no longer exist. */
  retain(exists: (id: number) => boolean): void {
    const next = [...this.ids.value].filter(exists);
    if (next.length !== this.ids.value.size) this.ids.set(new Set(next));
  }

  clear(): void {
    this.ids.set(new Set());
  }
}
