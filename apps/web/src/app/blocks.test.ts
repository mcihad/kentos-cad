import { describe, expect, it } from 'vitest';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { blocksDefine } from '../product/blocksDefine';
import type { Tool } from '../tools/Tool';
import { BlockInsertTool } from '../tools/blockTools';
import { toolHarness } from '../tools/toolHarness';
import { insertBlock, purgeBlocks, rebaseBlock, rebaseInsert, redefineBlock, removeBlock, removeRefusal, renameBlock, selectInserts } from './blocks';

/**
 * What the Bloklar panel and the block commands do (docs/adr/0144 §6), in
 * the words the desktop says (apps/desktop/src/blocks_panel.rs tests): a
 * drawing with a line and a circle made into “Vana” from (10, 20), put in
 * their place unless asked not to, and one more object.
 */
function withBlock(replace = true) {
  const t = toolHarness();
  const line = t.add({ kind: 'line', a: { x: 10, y: 20 }, b: { x: 12, y: 20 } });
  const circle = t.add({ kind: 'circle', c: { x: 11, y: 20 }, r: 0.5 });
  const other = t.add({ kind: 'point', p: { x: 30, y: 30 } });
  const made = blocksDefine.execute(
    { doc: t.doc },
    { name: 'Vana', base: { x: 10, y: 20 }, uids: [line.uid!, circle.uid!], ...(replace && { replace: true, layerId: 'cizim' }) },
  );
  if (made.status !== 'completed') throw new Error('Vana tanımlanamadı');
  const id = made.output.block;
  // The tool the actions run (a point asked for), and where a point on an insert falls in its block: unscaled, unturned.
  let running: Tool | null = null;
  const ctx = t.ctx as unknown as Record<string, unknown>;
  ctx.tools = { run: (tool: Tool) => ((running = tool), tool.activate?.()), exit: () => (running = null) };
  ctx.commands = { execute: (cmd: string) => t.log.info(`çalıştı: ${cmd}`) };
  (t.ctx.view as unknown as Record<string, unknown>).insertLocal = (slot: number, p: Vec2) => {
    const e = t.doc.get(slot) as Extract<Entity, { kind: 'insert' }> | undefined;
    const base = t.doc.block(id)?.base;
    return e?.kind === 'insert' && base ? { x: base.x + p.x - e.p.x, y: base.y + p.y - e.p.y } : null;
  };
  const insert = [...t.doc.all()].find((e) => e.kind === 'insert');
  const pick = (p: Vec2) => (running as unknown as { acceptPoint(p: Vec2): boolean } | null)?.acceptPoint(p);
  return { ...t, id, other, insert, pick, running: () => running };
}

const last = (said: () => string[]) => said().at(-1);

describe('the block actions (docs/adr/0144 §6)', () => {
  it('purge the unused blocks, saying how many', () => {
    const t = withBlock();
    // A second block, its object kept: no insert places it.
    blocksDefine.execute({ doc: t.doc }, { name: 'Blok 1', base: { x: 0, y: 0 }, uids: [t.other.uid!] });
    purgeBlocks(t.ctx);
    expect(t.doc.blocks.value.map((b) => b.name)).toEqual(['Vana']);
    expect(last(t.said)).toBe('1 kullanılmayan blok silindi.');
    purgeBlocks(t.ctx);
    expect(last(t.said)).toBe('Kullanılmayan blok yok.');
  });

  it('rename a block, and refuse a taken name in the command’s words', () => {
    const t = withBlock();
    expect(renameBlock(t.ctx, t.id, 'Su vanası')).toBe(true);
    expect(last(t.said)).toBe('“Vana” bloğunun adı “Su vanası” oldu.');
    blocksDefine.execute({ doc: t.doc }, { name: 'Rögar', base: { x: 0, y: 0 }, uids: [t.other.uid!] });
    expect(renameBlock(t.ctx, t.id, 'RÖGAR')).toBe(false);
    expect(t.doc.block(t.id)?.name).toBe('Su vanası');
  });

  it('keep a placed block, delete an unused one', () => {
    const t = withBlock();
    expect(removeRefusal(t.ctx, t.id)).toBe('“Vana” bloğu kullanılıyor (çizimde 1 yerleştirmesi); önce onları silin ya da patlatın.');
    const kept = withBlock(false);
    expect(removeRefusal(kept.ctx, kept.id)).toBeNull();
    removeBlock(kept.ctx, kept.id);
    expect(kept.doc.blocks.value).toEqual([]);
    expect(last(kept.said)).toBe('“Vana” bloğu silindi.');
  });

  it('select a block’s inserts, and place it with Blok ekle', () => {
    const t = withBlock();
    selectInserts(t.ctx, t.id);
    expect([...t.ctx.selection.ids.value]).toEqual([t.insert!.id]);
    expect(last(t.said)).toBe('“Vana” bloğunun 1 yerleştirmesi seçildi.');
    insertBlock(t.ctx, t.id);
    expect(BlockInsertTool.block).toBe(t.id);
    expect(last(t.said)).toBe('çalıştı: tool.blockInsert');
    BlockInsertTool.block = null;
  });

  it('show a new base point on the insert, and need an insert to show it on', () => {
    const t = withBlock();
    expect(rebaseInsert(t.ctx, t.id)).toEqual({ insert: t.insert!.id });
    rebaseBlock(t.ctx, t.id);
    expect(t.running()?.prompt.value).toBe('“Vana” için yeni taban noktası: haritada bir nokta gösterin ya da Y,X yazın [Vazgeç (Esc)]');
    t.pick({ x: 12, y: 21 });
    expect(t.doc.block(t.id)?.base).toEqual({ x: 12, y: 21 });
    expect(last(t.said)).toBe('“Vana” bloğunun taban noktası değişti.');
    const none = withBlock(false);
    rebaseBlock(none.ctx, none.id);
    expect(none.running()).toBeNull();
    expect(last(none.said)).toBe('“Vana” bloğu çizimde yerleştirilmemiş: yeni taban noktası bir yerleştirmesinde gösterilir. Önce Blok ekle ile yerleştirin.');
  });

  it('redefine a block from the selection and the base point shown', () => {
    const t = withBlock();
    redefineBlock(t.ctx, t.id);
    expect(last(t.said)).toBe('“Vana” bloğunun yeni nesnelerini önce seçin.');
    t.ctx.selection.set([t.other.id]);
    redefineBlock(t.ctx, t.id);
    t.pick({ x: 0, y: 0 });
    expect(t.doc.block(t.id)?.entities.length).toBe(1);
    expect(last(t.said)).toBe('“Vana” bloğu 1 nesneyle yeniden tanımlandı.');
  });
});
