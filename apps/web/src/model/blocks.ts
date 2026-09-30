import type { Entity, EntityGeometry, TextAlign } from './entities';
import type { Vec2 } from './geometry';

/**
 * Block definitions (docs/adr/0144) and the rules a drawing's blocks keep:
 * the same rules, in the same order and the same words, as the Rust
 * contracts' (`kentos_contracts::blocks`), which the file codec and the
 * desktop's document check. The shared document fixtures
 * (fixtures/document-ops/v1/blocks.json) hold the two together.
 *
 * - each definition's id once, and its name once, compared with Turkish case
 *   folding (`nameKey`); a name is not empty nor only white space;
 * - an attribute definition's tag is not empty and is once per definition;
 * - every insert names a definition of the drawing, its scale positive and finite;
 * - no definition holds itself, directly or through others, and nesting is
 *   at most `MAX_BLOCK_DEPTH` levels (a definition without inserts is one
 *   level). A cycle longer than that is reported as too deep.
 */

/** One attribute an insert shows as text (docs/adr/0144 §7): the insert's attribute `tag`, else `value`, at `p`. */
export interface AttributeDefinition {
  tag: string;
  /** What Blok ekle asks for it. */
  prompt?: string;
  /** The default value. */
  value?: string;
  p: Vec2;
  /** Metres, in the definition's size. */
  height: number;
  /** Degrees, counter-clockwise from east, as a text's. */
  rotation: number;
  /** Where `p` is on the text, as a text's (docs/adr/0145). */
  align?: TextAlign;
  /** The letters' width times this, as a text's (docs/adr/0145). */
  widthFactor?: number;
}

/**
 * A block definition: objects drawn once, placed many times. Its objects'
 * ids are local to it; their layer is kept, but an insert draws them on its
 * own layer, each with its own colour or line weight when it has one.
 */
export interface BlockDefinition {
  /** Persistent id (UUID); inserts name their definition by it. */
  id: string;
  name: string;
  /** The point placed at an insert's `p`, in the definition's coordinates. */
  base: Vec2;
  entities: Entity[];
  attributes?: AttributeDefinition[];
  description?: string;
}

/**
 * A block's piece as the geometry store gives it (`PickIndex.blockPieces`):
 * a shape (never an insert) with the colour and line weight it draws with
 * when it has its own (or a nested insert's).
 */
export type BlockPiece = EntityGeometry & { color?: string; lineWeight?: number; attribute?: string };

/**
 * What a block's text piece shows on an insert with these attributes
 * (docs/adr/0144 §7): an attribute's piece the insert's value under its tag,
 * else its text (the default); any other its text. Empty: nothing is drawn.
 */
export function pieceText(piece: { text: string; attribute?: string }, attrs: Readonly<Record<string, string>>): string {
  return (piece.attribute !== undefined && attrs[piece.attribute]) || piece.text;
}

/**
 * The attribute rows of an insert of `block` (the Öznitelikler panel): each
 * definition's tag, in its order, with the value the insert shows (its own,
 * else the default).
 */
export function attributeRows(block: BlockDefinition | undefined, attrs: Readonly<Record<string, string>>): { tag: string; value: string }[] {
  return (block?.attributes ?? []).map((a) => ({ tag: a.tag, value: attrs[a.tag] || a.value || '' }));
}

/** The deepest nesting of blocks. */
export const MAX_BLOCK_DEPTH = 16;

/** Where an insert is: in a definition's objects, or the drawing's own (`definition: null`). */
export interface BlockPlace {
  definition: number | null;
  entity: number;
}

/** The first broken rule; indices are places in the definition list and in a definition's attributes. */
export type BlockFault =
  | { kind: 'emptyName'; definition: number }
  | { kind: 'duplicateId'; definition: number; first: number }
  | { kind: 'duplicateName'; definition: number; first: number }
  | { kind: 'emptyTag'; definition: number; attribute: number }
  | { kind: 'duplicateTag'; definition: number; attribute: number; first: number }
  | { kind: 'badScale'; at: BlockPlace }
  | { kind: 'unknownBlock'; at: BlockPlace }
  | { kind: 'cycle'; definition: number }
  | { kind: 'tooDeep'; definition: number };

