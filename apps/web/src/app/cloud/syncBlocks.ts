import { blockFaultMessage, definitionsFault, importNames, nameKey, type BlockDefinition } from '../../model/blocks';
import type { CadDocument, ExternalMeta } from '../../model/document';
import type { Entity } from '../../model/entities';
import { readBlockDefinitions } from '../../model/snapshot';
import type { DraftBlock } from './drafts';
import type { SyncConflict, SyncCore } from './syncCore';
import { blockJson, blockKey } from './tracker';

/**
 * The block definitions of an open database project as they come from the
 * server (docs/adr/0144 §5): after other editors' events, when a conflict is
 * answered with the server's copy, after a device draft's command went
 * again, and when the server refused to remove a definition still placed.
 * The drawing's list and the block tracker (tracker.ts) follow the server's
 * as objects do (syncRemote.ts, syncCore.ts), with rules of their own:
 *
 * - A definition with unsent changes here keeps them: an event that changed
 *   it too makes it a conflict; otherwise only its server version moves on,
 *   so the change goes out over it.
 * - Data wins over a removal, as with layers (keptLayers.ts): a definition
 *   removed on the server stays while this drawing's objects or definitions
 *   place it, and goes back to the server; one removed here comes back from
 *   the server when an object that arrives places it.
 * - A name is the drawing's once: a definition made or renamed here whose
 *   name someone else took first gets a free one (“Kapı (2)”), as an
 *   import's does, and is sent so.
 */

/** A definition the server has, checked, and its version. */
export interface ServerBlock {
  block: BlockDefinition;
  version: string;
}

/** How the server's definitions are taken. */
export interface Taking {
  /**
   * The definitions the events named: one of them with unsent changes here
   * that moved on the server is a conflict. Without it, the server's version
   * of such a definition is only learned (after a draft's command went again).
   */
  events?: ReadonlySet<string>;
  /** Taken as the server has them even with unsent changes here (a conflict answered with the server's copy). */
  force?: ReadonlySet<string>;
  /** The blocks the drawing's objects place once the incoming change is in; asked only when a definition would go. */
  placed: () => ReadonlySet<string>;
}

/** The server's definitions merged into the drawing's, not applied yet (`applyMerge`). */
export interface BlockMerge {
  /** The drawing's definitions afterwards: its own order, then the server's new ones in the server's. */
  list: BlockDefinition[];
  /** What the tracker learns, by id: the server's version and text (text null: the drawing's definition, once in); null: the server has none. */
  learned: Map<string, { version: string; json: string | null } | null>;
  conflicts: SyncConflict[];
  /** What the user is told. */
  notes: string[];
}

/** The server's definitions, checked like a file's; null (and a warning) when they cannot be read. */
export async function serverBlocks(core: SyncCore): Promise<ServerBlock[] | null> {
  const { o } = core;
  const page = await o.api.blocks(o.tenantId, o.projectId);
  const read = readBlockDefinitions(page.blocks.map((r) => r.block));
  if (!read.ok) {
    o.warn(`Sunucudaki blok tanımları okunamadı: ${read.error}`);
    return null;
  }
  return read.blocks.map((block, i) => ({ block, version: page.blocks[i].version }));
}

/** The blocks the drawing's objects place, but those `leaving` it, and the ones `incoming` objects place. */
export function placedIds(doc: CadDocument, leaving: (uid: string) => boolean = () => false, incoming: Iterable<unknown> = []): Set<string> {
  const out = new Set<string>();
  for (const e of doc.all()) if (e.kind === 'insert' && !leaving(e.uid!)) out.add(e.block);
  for (const e of incoming) {
    const x = e as { kind?: unknown; block?: unknown } | null;
    if (x?.kind === 'insert' && typeof x.block === 'string') out.add(x.block);
  }
  return out;
}

/** Whether an incoming object places a block this drawing has no definition of. */
export function placesMissing(doc: CadDocument, entity: unknown): boolean {
  const x = entity as { kind?: unknown; block?: unknown } | null;
  return x?.kind === 'insert' && typeof x.block === 'string' && !doc.block(x.block);
}

export const keptBlockText = (name: string): string =>
  `“${name}” bloğunu başka biri sildi; bu çizimde onu yerleştiren gönderilmemiş nesneleriniz olduğu için blok kaldı ve yeniden kaydedilecek.`;
export const restoredBlockText = (name: string): string => `“${name}” bloğu başka birinin yerleştirmesinde kullanılıyor; silinmedi, çizime geri kondu.`;
export const renamedBlockText = (was: string, now: string): string => `Başka biri de “${was}” adında bir blok tanımladı; sizinki “${now}” oldu.`;

