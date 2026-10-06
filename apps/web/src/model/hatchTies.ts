import type { BlockDefinition } from '../contracts/generated/BlockDefinition';
import type { DrawingEntity, Entity, HatchAssoc, HatchEntity } from './entities';
import type { ObjectOp } from './linkedTexts';
import { hatchCutout, hatchRegion } from './ops/hatchPatterns';
import { sameJson } from './sameJson';

/**
 * Associative hatches (docs/adr/0186 §6): a hatch made inside a closed object with İlişkili on knows its objects
 * (`assoc`: the closed object, its islands, the texts and inserts left open, and the point clicked inside). Before a
 * step is recorded (`CadDocument`'s commit), after the linked texts, its changes are read:
 *
 * - a hatch one of whose objects the step changed or removed has its region cut again by the core's rule
 *   (`hatchRegion`), its pattern and colour kept; a removed island or cutout leaves the list;
 * - a hatch whose closed object is gone, no longer closed, or leaves no region stays where it is and follows nothing;
 * - a hatch whose ring or holes the step itself changed (a grip, the vertex table) follows nothing, unless its objects
 *   changed in the same step (then the rule writes it).
 *
 * The changes join the step: one undo takes them back with the rest. The desktop's is kentos_domain's hatch_ties.rs;
 * both run fixtures/document-ops/v1/hatch-ties.json.
 */

/** What the pass reads of the drawing, as the step left it. */
export interface TiedDrawing {
  /** The object in this slot. */
  get(id: number): DrawingEntity | undefined;
  /** The object with this persistent id, while it is in the drawing. */
  byUid(uid: string): DrawingEntity | undefined;
  /** The slots of the hatches whose region follows the object with this persistent id. */
  tiedTo(uid: string): Iterable<number>;
  /** The project's drawing typeface (`DrawingFont`): a text's box is measured in it. */
  font: string;
  /** The drawing's block definitions (an insert's box needs its block's). */
  blocks(): readonly BlockDefinition[];
}

type TiedHatch = HatchEntity & { uid: string };

/** A hatch that follows nothing: it stays as it is. */
function untied(h: TiedHatch): TiedHatch {
  const out = { ...h };
  delete out.assoc;
  return out;
}

/** The hatch `h` cut again from its objects now, or none when the rule gives no region. */
function retied(h: TiedHatch, d: TiedDrawing): TiedHatch | undefined {
  const a = h.assoc;
  const outer = a && d.byUid(a.outer);
  if (!a || !outer) return undefined;
  const present = (ids: readonly string[] | undefined) => (ids ?? []).flatMap((id) => {
    const e = d.byUid(id);
    return e ? [[id, e] as const] : [];
  });
  const islands = present(a.islands);
  const cutouts = present(a.cutouts);
  const blocks = cutouts.some(([, e]) => e.kind === 'insert') ? d.blocks() : null;
  const boxes = cutouts.map(([, e]) => hatchCutout(e as Entity, blocks, d.font) ?? []);
  const r = hatchRegion(outer as Entity, islands.map(([, e]) => e as Entity), boxes, a.seed);
  if (!r) return undefined;
  const assoc: HatchAssoc = { outer: a.outer, ...(islands.length && { islands: islands.map(([id]) => id) }), ...(cutouts.length && { cutouts: cutouts.map(([id]) => id) }), seed: a.seed };
  const next: TiedHatch = { ...h, ring: r.ring.map((p) => ({ x: p.x, y: p.y })), assoc };
  if (r.holes.length) next.holes = r.holes.map((ring) => ring.map((p) => ({ x: p.x, y: p.y })));
  else delete next.holes;
  return next;
}

/** What keeps the associative hatches with their objects after `ops` (a step about to be recorded); the caller applies it. */
export function followHatches(ops: readonly ObjectOp[], d: TiedDrawing): ObjectOp[] {
  // The objects the step changed or removed, by persistent id, and the hatches whose ring or holes it changed itself.
  const objects: string[] = [];
  const seen = new Set<string>();
  const edited = new Set<number>();
  for (const op of ops) {
    const e = op.type === 'update' ? op.after : op.entity;
    if (!seen.has(e.uid)) {
      seen.add(e.uid);
      objects.push(e.uid);
    }
    if (op.type === 'update' && op.before.kind === 'hatch' && op.after.kind === 'hatch' && op.after.assoc && (!sameJson(op.before.ring, op.after.ring) || !sameJson(op.before.holes ?? null, op.after.holes ?? null))) {
      edited.add(op.after.id);
    }
  }
  const out: ObjectOp[] = [];
  const done = new Set<number>();
  for (const uid of objects) {
    for (const slot of [...d.tiedTo(uid)].sort((a, b) => a - b)) {
      if (done.has(slot)) continue;
      done.add(slot);
      const current = d.get(slot);
      if (current?.kind !== 'hatch') continue;
      const next = retied(current, d) ?? untied(current);
      if (!sameJson(next, current)) out.push({ type: 'update', before: current, after: next });
    }
  }
  // A hatch edited by itself, its objects unchanged: it follows nothing now.
  for (const slot of [...edited].sort((a, b) => a - b)) {
    if (done.has(slot)) continue;
    const current = d.get(slot);
    if (current?.kind === 'hatch' && current.assoc) out.push({ type: 'update', before: current, after: untied(current) });
  }
  return out;
}

/** The persistent ids of the objects a hatch's region follows (docs/adr/0186 §6); none for any other object. */
export function tiesOf(e: DrawingEntity): string[] {
  if (e.kind !== 'hatch' || !e.assoc) return [];
  return [e.assoc.outer, ...(e.assoc.islands ?? []), ...(e.assoc.cutouts ?? [])];
}
