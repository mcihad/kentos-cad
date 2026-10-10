// Zaman sürgüsü (docs/adr/0210 §5): the session's time slider. Its range is the shown temporal layers' objects'
// times, its positions the core's (`model/time.ts`); the window it gives filters the drawing (the geometry store and
// the layer builder, viewport/ViewportController.ts). Nothing of it is saved: the drawing does not change.

import { Signal } from '../core/signal';
import type { CadDocument } from '../model/document';
import { autoStep, showEnds, showTime, showWindow, timePosition, timePositions, windowAt, type TimeStep, type TimeWindow } from '../model/time';

/** The speeds of playback, in steps a second. */
export const TIME_SPEEDS = [0.5, 1, 2, 4] as const;

/** What the slider reads of the drawing: the shown temporal layers' objects with a time and their extent. */
export type TimeExtent = () => { count: number; extent: [number, number] | null };

export class TimeSlider {
  /** Whether the slider is open (its bar under the drawing; the filter on). */
  readonly open = new Signal(false);
  /** Aralık: the window runs to the next position; Anlık: the position's moment. */
  readonly ranged = new Signal(false);
  readonly step = new Signal<TimeStep>({ n: 1, unit: 'year' });
  /** The range the positions cover, null without temporal objects. */
  readonly extent = new Signal<[number, number] | null>(null);
  /** The positions: the anchor and the last one's number (0 … `last`). */
  readonly anchor = new Signal(0);
  readonly last = new Signal(0);
  readonly position = new Signal(0);
  /** The window the slider gives now; null when it is closed (no filter). */
  readonly window = new Signal<TimeWindow | null>(null);
  readonly playing = new Signal(false);
  readonly speed = new Signal<(typeof TIME_SPEEDS)[number]>(1);
  readonly loop = new Signal(false);
  private readonly doc: CadDocument;
  private readonly read: TimeExtent;
  private timer: ReturnType<typeof setTimeout> | undefined;
  /** Waits for the drawing to show a position before the next one (playback). */
  drawn: () => Promise<void> = () => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(() => done())));

  constructor(doc: CadDocument, read: TimeExtent) {
    this.doc = doc;
    this.read = read;
  }

  /** Opens the slider at its last position (the latest state); the reason when there is nothing to show. */
  show(): string | null {
    const { count, extent } = this.read();
    if (!count || !extent) return 'Görünen zamansal katmanlarda zamanı olan nesne yok. Bir katmana Zaman ayarları’ndan başlangıç alanı verin.';
    // One ranged layer shown: moments; only instant ones: periods (a moment would show almost nothing).
    this.ranged.set(!this.doc.layers.leaves().some((l) => l.time?.end != null && this.doc.layers.isVisible(l.id)));
    this.step.set(autoStep(extent));
    this.extent.set(extent);
    if (!this.place(null)) return 'Zaman aralığı bu adımla 100 000 konumdan fazla; daha büyük bir adım seçin.';
    this.position.set(this.last.value);
    this.open.set(true);
    this.apply();
    return null;
  }

  close(): void {
    this.stop();
    this.open.set(false);
    this.window.set(null);
  }

  toggle(): string | null {
    if (this.open.value) {
      this.close();
      return null;
    }
    return this.show();
  }

  /** A new step: refused (the reason) when it makes more than 100 000 positions; the position moves to the nearest moment. */
  setStep(step: TimeStep): string | null {
    const at = this.moment();
    const before = this.step.value;
    this.step.set(step);
    if (!this.place(at)) {
      this.step.set(before);
      return 'Zaman aralığı bu adımla 100 000 konumdan fazla; daha büyük bir adım seçin.';
    }
    this.apply();
    return null;
  }

  setRanged(ranged: boolean): void {
    if (this.ranged.value === ranged) return;
    this.ranged.set(ranged);
    this.apply();
  }

  go(k: number): void {
    const next = Math.max(0, Math.min(this.last.value, Math.round(k)));
    if (next === this.position.value) return;
    this.position.set(next);
    this.apply();
  }

  /** The drawing changed: the range and the positions again, the position at the nearest moment. */
  refresh(): void {
    if (!this.open.value) return;
    const { count, extent } = this.read();
    if (!count || !extent) {
      this.close();
      return;
    }
    const same = this.extent.value && this.extent.value[0] === extent[0] && this.extent.value[1] === extent[1];
    if (same) return;
    const at = this.moment();
    this.extent.set(extent);
    // A range the step would cut into too many positions takes the step it would take by itself.
    if (!this.place(at)) {
      this.step.set(autoStep(extent));
      this.place(at);
    }
    this.apply();
  }

  /** The moment of the position now. */
  moment(): number {
    return timePosition(this.anchor.value, this.step.value, this.position.value);
  }

  /** Position `k`'s moment as the step's unit shows it (the slider's ends). */
  positionText(k: number): string {
    return showTime(timePosition(this.anchor.value, this.step.value, k), this.step.value.unit);
  }

  /** What the position shows: its date, or a period's two (its date once when inside one day). */
  label(): string {
    const w = this.window.value;
    return w ? showWindow(w, this.step.value.unit) : '';
  }

  /** The slider's ends: their clocks when both lie in one day under a day's step, else their dates (docs/adr/0210 §5). */
  ends(): [string, string] {
    return showEnds(this.momentAt(0), this.momentAt(this.last.value), this.step.value.unit);
  }

  private momentAt(k: number): number {
    return timePosition(this.anchor.value, this.step.value, k);
  }

  play(): void {
    if (this.playing.value || !this.open.value) return;
    if (this.position.value >= this.last.value) this.go(0);
    this.playing.set(true);
    void this.tick();
  }

  stop(): void {
    clearTimeout(this.timer);
    this.playing.set(false);
  }

  /** One step of playback; the next waits for the drawing to show this one, and for the speed. */
  private async tick(): Promise<void> {
    const started = performance.now();
    await this.drawn();
    if (!this.playing.value) return;
    const wait = Math.max(0, 1000 / this.speed.value - (performance.now() - started));
    this.timer = setTimeout(() => {
      if (!this.playing.value) return;
      if (this.position.value >= this.last.value) {
        if (!this.loop.value) return this.stop();
        this.go(0);
      } else this.go(this.position.value + 1);
      void this.tick();
    }, wait);
  }

  /** The positions for the range and the step; the position at `at`'s nearest one (none: kept). False past the limit. */
  private place(at: number | null): boolean {
    const extent = this.extent.value;
    if (!extent) return false;
    const p = timePositions(extent, this.step.value);
    if (!p) return false;
    this.anchor.set(p.anchor);
    this.last.set(p.last);
    if (at !== null) this.position.set(this.nearest(at));
    else if (this.position.value > p.last) this.position.set(p.last);
    return true;
  }

  /** The position whose moment is nearest `t` (the earlier of two as near). */
  private nearest(t: number): number {
    let lo = 0;
    let hi = this.last.value;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (timePosition(this.anchor.value, this.step.value, mid) < t) lo = mid + 1;
      else hi = mid;
    }
    if (lo > 0 && t - timePosition(this.anchor.value, this.step.value, lo - 1) <= timePosition(this.anchor.value, this.step.value, lo) - t) return lo - 1;
    return lo;
  }

  private apply(): void {
    this.window.set(this.open.value ? windowAt(this.anchor.value, this.step.value, this.position.value, this.ranged.value) : null);
  }
}