/** Unicode's White_Space characters (Rust's `char::is_whitespace`; JavaScript's `\s` differs). */
const WHITE_SPACE = new Set(['\t', '\n', '\v', '\f', '\r', ' ', '\u0085', ' ', ' ', ' ', ' ', ' ', ' ', '　']);
for (let c = 0x2000; c <= 0x200a; c++) WHITE_SPACE.add(String.fromCharCode(c));

/** A definition's name as names are compared: each character lowercased, Turkish I (I → ı, İ → i). */
export function nameKey(name: string): string {
  let key = '';
  for (const c of name) key += c === 'I' ? 'ı' : c === 'İ' ? 'i' : c.toLowerCase();
  return key;
}

/** Whether `name` may name a definition: something besides white space. */
export function nameOk(name: string): boolean {
  for (const c of name) if (!WHITE_SPACE.has(c)) return true;
  return false;
}

/** `name` without white space at either end, as Rust's `str::trim` has it (Unicode White_Space): a name as a window takes it. */
export function trimName(name: string): string {
  const cs = [...name];
  let a = 0;
  let b = cs.length;
  while (a < b && WHITE_SPACE.has(cs[a])) a++;
  while (b > a && WHITE_SPACE.has(cs[b - 1])) b--;
  return cs.slice(a, b).join('');
}

/** The first “Blok n” (n = 1, 2, …) no name of `blocks` is, names compared by `nameKey`: what Blok oluştur offers. */
export function freeBlockName(blocks: readonly { name: string }[]): string {
  const taken = new Set(blocks.map((b) => nameKey(b.name)));
  let n = 1;
  while (taken.has(nameKey(`Blok ${n}`))) n++;
  return `Blok ${n}`;
}

/**
 * The names the blocks an import brings go in as (docs/adr/0144 §5): each
 * its own, or “Ad (2)”, “Ad (3)” … when the drawing or a block before it in
 * the import has that name (names compared by `nameKey`). The contracts'
 * `blocks::import_names` is the same.
 */
export function importNames(taken: Iterable<string>, incoming: Iterable<string>): string[] {
  const keys = new Set([...taken].map(nameKey));
  const out: string[] = [];
  for (const name of incoming) {
    let chosen = name;
    for (let k = 2; keys.has(nameKey(chosen)); k++) chosen = `${name} (${k})`;
    keys.add(nameKey(chosen));
    out.push(chosen);
  }
  return out;
}

/**
 * An insert's turn in radians from degrees as typed (Blok ekle's Dönüş,
 * Öznitelikler): taken into 0–360°, then (d · π) / 180, so a quarter turn is
 * the core's exact one. The contracts' `blocks::turn_of` is the same.
 */
export const turnOf = (degrees: number): number => ((((degrees % 360) + 360) % 360) * Math.PI) / 180;

/** Whether an insert's scale is allowed: positive and finite. */
export function scaleOk(scale: number): boolean {
  return Number.isFinite(scale) && scale > 0;
}

/** How many inserts of `block` the list holds directly. */
export function blockUses(entities: Iterable<Entity>, block: string): number {
  let n = 0;
  for (const e of entities) if (e.kind === 'insert' && e.block === block) n++;
  return n;
}

/**
 * How often a definition is placed: its inserts among the drawing's own
 * objects, and inside definitions (the Bloklar panel's count; a block placed
 * anywhere is not deleted). The contracts' `blocks::Placements`.
 */
export interface Placements {
  drawing: number;
  nested: number;
}

