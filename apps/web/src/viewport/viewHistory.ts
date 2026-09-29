import { Signal } from '../core/signal';
import type { Vec2 } from '../model/geometry';
import type { Camera } from './Camera';

/**
 * Where the view has been: Önceki görünüm and Sonraki görünüm (docs/adr/0141). The history is the
 * session's, one per window: not in the drawing, not in undo, not in the file, and empty again when
 * another drawing is opened. `ViewHistory` holds the two stacks and knows no camera; `ViewNavigation`
 * ties it to one and holds the recording rules, so a command or a gesture only says what it is
 * about to do.
 */

/** A view as the history keeps it: where the camera looks and how far in. The window's size is not part of it. */
export interface ViewState {
  readonly center: Vec2;
  /** Pixels per metre. */
  readonly scale: number;
}

/** Views kept; the oldest goes first. */
export const HISTORY_LIMIT = 30;
/** A wheel step this long (ms) after the last one begins a new pass; steps closer together are one pass. */
export const WHEEL_PAUSE_MS = 500;

export const sameView = (a: ViewState, b: ViewState): boolean => a.scale === b.scale && a.center.x === b.center.x && a.center.y === b.center.y;

export class ViewHistory {
  /** The views the user left, the newest last. */
  private past: ViewState[] = [];
  /** The views Önceki görünüm stepped away from, the next to come back last. */
  private future: ViewState[] = [];
  /** When the wheel last stepped (ms); minus infinity when a pass may begin at once. */
  private lastWheel = Number.NEGATIVE_INFINITY;
  /** Whether there is a view to go back to. */
  readonly canBack = new Signal(false);
  /** Whether there is a view to go forward to. */
  readonly canForward = new Signal(false);

  /**
   * The view the user is leaving (a command, the start of a pan). Kept unless it is the one kept
   * last; a new view leaves nothing to go forward to.
   */
  record(view: ViewState): void {
    this.lastWheel = Number.NEGATIVE_INFINITY;
    this.push(view);
  }

  /**
   * A wheel step at `now` (ms) from `view`. The first step after a pause opens a pass and keeps the
   * view it leaves; the steps that follow before another pause belong to the pass and keep nothing.
   */
  wheel(view: ViewState, now: number): void {
    const opens = now - this.lastWheel >= WHEEL_PAUSE_MS;
    this.lastWheel = now;
    if (opens) this.push(view);
  }

  /**
   * Önceki görünüm from `current`: the view kept last, which `current` goes ahead of; null when
   * there is none. A kept view equal to `current` is no place to go back to and is passed over.
   */
  back(current: ViewState): ViewState | null {
    return this.step(this.past, this.future, current);
  }

  /** Sonraki görünüm from `current`: the reverse of `back`. */
  forward(current: ViewState): ViewState | null {
    return this.step(this.future, this.past, current);
  }

  /** Another drawing was opened: nowhere to go back to or forward to. */
  clear(): void {
    this.past = [];
    this.future = [];
    this.lastWheel = Number.NEGATIVE_INFINITY;
    this.sync();
  }

  private push(view: ViewState): void {
    if (this.past.length && sameView(this.past[this.past.length - 1], view)) return;
    this.future = [];
    this.past.push(view);
    if (this.past.length > HISTORY_LIMIT) this.past.shift();
    this.sync();
  }

  /** Takes the newest view of `from` that is not `current`, and puts `current` on `to`. */
  private step(from: ViewState[], to: ViewState[], current: ViewState): ViewState | null {
    let target = from.pop();
    while (target && sameView(target, current)) target = from.pop();
    if (target) {
      to.push(current);
      if (to.length > HISTORY_LIMIT) to.shift();
    }
    this.lastWheel = Number.NEGATIVE_INFINITY;
    this.sync();
    return target ?? null;
  }

  private sync(): void {
    this.canBack.set(this.past.length > 0);
    this.canForward.set(this.future.length > 0);
  }
}

/**
 * A camera with its history and the rules for keeping it (docs/adr/0141): a view is kept when the
 * user leaves it by a navigation command (Tümünü göster, Seçime yakınlaştır, Pencere yakınlaştır,
 * Katmana yakınlaştır, Yakınlaştır, Uzaklaştır), at the start of a pan, and at the first wheel step
 * after a pause. The window's size changing keeps nothing (it is not part of a view), and neither
 * does putting a drawing on screen.
 */
export class ViewNavigation {
  readonly history = new ViewHistory();
  private readonly camera: Camera;

  constructor(camera: Camera) {
    this.camera = camera;
  }

  /** The view now. */
  get view(): ViewState {
    return { center: { x: this.camera.center.x, y: this.camera.center.y }, scale: this.camera.scale };
  }

  /** A gesture (a pan) is about to move the view: keeps it as it is. */
  remember(): void {
    this.history.record(this.view);
  }

  /** A wheel step is about to zoom; `now` is the clock in ms. */
  wheel(now: number = performance.now()): void {
    this.history.wheel(this.view, now);
  }

  /** A navigation command: runs `move`, and keeps the view it left when it changed the view (Tümünü göster on a view already there keeps nothing). */
  navigate(move: () => void): void {
    const before = this.view;
    move();
    if (!sameView(before, this.view)) this.history.record(before);
  }

  /** Önceki görünüm; false when there is none. */
  back(): boolean {
    return this.goTo(this.history.back(this.view));
  }

  /** Sonraki görünüm; false when there is none. */
  forward(): boolean {
    return this.goTo(this.history.forward(this.view));
  }

  private goTo(view: ViewState | null): boolean {
    if (!view) return false;
    this.camera.setView(view.center, view.scale);
    return true;
  }
}