/**
 * The drawing's definitions with the server's taken in (the module's
 * comment says how); the reason instead when the result would break a
 * block rule (a nesting both sides changed at once): then nothing is taken.
 */
export function mergeBlocks(core: SyncCore, server: readonly ServerBlock[], how: Taking): BlockMerge | { error: string } {
  const doc = core.o.doc;
  const tracker = core.blocks;
  const byServer = new Map(server.map((s) => [s.block.id, s]));
  const learned: BlockMerge['learned'] = new Map();
  const conflicts: SyncConflict[] = [];
  const notes: string[] = [];
  // Each id's definition afterwards (absent: none), and whether it holds unsent changes made here.
  const next = new Map<string, BlockDefinition>();
  const mine = new Set<string>();
  const ids = new Set([...doc.blocks.value.map((b) => b.id), ...byServer.keys(), ...tracker.ids()]);
  for (const id of ids) {
    const here = doc.block(id);
    const s = byServer.get(id);
    const moved = (s?.version ?? null) !== (tracker.get(id)?.version ?? null);
    if (!tracker.differs(doc, id) || how.force?.has(id)) {
      // Nothing unsent here: the server's, whatever it is.
      if (s) next.set(id, s.block);
      learned.set(id, s ? { version: s.version, json: null } : null);
      continue;
    }
    if (!here && !s) {
      // Removed here and there alike: nothing is left to send or to ask.
      learned.set(id, null);
      continue;
    }
    mine.add(id);
    if (here) next.set(id, here);
    if (!moved) continue;
    if (how.events?.has(id)) conflicts.push({ featureId: blockKey(id), reason: s ? 'remote' : 'deleted', server: null, actual: s?.version ?? null });
    else if (!how.events) learned.set(id, s ? { version: s.version, json: blockJson(s.block) } : null);
  }
  // Data wins over a removal: whatever the objects or the definitions left place stays, or comes back.
  let placed: ReadonlySet<string> | null = null;
  for (let grew = true; grew; ) {
    grew = false;
    const inside = new Set<string>();
    for (const b of next.values()) for (const e of b.entities) if (e.kind === 'insert') inside.add(e.block);
    for (const id of ids) {
      if (next.has(id)) continue;
      placed ??= how.placed();
      if (!placed.has(id) && !inside.has(id)) continue;
      const here = doc.block(id);
      const s = byServer.get(id);
      if (here && !s) {
        // Removed on the server, placed here: it stays, and is made there again, as this drawing's.
        next.set(id, here);
        learned.set(id, null);
        mine.add(id);
        notes.push(keptBlockText(here.name));
      } else if (s) {
        // Removed here, placed by what arrives: the server's comes back, and the removal goes with its conflict.
        next.set(id, s.block);
        learned.set(id, { version: s.version, json: null });
        mine.delete(id);
        const k = conflicts.findIndex((c) => c.featureId === blockKey(id));
        if (k >= 0) conflicts.splice(k, 1);
        notes.push(restoredBlockText(s.block.name));
      } else continue;
      grew = true;
    }
  }
  // The drawing's order, then the server's new definitions in the server's.
  const order = [...doc.blocks.value.map((b) => b.id), ...server.map((s) => s.block.id)];
  const list: BlockDefinition[] = [];
  const seen = new Set<string>();
  for (const id of order) {
    const b = next.get(id);
    if (!b || seen.has(id)) continue;
    seen.add(id);
    list.push(b);
  }
  // A name taken by someone else first: ours gives way.
  const theirs = new Map(list.filter((b) => !mine.has(b.id)).map((b) => [nameKey(b.name), b]));
  for (let i = 0; i < list.length; i++) {
    const b = list[i];
    if (!mine.has(b.id) || !theirs.has(nameKey(b.name))) continue;
    const [name] = importNames(
      list.filter((x) => x !== b).map((x) => x.name),
      [b.name],
    );
    notes.push(renamedBlockText(b.name, name));
    list[i] = { ...b, name };
  }
  const fault = definitionsFault(list);
  if (fault) return { error: blockFaultMessage(fault, (i) => list[i]?.name ?? '') };
  return { list, learned, conflicts, notes };
}

/**
 * Puts a merge into the drawing with the objects that come with it, as one
 * change without history (`CadDocument.applyExternal`), and tells the
 * tracker what the server has.
 */
