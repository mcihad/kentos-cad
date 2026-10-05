import type { DrawingEntity, Entity, TextEntity } from './entities';
import { DEFAULT_LABELS } from './labelDefaults';
import type { LabelStyle } from './layers';
import { labelTextOf } from './ops/labelText';
import { sameJson } from './sameJson';

/**
 * Linked texts (docs/adr/0175 §4): a text that writes an object's label (`labelOf`, `labelScale`) keeps following
 * it. Before a step is recorded (`CadDocument`'s commit), its changes are read:
 *
 * - an object the step changed has its linked texts written again by the label rule (`labelTextOf`): from its label
 *   now, its layer's label style (or its kind's default), the project's typeface and the text's scale; a text the
 *   rule no longer writes (no label, out of the style's scale range, too small) stays where it is and loses its link;
 * - an object the step removed takes its linked texts with it;
 * - a linked text the step itself edited (its place, text, height, turn or alignment) loses its link, unless its
 *   object changed in the same step (then the rule writes it).
 *
 * The changes join the step: one undo takes them back with the rest, and undo and redo replay what was recorded.
 * The desktop's is kentos_domain's linked.rs; both run fixtures/document-ops/v1/linked-texts.json.
 */

/** An object's change as a step records it. */
export type ObjectOp =
  | { type: 'add'; entity: DrawingEntity }
  | { type: 'remove'; entity: DrawingEntity }
  | { type: 'update'; before: DrawingEntity; after: DrawingEntity };

/** What the pass reads of the drawing, as the step left it. */
export interface LinkedDrawing {
  /** The object in this slot. */
  get(id: number): DrawingEntity | undefined;
  /** The object with this persistent id, while it is in the drawing. */
  byUid(uid: string): DrawingEntity | undefined;
  /** The slots of the texts that write the label of the object with this persistent id. */
  linkedTo(uid: string): Iterable<number>;
  /** The layer's own label style, when it has one. */
  layerLabel(layerId: string): LabelStyle | null | undefined;
  /** The project's drawing typeface (`DrawingFont`). */
  font: string;
}

type LinkedText = TextEntity & { uid: string };

/** Whether `after` is the same linked text as `before` written elsewhere or otherwise: its place, text, height, turn or alignment changed. */
function edited(before: TextEntity, after: TextEntity): boolean {
  return (
    before.p.x !== after.p.x ||
    before.p.y !== after.p.y ||
    before.text !== after.text ||
    before.height !== after.height ||
    before.rotation !== after.rotation ||
    before.align !== after.align
  );
}

/** A text without its link: it stays as it is and follows nothing. */
function unlinked(text: LinkedText): LinkedText {
  const out = { ...text };
  delete out.labelOf;
  delete out.labelScale;
  return out;
}

/** The linked text `text` written again for its object `object` now, or none when the rule writes nothing. */
function rewritten(text: LinkedText, object: Entity, d: LinkedDrawing): LinkedText | undefined {
  const label = object.label;
  if (!label) return undefined;
  const style = d.layerLabel(object.layerId) ?? DEFAULT_LABELS[object.kind];
  if (!style || text.labelScale === undefined) return undefined;
  const t = labelTextOf(object, label, style, text.labelScale, d.font);
  if (!t) return undefined;
  return { ...text, p: { x: t.p.x, y: t.p.y }, text: t.text, height: t.height, rotation: t.rotation, align: t.align };
}

/** What keeps the linked texts with their objects after `ops` (a step about to be recorded); the caller applies it. */
export function followLinks(ops: readonly ObjectOp[], d: LinkedDrawing): ObjectOp[] {
  // The objects the step changed or removed, by persistent id, and the linked texts it edited itself, by slot.
  const objects: string[] = [];
  const seen = new Set<string>();
  const texts = new Set<number>();
  for (const op of ops) {
    const e = op.type === 'update' ? op.after : op.entity;
    if (!seen.has(e.uid)) {
      seen.add(e.uid);
      objects.push(e.uid);
    }
    if (op.type === 'update' && op.before.kind === 'text' && op.after.kind === 'text' && op.after.labelOf !== undefined && edited(op.before, op.after)) {
      texts.add(op.after.id);
    }
  }
  const out: ObjectOp[] = [];
  const done = new Set<number>();
  for (const uid of objects) {
    for (const slot of [...d.linkedTo(uid)].sort((a, b) => a - b)) {
      if (done.has(slot)) continue;
      done.add(slot);
      const current = d.get(slot);
      if (current?.kind !== 'text') continue;
      const object = d.byUid(uid);
      // Its object is gone: so is the text.
      if (!object) {
        out.push({ type: 'remove', entity: current });
        continue;
      }
      const next = rewritten(current, object, d) ?? unlinked(current);
      if (!sameJson(next, current)) out.push({ type: 'update', before: current, after: next });
    }
  }
  // A linked text edited by itself, its object unchanged: it follows nothing now.
  for (const slot of [...texts].sort((a, b) => a - b)) {
    if (done.has(slot)) continue;
    const current = d.get(slot);
    if (current?.kind === 'text' && current.labelOf !== undefined) out.push({ type: 'update', before: current, after: unlinked(current) });
  }
  return out;
}
