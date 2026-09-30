import { afterEach, describe, expect, it, vi } from 'vitest';
import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { ProjectChanges } from '../../contracts/generated/ProjectChanges';
import type { BlockDefinition } from '../../model/blocks';
import type { Entity } from '../../model/entities';
import { MemoryDraftStore } from './drafts';
import { keptBlockText, renamedBlockText, restoredBlockText } from './syncBlocks';
import { disposeAll, pt, reopen, setup, storedDraft, wire } from './syncTesting';
import { BlockTracker, blockKey } from './tracker';

/**
 * Block definitions of a database project in the web's autosave
 * (docs/adr/0144 §5): sent with the objects, other editors' taken in, their
 * conflicts, the rules that keep data over a removal and a name once, and
 * the device draft. The fake server follows the real one's rules
 * (crates/server/application/src/changes.rs); the real one is tested in
 * crates/server/application/tests/blocks.rs and end to end in the browser.
 */

afterEach(disposeAll);

const line = { kind: 'line' as const, id: 1, layerId: '', a: { x: 0, y: 0 }, b: { x: 1, y: 0 }, attrs: {} };
const def = (name: string, inside: Entity[] = [line as Entity]): BlockDefinition => ({ id: crypto.randomUUID(), name, base: { x: 0, y: 0 }, entities: inside });
const insertOf = (block: string, x = 10) => ({ kind: 'insert' as const, layerId: 'cizim', block, p: { x: 486500 + x, y: 4420210 }, scale: 1, rotation: 0, attrs: {} });
/** Another editor's insert as it goes to the server. */
const theirInsert = (block: string) => wire({ ...insertOf(block), id: 0 } as Entity);

/** The commands the server is sent, in order. */
function sent(server: { command: (e: CommandEnvelope) => Promise<unknown> }): CommandEnvelope[] {
  const out: CommandEnvelope[] = [];
  const real = server.command.bind(server);
  vi.spyOn(server, 'command').mockImplementation((e: CommandEnvelope) => {
    out.push(structuredClone(e));
    return real(e) as never;
  });
  return out;
}
const inputOf = (e: CommandEnvelope) => e.input as ProjectChanges;