/** Every definition's placements, in the definitions' order: one pass over the drawing's objects and one over the definitions'. */
export function placements(blocks: readonly BlockDefinition[], drawing: Iterable<Entity>): Placements[] {
  const at = new Map(blocks.map((b, i) => [b.id, i]));
  const out = blocks.map(() => ({ drawing: 0, nested: 0 }));
  for (const e of drawing) {
    const k = e.kind === 'insert' ? at.get(e.block) : undefined;
    if (k !== undefined) out[k].drawing++;
  }
  for (const b of blocks)
    for (const e of b.entities) {
      const k = e.kind === 'insert' ? at.get(e.block) : undefined;
      if (k !== undefined) out[k].nested++;
    }
  return out;
}

/**
 * How many definitions these objects place, with those nested in them: the
 * blocks a DXF export writes (docs/adr/0144 §5; contracts `placed_blocks`).
 * An insert of a block the list does not have places none.
 */
export function placedBlocks(blocks: readonly BlockDefinition[], objects: Iterable<Entity>): number {
  const byId = new Map(blocks.map((b) => [b.id, b]));
  const seen = new Set<string>();
  const open: string[] = [];
  for (const e of objects) if (e.kind === 'insert') open.push(e.block);
  for (let id = open.pop(); id !== undefined; id = open.pop()) {
    const b = byId.get(id);
    if (!b || seen.has(id)) continue;
    seen.add(id);
    for (const e of b.entities) if (e.kind === 'insert') open.push(e.block);
  }
  return seen.size;
}

/** Whether an insert places it anywhere. */
export const placed = (p: Placements): boolean => p.drawing + p.nested > 0;

/**
 * The definitions whose drawing may differ between two lists (a change, an
 * undo, another editor's): the made, changed and removed ones (the document
 * gives a changed definition a new object and keeps an unchanged one's), and
 * every definition placing one of them, however deep. What an insert of any
 * of them shows is drawn again.
 */
export function changedDefinitions(before: readonly BlockDefinition[], after: readonly BlockDefinition[]): Set<string> {
  const was = new Map(before.map((b) => [b.id, b]));
  const out = new Set<string>();
  for (const b of after) if (was.get(b.id) !== b) out.add(b.id);
  const kept = new Set(after.map((b) => b.id));
  for (const id of was.keys()) if (!kept.has(id)) out.add(id);
  for (let grew = out.size > 0; grew; ) {
    grew = false;
    for (const b of after)
      if (!out.has(b.id) && b.entities.some((e) => e.kind === 'insert' && out.has(e.block))) {
        out.add(b.id);
        grew = true;
      }
  }
  return out;
}

/** The definitions checked on their own (names, ids, tags): each id's place, or the fault. */
export function checkDefinitions(blocks: readonly BlockDefinition[]): { index: Map<string, number> } | { fault: BlockFault } {
  const index = new Map<string, number>();
  const names = new Map<string, number>();
  for (let i = 0; i < blocks.length; i++) {
    const block = blocks[i];
    if (!nameOk(block.name)) return { fault: { kind: 'emptyName', definition: i } };
    const sameId = index.get(block.id);
    if (sameId !== undefined) return { fault: { kind: 'duplicateId', definition: i, first: sameId } };
    index.set(block.id, i);
    const key = nameKey(block.name);
    const sameName = names.get(key);
    if (sameName !== undefined) return { fault: { kind: 'duplicateName', definition: i, first: sameName } };
    names.set(key, i);
    const attributes = block.attributes ?? [];
    const seen = new Map<string, number>();
    for (let a = 0; a < attributes.length; a++) {
      const tag = attributes[a].tag;
      if (!tag) return { fault: { kind: 'emptyTag', definition: i, attribute: a } };
      const first = seen.get(tag);
      if (first !== undefined) return { fault: { kind: 'duplicateTag', definition: i, attribute: a, first } };
      seen.set(tag, a);
    }
  }
  return { index };
}

/** The inserts among `entities` (a definition's, or the drawing's own with `definition` null): scales allowed, blocks known. */
export function checkInserts(entities: readonly Entity[], definition: number | null, index: ReadonlyMap<string, number>): BlockFault | null {
  for (let entity = 0; entity < entities.length; entity++) {
    const e = entities[entity];
    if (e.kind !== 'insert') continue;
    if (!scaleOk(e.scale)) return { kind: 'badScale', at: { definition, entity } };
    if (!index.has(e.block)) return { kind: 'unknownBlock', at: { definition, entity } };
  }
  return null;
}

