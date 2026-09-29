import type { Entity } from './entities';
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