describe('block definitions in the cloud autosave', () => {
  it('a block made and placed goes with its insert in one command, and opens again as it was', async () => {
    const { doc, server, sync } = setup();
    const commands = sent(server);
    const b = def('Kapı');
    doc.addBlock(b);
    const ins = doc.add(insertOf(b.id));
    expect(sync.pending.value).toBe(2);
    expect(await sync.flush()).toBe(true);
    expect(commands).toHaveLength(1);
    expect(inputOf(commands[0]).blocks).toEqual([{ op: 'create', block: b }]);
    expect(server.blockStore.get(b.id)).toMatchObject({ version: 1, block: { name: 'Kapı' } });
    expect(server.store.get(ins.uid)?.entity).toMatchObject({ kind: 'insert', block: b.id });
    expect([sync.pending.value, sync.state.value, doc.dirty.value, sync.blockVersionOf(b.id)]).toEqual([0, 'saved', false, '1']);
    const again = await reopen(server);
    expect(again.doc.blocks.value).toEqual([b]);
    expect(again.blocks).toEqual([{ id: b.id, version: '1' }]);
    expect(again.doc.byUid(ins.uid)).toMatchObject({ kind: 'insert', block: b.id });
  });

  it('a changed definition goes over its version; a removed one with the last insert that placed it', async () => {
    const { doc, server, sync } = setup();
    const commands = sent(server);
    const b = def('Kapı');
    doc.addBlock(b);
    const ins = doc.add(insertOf(b.id));
    await sync.flush();
    doc.updateBlock({ ...b, name: 'Pencere' });
    expect(await sync.flush()).toBe(true);
    expect(commands[1].expectedVersions).toEqual({ [blockKey(b.id)]: '1' });
    expect(server.blockStore.get(b.id)).toMatchObject({ version: 2, block: { name: 'Pencere' } });
    doc.remove([ins.id]);
    doc.removeBlock(b.id);
    expect(await sync.flush()).toBe(true);
    expect(inputOf(commands[2])).toEqual({ features: [{ op: 'delete', id: ins.uid }], blocks: [{ op: 'delete', id: b.id }] });
    expect(commands[2].expectedVersions).toEqual({ [ins.uid]: '1', [blockKey(b.id)]: '2' });
    expect([server.blockStore.size, server.store.size, sync.blockVersionOf(b.id)]).toEqual([0, 0, undefined]);
  });

  it('an undo back to the saved definitions sends nothing', async () => {
    const { doc, server, sync } = setup();
    const b = def('Kapı');
    doc.addBlock(b);
    await sync.flush();
    const commits = server.commits;
    doc.updateBlock({ ...b, name: 'Pencere' });
    doc.undo();
    expect(sync.pending.value).toBe(0);
    expect(await sync.flush()).toBe(true);
    expect(server.commits).toBe(commits);
  });

  it('another editor’s change comes without an undo step, and the steps that made ours go', async () => {
    const { doc, server, sync } = setup();
    const b = def('Kapı');
    doc.addBlock(b);
    const ins = doc.add(insertOf(b.id));
    await sync.flush();
    await sync.receive([server.commitAs('baska', [], undefined, [{ op: 'update', block: { ...b, name: 'Kapı 2' } as never }])]);
    expect(doc.block(b.id)?.name).toBe('Kapı 2');
    expect([sync.state.value, sync.pending.value, sync.blockVersionOf(b.id)]).toEqual(['saved', 0, '2']);
    // Undo takes the insert back, not someone else's definition.
    expect(doc.undo()).not.toBeNull();
    expect(doc.byUid(ins.uid)).toBeUndefined();
    expect(doc.undo()).toBeNull();
    expect(doc.block(b.id)?.name).toBe('Kapı 2');
  });

  it('another editor’s new block and its insert come in one change', async () => {
    const { doc, server, sync } = setup();
    const b = def('Ağaç');
    const id = crypto.randomUUID();
    await sync.receive([server.commitAs('baska', [{ op: 'create', id, entity: theirInsert(b.id) }], undefined, [{ op: 'create', block: b as never }])]);
    expect(doc.blocks.value).toEqual([b]);
    expect(doc.byUid(id)).toMatchObject({ kind: 'insert', block: b.id });
    expect([sync.state.value, sync.blockVersionOf(b.id), sync.versionOf(id)]).toEqual(['saved', '1', '1']);
    // Removed by them with the insert: both go.
    await sync.receive([server.commitAs('baska2', [{ op: 'delete', id }], undefined, [{ op: 'delete', id: b.id }])]);
    expect([doc.blocks.value, doc.byUid(id), sync.blockVersionOf(b.id)]).toEqual([[], undefined, undefined]);
  });

  it('a definition changed here and there is a conflict: the server’s copy …', async () => {
    const { doc, server, sync } = setup();
    const b = def('Kapı');
    doc.addBlock(b);
    await sync.flush();
    doc.updateBlock({ ...b, name: 'Benim' });
    await sync.receive([server.commitAs('baska', [], undefined, [{ op: 'update', block: { ...b, name: 'Onların' } as never }])]);
    expect(sync.conflicts.value).toEqual([{ featureId: blockKey(b.id), reason: 'remote', server: null, actual: '2' }]);
    expect(doc.block(b.id)?.name).toBe('Benim');
    await sync.resolve('server');
    expect([doc.block(b.id)?.name, sync.state.value, server.blockStore.get(b.id)?.version]).toEqual(['Onların', 'saved', 2]);
  });

  it('… or mine over it', async () => {
    const { doc, server, sync } = setup();
    const b = def('Kapı');
    doc.addBlock(b);
    await sync.flush();
    // Theirs is committed before ours goes: the server refuses ours.
    server.commitAs('baska', [], undefined, [{ op: 'update', block: { ...b, name: 'Onların' } as never }]);
    doc.updateBlock({ ...b, name: 'Benim' });
    expect(await sync.flush()).toBe(false);
    expect(sync.conflicts.value).toEqual([{ featureId: blockKey(b.id), reason: 'changed', server: null, actual: '2' }]);
    await sync.resolve('mine');
    expect(server.blockStore.get(b.id)).toMatchObject({ version: 3, block: { name: 'Benim' } });
    expect([doc.block(b.id)?.name, sync.state.value, sync.blockVersionOf(b.id)]).toEqual(['Benim', 'saved', '3']);
  });

  it('a definition removed here and by someone else alike: nothing is asked, nothing sent', async () => {
    const { doc, server, sync } = setup();
    const b = def('Kapı');
    doc.addBlock(b);
    await sync.flush();
    doc.removeBlock(b.id);
    await sync.receive([server.commitAs('baska', [], undefined, [{ op: 'delete', id: b.id }])]);
    expect([sync.conflicts.value, sync.pending.value, sync.blockVersionOf(b.id)]).toEqual([[], 0, undefined]);
    const commits = server.commits;
    expect(await sync.flush()).toBe(true);
    expect(server.commits).toBe(commits);
  });

  it('a block someone placed is not removed: it comes back with their insert, and nothing is asked', async () => {
    const { doc, server, sync, warnings } = setup();
    const b = def('Kapı');
    doc.addBlock(b);
    await sync.flush();
    // They place it; we remove it before hearing so.
    const theirs = crypto.randomUUID();
    server.commitAs('baska', [{ op: 'create', id: theirs, entity: theirInsert(b.id) }]);
    doc.removeBlock(b.id);
    await sync.flush();
    await vi.waitFor(() => expect(sync.state.value).toBe('saved'));
    expect(doc.block(b.id)?.name).toBe('Kapı');
    expect(doc.byUid(theirs)).toMatchObject({ kind: 'insert', block: b.id });
    expect(sync.conflicts.value).toEqual([]);
    expect(warnings.filter((w) => w === restoredBlockText('Kapı'))).toHaveLength(1);
    expect(server.blockStore.get(b.id)?.version).toBe(1);
  });

  it('an insert of a block removed here arrives: the block comes back', async () => {
    const { doc, server, sync, warnings } = setup();
    const b = def('Kapı');
    doc.addBlock(b);
    await sync.flush();
    doc.removeBlock(b.id);
    const theirs = crypto.randomUUID();
    await sync.receive([server.commitAs('baska', [{ op: 'create', id: theirs, entity: theirInsert(b.id) }])]);
    expect([doc.block(b.id)?.name, doc.byUid(theirs)?.kind]).toEqual(['Kapı', 'insert']);
    expect(warnings).toContain(restoredBlockText('Kapı'));
    // The removal is gone with it: nothing waits.
    expect(sync.pending.value).toBe(0);
  });

  it('a block removed on the server while an unsent insert here places it stays, and is made again', async () => {
    const { doc, server, sync, warnings } = setup();
    const b = def('Kapı');
    doc.addBlock(b);
    await sync.flush();
    const mine = doc.add(insertOf(b.id));
    await sync.receive([server.commitAs('baska', [], undefined, [{ op: 'delete', id: b.id }])]);
    expect(doc.block(b.id)?.name).toBe('Kapı');
    expect(warnings).toContain(keptBlockText('Kapı'));
    expect(await sync.flush()).toBe(true);
    expect(server.blockStore.get(b.id)?.block.name).toBe('Kapı');
    expect(server.store.get(mine.uid)?.entity).toMatchObject({ block: b.id });
  });

  it('a name someone else took first: ours gives way, and goes so', async () => {
    const { doc, server, sync, warnings } = setup();
    await sync.flush();
    const mine = def('Blok 1');
    doc.addBlock(mine);
    const theirs = def('BLOK 1');
    await sync.receive([server.commitAs('baska', [], undefined, [{ op: 'create', block: theirs as never }])]);
    expect(doc.blocks.value.map((b) => [b.id, b.name])).toEqual([
      [mine.id, 'Blok 1 (2)'],
      [theirs.id, 'BLOK 1'],
    ]);
    expect(warnings).toContain(renamedBlockText('Blok 1', 'Blok 1 (2)'));
    expect(await sync.flush()).toBe(true);
    expect(server.blockStore.get(mine.id)?.block.name).toBe('Blok 1 (2)');
  });

  it('unsent definitions wait in the device draft and come back when the project opens again', async () => {
    const drafts = new MemoryDraftStore();
    const first = setup({ drafts });
    const b = def('Kapı');
    first.doc.addBlock(b);
    const ins = first.doc.add(insertOf(b.id));
    await first.sync.keepDraft();
    expect((await storedDraft(drafts))?.blocks).toEqual({ [b.id]: { base: null, block: b } });
    first.sync.dispose();
    const again = await reopen(first.server);
    const second = setup({ drafts, server: first.server, doc: again.doc, records: again.records, blocks: again.blocks });
    expect(await second.sync.restore(await drafts.get('u1/t/p'))).toBe(true);
    expect([second.doc.block(b.id)?.name, second.doc.byUid(ins.uid)?.kind, second.sync.pending.value]).toEqual(['Kapı', 'insert', 2]);
    expect(await second.sync.flush()).toBe(true);
    expect(first.server.blockStore.get(b.id)?.version).toBe(1);
    expect(await storedDraft(drafts)).toBeNull();
  });

  it('a draft’s definition someone changed meanwhile is a conflict, shown as mine until chosen', async () => {
    const drafts = new MemoryDraftStore();
    const first = setup({ drafts });
    const b = def('Kapı');
    first.doc.addBlock(b);
    await first.sync.flush();
    first.doc.updateBlock({ ...b, name: 'Benim' });
    await first.sync.keepDraft();
    first.sync.dispose();
    first.server.commitAs('baska', [], undefined, [{ op: 'update', block: { ...b, name: 'Onların' } as never }]);
    const again = await reopen(first.server);
    const second = setup({ drafts, server: first.server, doc: again.doc, records: again.records, blocks: again.blocks });
    await second.sync.restore(await drafts.get('u1/t/p'));
    expect(second.sync.conflicts.value).toEqual([{ featureId: blockKey(b.id), reason: 'changed', server: null, actual: '2' }]);
    expect(second.doc.block(b.id)?.name).toBe('Benim');
    await second.sync.resolve('mine');
    expect(first.server.blockStore.get(b.id)).toMatchObject({ version: 3, block: { name: 'Benim' } });
  });

  it('a command with definitions whose answer was lost goes again with its key after a reload: nothing twice', async () => {
    const drafts = new MemoryDraftStore();
    const first = setup({ drafts });
    const b = def('Kapı');
    first.doc.addBlock(b);
    first.doc.add(insertOf(b.id));
    first.server.loseNextAnswer = true;
    expect(await first.sync.flush()).toBe(false);
    first.sync.dispose();
    const again = await reopen(first.server);
    const second = setup({ drafts, server: first.server, doc: again.doc, records: again.records, blocks: again.blocks });
    await second.sync.restore(await drafts.get('u1/t/p'));
    expect([first.server.commits, first.server.replays, second.doc.blocks.value.length, second.sync.pending.value]).toEqual([1, 1, 1, 0]);
    expect(await second.sync.flush()).toBe(true);
    expect(first.server.commits).toBe(1);
  });

  it('many objects: the definition goes with the first command, its removal with the last', async () => {
    const { doc, server, sync } = setup();
    const commands = sent(server);
    const b = def('Kapı');
    doc.addBlock(b);
    const made = doc.addMany([...Array.from({ length: 2100 }, (_, i) => pt(i)), insertOf(b.id)]);
    expect(await sync.flush()).toBe(true);
    expect(commands.map((c) => [inputOf(c).features.length, inputOf(c).blocks?.map((x) => x.op)])).toEqual([
      [1999, ['create']],
      [102, undefined],
    ]);
    doc.remove(made.map((e) => e.id));
    doc.removeBlock(b.id);
    expect(await sync.flush()).toBe(true);
    expect(commands.slice(2).map((c) => [inputOf(c).features.length, inputOf(c).blocks?.map((x) => x.op)])).toEqual([
      [2000, undefined],
      [101, ['delete']],
    ]);
    expect([server.blockStore.size, server.store.size]).toEqual([0, 0]);
  });

  it('long lists go inner first when made, outer first when removed', () => {
    const inner = def('İç');
    const outer = def('Dış');
    const doc = setup().doc;
    doc.addBlock(outer);
    doc.addBlock(inner);
    doc.updateBlock({ ...outer, entities: [{ ...insertOf(inner.id), id: 1, layerId: '' } as Entity] });
    const tracker = new BlockTracker();
    const made = tracker.ordered(doc, tracker.plan(doc));
    expect(made.upserts.map((p) => p.id)).toEqual([inner.id, outer.id]);
    for (const p of made.upserts) tracker.acknowledge(p, '1');
    const was = tracker.get(outer.id)!;
    // Both removed, the outer as the server last had it (placing the inner).
    doc.removeBlock(outer.id);
    doc.removeBlock(inner.id);
    expect(JSON.parse(was.json).entities).toHaveLength(1);
    expect(tracker.ordered(doc, tracker.plan(doc)).deletes.map((p) => p.id)).toEqual([outer.id, inner.id]);
  });
});
