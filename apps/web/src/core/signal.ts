import type { Disposable } from './disposable';

export type Listener<T> = (value: T, prev: T) => void;

export interface ReadonlySignal<T> {
  readonly value: T;
  subscribe(fn: Listener<T>, immediate?: boolean): Disposable;
}

/**
 * Minimal observable value. Deliberately explicit (no auto-tracking) so data
 * flow stays easy to follow in a large codebase.
 */
export class Signal<T> implements ReadonlySignal<T> {
  private listeners = new Set<Listener<T>>();
  private current: T;
  private readonly equals: (a: T, b: T) => boolean;

  constructor(initial: T, equals: (a: T, b: T) => boolean = Object.is) {
    this.current = initial;
    this.equals = equals;
  }

  get value(): T {
    return this.current;
  }

  set(next: T): void {
    if (this.equals(next, this.current)) return;
    const prev = this.current;
    this.current = next;
    this.emit(prev);
  }

  update(fn: (v: T) => T): void {
    this.set(fn(this.current));
  }

  /** Notify listeners although the reference did not change (mutable containers). */
  touch(): void {
    this.emit(this.current);
  }

  subscribe(fn: Listener<T>, immediate = false): Disposable {
    this.listeners.add(fn);
    if (immediate) fn(this.current, this.current);
    return () => this.listeners.delete(fn);
  }

  private emit(prev: T): void {
    for (const l of [...this.listeners]) l(this.current, prev);
  }
}

/** Subscribe to several signals with one callback. */
export function watchAll(signals: readonly ReadonlySignal<unknown>[], fn: () => void): Disposable {
  const subs = signals.map((s) => s.subscribe(fn));
  return () => subs.forEach((d) => d());
}
