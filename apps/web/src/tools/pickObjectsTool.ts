import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import type { Entity, EntityKind } from '../model/entities';
import type { Bounds, Vec2 } from '../model/geometry';
import { drawSelectionBox } from './preview';
import type { Tool, ToolPointer } from './Tool';

/** How far the pointer must move with the button down to draw a box, CSS px (as the select tool). */
const DRAG_THRESHOLD = 4;

/**
 * What the command line says while objects are picked:
 * “Alanlar: nesneleri tıklayın ya da pencereyle seçin (3 seçili) [Bitti (Enter) / Vazgeç (Esc)]”.
 */
export const pickObjectsPrompt = ({ label, count }: { label: string; count: number }): string =>
  `${label}: nesneleri tıklayın ya da pencereyle seçin (${count} seçili) [Bitti (Enter) / Vazgeç (Esc)]`;

/** Whether a field of these kinds takes an object of this kind: any kind when it names none. */
export const takesKind = (kinds: readonly EntityKind[] | undefined, kind: EntityKind): boolean => !kinds?.length || kinds.includes(kind);

/**
 * The object a click picks for a field of `kinds`, in this order:
 * 1. the most specific one under the pointer (`hit`), when the field takes its kind;
 * 2. else the nearest edge of one it takes within reach (`nearestEdge`): a
 *    click on a point where only areas are wanted takes the parcel whose edge is there;
 * 3. else the smallest closed shape the click is inside (`enclosing`), when
 *    the field takes its kind: a spot height inside a parcel takes the parcel.
 */
export function pickedAt(
  hit: Entity | null,
  nearestEdge: (takes: (e: Entity) => boolean) => Entity | null,
  enclosing: () => Entity | null,
  kinds: readonly EntityKind[] | undefined,
): Entity | null {
  const takes = (e: Entity) => takesKind(kinds, e.kind);
  if (hit && takes(hit)) return hit;
  const edge = nearestEdge(takes);
  if (edge) return edge;
  const inside = enclosing();
  return inside && takes(inside) ? inside : null;
}

/** The corners' box, and whether it is a crossing: drawn right to left (the select tool's rule). */
export function boxOf(from: Vec2, to: Vec2): { bounds: Bounds; crossing: boolean } {
  return {
    bounds: { minX: Math.min(from.x, to.x), minY: Math.min(from.y, to.y), maxX: Math.max(from.x, to.x), maxY: Math.max(from.y, to.y) },
    crossing: to.x < from.x,
  };
}

/** What a box adds: the objects it holds (`found`, in the document's order) whose kind the field takes. */
export const pickedIn = (found: readonly number[], kindOf: (id: number) => EntityKind | undefined, kinds: readonly EntityKind[] | undefined): number[] =>
  found.filter((id) => {
    const kind = kindOf(id);
    return kind !== undefined && takesKind(kinds, kind);
  });

/**
 * Sahneden seç for a window's objects (the processing window's input,
 * docs/adr/0088; the desktop's `kentos_interaction::pick_objects`). A
 * click turns the object it picks (`pickedAt`) over in the selection; a drag past 4 px
 * adds what its box holds, left to right a window, right to left a
 * crossing. Only the field's kinds are taken, and nothing snaps. Enter,
 * Space or a quick right click keep the selection (`done(true)`); Esc, or
 * the tool ending any other way, leaves it (`done(false)`). The host saves
 * and clears the selection before and puts it back after, as it decides.
 */
export class PickObjectsTool implements Tool {
  readonly id = 'pickObjects';
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  private readonly ctx: AppContext;
  private readonly label: string;
  private readonly kinds: readonly EntityKind[] | undefined;
  private readonly done: (keep: boolean) => void;
  private press: { screen: Vec2; world: Vec2 } | null = null;
  private current: { screen: Vec2; world: Vec2 } | null = null;
  private dragging = false;
  private finished = false;

  constructor(ctx: AppContext, label: string, kinds: readonly EntityKind[] | undefined, done: (keep: boolean) => void) {
    this.ctx = ctx;
    this.label = label;
    this.kinds = kinds?.length ? kinds : undefined;
    this.done = done;
  }

  activate(): void {
    this.say();
  }

  deactivate(): void {
    this.ctx.selection.hover.set(null);
    if (!this.finished) {
      this.finished = true;
      this.done(false);
    }
  }

  pointerMove(p: ToolPointer): void {
    if (this.press) {
      this.current = { screen: p.screen, world: p.raw };
      if (!this.dragging && Math.hypot(p.screen.x - this.press.screen.x, p.screen.y - this.press.screen.y) > DRAG_THRESHOLD) {
        this.dragging = true;
        this.ctx.selection.hover.set(null);
      }
      if (this.dragging) this.ctx.view.requestOverlay();
      return;
    }
    this.ctx.selection.hover.set(this.at(p)?.id ?? null);
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    // A click turns one object over; a drag draws a box (decided on release).
    this.press = { screen: p.screen, world: p.raw };
    this.current = this.press;
    this.dragging = false;
  }

  pointerUp(p: ToolPointer): void {
    const press = this.press;
    if (!press) return;
    const { selection, doc } = this.ctx;
    if (this.dragging && this.current) {
      // The box as it is drawn on the screen decides window or crossing.
      const { bounds } = boxOf(press.world, this.current.world);
      const crossing = this.current.screen.x < press.screen.x;
      selection.add(pickedIn(this.ctx.view.pickRect(bounds, crossing), (id) => doc.get(id)?.kind, this.kinds));
    } else {
      const hit = this.at(p);
      if (hit) selection.toggle(hit.id);
    }
    this.press = this.current = null;
    this.dragging = false;
    this.say();
    this.ctx.view.requestOverlay();
  }

  /** Enter, Space or a quick right click: what is selected goes to the field. */
  confirm(): void {
    if (this.finished) return;
    this.finished = true;
    this.ctx.selection.hover.set(null);
    this.done(true);
    // Leaving from inside the event would race the manager; defer it.
    queueMicrotask(() => this.ctx.tools.exit());
  }

  draw(g: CanvasRenderingContext2D): void {
    if (this.dragging && this.press && this.current) drawSelectionBox(g, this.press.screen, this.current.screen, this.ctx.view.palette.snap);
  }

  private at(p: ToolPointer): Entity | null {
    const view = this.ctx.view;
    return pickedAt(view.pick(p.screen), (takes) => view.pickEdge(p.screen, takes), () => view.enclosingRing(p.raw)?.entity ?? null, this.kinds);
  }

  private say(): void {
    this.prompt.set(pickObjectsPrompt({ label: this.label, count: this.ctx.selection.size }));
  }
}