export function applyMerge(core: SyncCore, merge: BlockMerge, objects: { put?: readonly Entity[]; remove?: readonly number[]; meta?: ExternalMeta } = {}): void {
  const doc = core.o.doc;
  core.takingBlocks = true;
  try {
    doc.applyExternal({ ...objects, blocks: merge.list });
  } finally {
    core.takingBlocks = false;
  }
  for (const [id, t] of merge.learned) {
    if (!t) core.blocks.set(id, null);
    else {
      const json = t.json ?? (doc.block(id) ? blockJson(doc.block(id)!) : null);
      core.blocks.set(id, json === null ? null : { version: t.version, json });
    }
  }
  core.blocksDirty = core.blocks.plan(doc).length > 0;
}

/**
 * Takes the server's definitions now, with the drawing's objects as they
 * are (`force`: taken even with unsent changes here). The notes are told.
 * False when they could not be read or would break a block rule (said).
 */
export async function takeServerBlocks(core: SyncCore, force: ReadonlySet<string> = new Set(), keep = false): Promise<boolean> {
  const server = await serverBlocks(core);
  if (!server || core.closed) return false;
  await core.whenIdle();
  if (core.closed) return false;
  const merge = mergeBlocks(core, server, { force, ...(keep ? {} : { events: new Set<string>() }), placed: () => placedIds(core.o.doc) });
  if ('error' in merge) {
    core.o.warn(unmergedText(merge.error));
    return false;
  }
  applyMerge(core, merge);
  for (const n of merge.notes) core.o.warn(n);
  return true;
}

/**
 * A device draft's made and changed definitions put into the drawing
 * (syncRestore.ts), each checked like a file's with the list it goes into:
 * a made one after the drawing's, a changed one in its place. One whose
 * name someone else took meanwhile gets a free one; one the list cannot take
 * (a nesting someone else changed meanwhile) is held in the device draft,
 * unsent, and said. Local work: no tracker changes, it counts as unsent.
 * The removals wait for the draft's objects (`draftRemovals`).
 */
export function draftUpserts(core: SyncCore, changes: readonly (readonly [string, DraftBlock])[]): string[] {
  const doc = core.o.doc;
  let list = [...doc.blocks.value];
  const put: string[] = [];
  for (const [id, change] of changes) {
    if (!change.block) continue;
    let block = change.block as unknown as BlockDefinition;
    const others = list.filter((b) => b.id !== id);
    if (others.some((b) => nameKey(b.name) === nameKey(block.name))) {
      const [name] = importNames(
        others.map((b) => b.name),
        [block.name],
      );
      core.o.warn(renamedBlockText(block.name, name));
      block = { ...block, name };
    }
    const at = list.findIndex((b) => b.id === id);
    const candidate = at >= 0 ? list.map((b, i) => (i === at ? block : b)) : [...list, block];
    const read = readBlockDefinitions(candidate);
    if (!read.ok) {
      core.heldBlocks.set(id, change);
      core.o.warn(`Cihazdaki taslakta bir blok tanımı çizime konamadı (“${block.name}”): ${read.error}. Değişiklik bu cihazda saklanıyor, gönderilmedi.`);
      continue;
    }
    list = candidate;
    put.push(id);
  }
  if (put.length) doc.applyExternal({ blocks: list });
  return put;
}

/**
 * A device draft's removed definitions taken out of the drawing, once its
 * objects are in: one that the drawing's objects or definitions still place
 * (someone else's insert, meanwhile) stays, and is said.
 */
export function draftRemovals(core: SyncCore, ids: readonly string[]): string[] {
  const doc = core.o.doc;
  const gone = new Set(ids.filter((id) => doc.block(id)));
  if (!gone.size) return [];
  const placed = placedIds(doc);
  // A definition that stays may place one the draft removes: that one stays too.
  for (let grew = true; grew; ) {
    grew = false;
    const inside = new Set<string>();
    for (const b of doc.blocks.value) if (!gone.has(b.id)) for (const e of b.entities) if (e.kind === 'insert') inside.add(e.block);
    for (const id of gone) {
      if (!placed.has(id) && !inside.has(id)) continue;
      gone.delete(id);
      grew = true;
      core.o.warn(`“${doc.block(id)?.name ?? id}” bloğu başka birinin yerleştirmesinde kullanılıyor; taslaktaki silinmesi uygulanmadı.`);
    }
  }
  if (gone.size) doc.applyExternal({ blocks: doc.blocks.value.filter((b) => !gone.has(b.id)) });
  return [...gone];
}

/** What the user hears when the server's definitions cannot go in with the ones changed here. */
export const unmergedText = (why: string): string =>
  `Sunucudaki blok tanımları bu çizimdekilerle birleştirilemedi (${why}); bloklardaki gönderilmemiş değişikliklerinizi gözden geçirin.`;