/**
 * Each definition's nesting depth, or the cycle or the too deep nesting:
 * depth first, the stack never deeper than the limit. Every insert in the
 * definitions must name a known one (`checkInserts` first).
 */
export function checkNesting(blocks: readonly BlockDefinition[], index: ReadonlyMap<string, number>): { depth: number[] } | { fault: BlockFault } {
  const depth = blocks.map(() => 0);
  const stack: number[] = [];
  const visit = (i: number): BlockFault | null => {
    stack.push(i);
    let deepest = 0;
    for (const e of blocks[i].entities) {
      if (e.kind !== 'insert') continue;
      const k = index.get(e.block);
      if (k === undefined) continue;
      if (depth[k] === 0) {
        if (stack.includes(k)) return { kind: 'cycle', definition: k };
        if (stack.length === MAX_BLOCK_DEPTH) return { kind: 'tooDeep', definition: stack[0] };
        const fault = visit(k);
        if (fault) return fault;
      }
      if (stack.length + depth[k] > MAX_BLOCK_DEPTH) return { kind: 'tooDeep', definition: stack[0] };
      deepest = Math.max(deepest, depth[k]);
    }
    stack.pop();
    depth[i] = deepest + 1;
    return null;
  };
  for (let i = 0; i < blocks.length; i++) {
    if (depth[i] !== 0) continue;
    const fault = visit(i);
    if (fault) return { fault };
  }
  return { depth };
}

/** The definitions checked whole (their names, ids, tags, inserts and nesting); the first fault, or null. */
export function definitionsFault(blocks: readonly BlockDefinition[]): BlockFault | null {
  const own = checkDefinitions(blocks);
  if ('fault' in own) return own.fault;
  for (let i = 0; i < blocks.length; i++) {
    const fault = checkInserts(blocks[i].entities, i, own.index);
    if (fault) return fault;
  }
  const nesting = checkNesting(blocks, own.index);
  return 'fault' in nesting ? nesting.fault : null;
}

/** A drawing's blocks and its own objects checked whole: the first fault, or null. */
export function blocksFault(blocks: readonly BlockDefinition[], entities: readonly Entity[]): BlockFault | null {
  const fault = definitionsFault(blocks);
  if (fault) return fault;
  return checkInserts(entities, null, new Map(blocks.map((b, i) => [b.id, i])));
}

/** What an edit that breaks the rule is told (the documents' refusals): `name` gives a definition's name. */
export function blockFaultMessage(fault: BlockFault, name: (i: number) => string): string {
  switch (fault.kind) {
    case 'emptyName':
      return 'Blok adı boş olamaz; bir ad yazın.';
    case 'duplicateId':
      return 'Bu kimlikte bir blok zaten var; blok eklenmedi.';
    case 'duplicateName':
      return `Çizimde “${name(fault.first)}” adında bir blok var; başka bir ad verin.`;
    case 'emptyTag':
      return `“${name(fault.definition)}” bloğunda boş bir öznitelik etiketi var; her özniteliğe bir etiket verin.`;
    case 'duplicateTag':
      return `“${name(fault.definition)}” bloğunda bir öznitelik etiketi iki kez var; her etiket bir kez olmalı.`;
    case 'badScale':
      return 'Blok ölçeği pozitif bir sayı olmalı.';
    case 'unknownBlock':
      return 'Yerleştirilen blok çizimde tanımlı değil.';
    case 'cycle':
      return `“${name(fault.definition)}” bloğu kendini içeremez (doğrudan ya da başka bloklar yoluyla).`;
    case 'tooDeep':
      return `Bloklar en çok ${MAX_BLOCK_DEPTH} düzey iç içe olabilir; “${name(fault.definition)}” bloğu daha derin.`;
  }
}
