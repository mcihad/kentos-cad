import { Signal } from '../core/signal';
import type { Entity, NewEntity } from '../model/entities';
import type { Bounds, Vec2 } from '../model/geometry';

/**
 * In-app clipboard (session scope). Holds deep copies of entities and the
 * base point they are pasted by: the lower-left corner of their bounds, so
 * "yapıştır" places that corner at the clicked point. The copies keep no
 * slot or persistent id: a paste, even after a cut, makes new objects with
 * ids of their own (docs/adr/0014).
 */
export class Clipboard {
  readonly count = new Signal(0);
  private items: NewEntity[] = [];
  private basePoint: Vec2 = { x: 0, y: 0 };

  /** `extent`: the box around the entities (the geometry store has them already; see `ViewportController.extent`). */
  set(entities: readonly Entity[], extent: Bounds | null): void {
    this.items = entities.map(({ id: _id, uid: _uid, ...rest }) => structuredClone(rest) as NewEntity);
    this.basePoint = entities.length && extent ? { x: extent.minX, y: extent.minY } : { x: 0, y: 0 };
    this.count.set(this.items.length);
  }

  /** Fresh copies each time, so one clipboard can be pasted many times. */
  get(): { items: NewEntity[]; base: Vec2 } {
    return { items: this.items.map((e) => structuredClone(e)), base: { ...this.basePoint } };
  }
}
