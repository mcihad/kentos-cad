import type { Vec2 } from '../model/geometry';
import { placements } from '../model/blocks';
import { blocksEdit } from '../product/blocksEdit';
import { BlockInsertTool } from '../tools/blockTools';
import { PickPointTool } from '../tools/pickPointTool';
import type { BlocksEdit } from '../contracts/generated/BlocksEdit';
import type { AppContext } from './context';

/**
 * The blocks in the app (docs/adr/0144 §6): the window that names a block
 * Blok oluştur has picked, what the session keeps for it, and what the
 * Bloklar panel and the block commands do. The window is loaded when first
 * opened (ui/blocks/BlockDefineDialog.ts). Every change goes through
 * `cad.blocks.edit`, one undo step, its refusal in its own words; the
 * desktop's `apps/desktop/src/blocks_panel.rs` does the same, in the same
 * words.
 */
export interface BlockService {
  /** Opens the window that names a new block of the objects `uids` with the base point `base`. */
  define(base: Vec2, uids: readonly string[]): void;
  /** Opens the window of a block's attribute definitions (docs/adr/0144 §7). */
  attributes(id: string): void;
  /**
   * Blok ekle's question for a block with attribute definitions (§7): `done`
   * gets the values to write (those not empty and not the default), or null
   * when the user leaves it.
   */
  values(id: string, done: (values: Record<string, string> | null) => void): void;
  /** “Seçilenleri blokla değiştir”: the window's last choice, for as long as the app lives. */
  replace: boolean;
}

/** The service; `open` and `attributes` load and open the windows. */
export function createBlocks(
  open: (base: Vec2, uids: readonly string[]) => void,
  attributes: (id: string) => void,
  values: (id: string, done: (values: Record<string, string> | null) => void) => void,
): BlockService {
  return { define: open, attributes, values, replace: true };
}

const nameOf = (ctx: AppContext, id: string): string => ctx.doc.block(id)?.name ?? '';

/** Runs `cad.blocks.edit`; its warnings and refusal go to the log. Whether it completed, and its output. */
function edit(ctx: AppContext, input: BlocksEdit) {
  const result = blocksEdit.execute({ doc: ctx.doc }, input);
  if (result.status !== 'completed') {
    if ('error' in result) ctx.log.warn(result.error.message);
    return null;
  }
  for (const w of result.warnings) ctx.log.warn(w.message);
  return result.output;
}

/** Blokları temizle: every definition no insert uses, in one step. */
export function purgeBlocks(ctx: AppContext): void {
  const out = edit(ctx, { operation: 'purge' });
  if (!out) return;
  if (out.removed.length) ctx.log.success(`${out.removed.length} kullanılmayan blok silindi.`);
  else ctx.log.info('Kullanılmayan blok yok.');
}

/** A new name for a block; the old one stays when it is refused. */
export function renameBlock(ctx: AppContext, id: string, name: string): boolean {
  const was = nameOf(ctx, id);
  const out = edit(ctx, { operation: 'rename', block: id, name });
  if (out?.changed.length) ctx.log.success(`“${was}” bloğunun adı “${name}” oldu.`);
  return !!out;
}

/** Deletes a block no insert uses (the command refuses one in use, in its words). */
export function removeBlock(ctx: AppContext, id: string): void {
  const name = nameOf(ctx, id);
  if (edit(ctx, { operation: 'remove', block: id })) ctx.log.success(`“${name}” bloğu silindi.`);
}

/** The drawing's inserts of a block, in the drawing's order. */
export function insertsOf(ctx: AppContext, id: string): number[] {
  const out: number[] = [];
  for (const e of ctx.doc.all()) if (e.kind === 'insert' && e.block === id) out.push(e.id);
  return out;
}

/** Selects the drawing's inserts of a block. */
export function selectInserts(ctx: AppContext, id: string): void {
  const ids = insertsOf(ctx, id);
  const name = nameOf(ctx, id);
  if (!ids.length) return ctx.log.info(`“${name}” bloğunun çizimde yerleştirmesi yok.`);
  ctx.selection.set(ids);
  ctx.log.info(`“${name}” bloğunun ${ids.length} yerleştirmesi seçildi.`);
}

/** Blok ekle with this block: it is the one placed. */
export function insertBlock(ctx: AppContext, id: string): void {
  BlockInsertTool.block = id;
  ctx.commands.execute('tool.blockInsert');
}

/** The insert of a block a point is shown on: its selected one, or its only one; else none or many. */
function shownOn(ctx: AppContext, id: string): number | 'none' | 'many' {
  const all = insertsOf(ctx, id);
  const chosen = all.filter((i) => ctx.selection.has(i));
  if (chosen.length === 1) return chosen[0];
  if (all.length === 1) return all[0];
  return all.length ? 'many' : 'none';
}

/**
 * The insert a new base point is shown on: the selected insert of the block,
 * or its only one in the drawing; else why there is none to show it on.
 */
export function rebaseInsert(ctx: AppContext, id: string): { insert: number } | { why: string } {
  const name = nameOf(ctx, id);
  const on = shownOn(ctx, id);
  if (typeof on === 'number') return { insert: on };
  if (on === 'none') return { why: `“${name}” bloğu çizimde yerleştirilmemiş: yeni taban noktası bir yerleştirmesinde gösterilir. Önce Blok ekle ile yerleştirin.` };
  return { why: `“${name}” bloğunun birden çok yerleştirmesi var: yeni taban noktasını göstereceğiniz yerleştirmeyi seçin.` };
}

/** The insert an attribute's place is shown on (Blok öznitelikleri), as the base point's; else why none. */
export function attributesInsert(ctx: AppContext, id: string): { insert: number } | { why: string } {
  const name = nameOf(ctx, id);
  const on = shownOn(ctx, id);
  if (typeof on === 'number') return { insert: on };
  if (on === 'none') return { why: `“${name}” bloğu çizimde yerleştirilmemiş: özniteliğin yeri bir yerleştirmesinde gösterilir. Yeri yazın ya da önce Blok ekle ile yerleştirin.` };
  return { why: `“${name}” bloğunun birden çok yerleştirmesi var: yeri göstereceğiniz yerleştirmeyi pencereyi açmadan önce seçin ya da yeri yazın.` };
}

/**
 * Taban noktasını değiştir: the new base point is shown on an insert of the
 * block (the selected one, or its only one) and taken back into the
 * definition (`view.insertLocal`); every insert then places the block from it.
 */
export function rebaseBlock(ctx: AppContext, id: string): void {
  const via = rebaseInsert(ctx, id);
  if ('why' in via) return ctx.log.warn(via.why);
  const name = nameOf(ctx, id);
  const done = (p: Vec2 | null) => {
    const base = p && ctx.view.insertLocal(via.insert, p);
    if (!base) return;
    if (edit(ctx, { operation: 'rebase', block: id, base })?.changed.length) ctx.log.success(`“${name}” bloğunun taban noktası değişti.`);
  };
  ctx.tools.run(new PickPointTool(ctx, `“${name}” için yeni taban noktası`, done), `Taban noktası: ${name}`);
}

/** The selected objects' persistent ids. */
function selectedUids(ctx: AppContext): string[] {
  return [...ctx.selection.ids.value].map((i) => ctx.doc.uidOf(i)).filter((u): u is string => u !== undefined);
}

/**
 * Seçili nesnelerle yeniden tanımla: the block is made of the selected
 * objects, from the base point shown now; the objects stay where they are.
 */
export function redefineBlock(ctx: AppContext, id: string): void {
  const uids = selectedUids(ctx);
  const name = nameOf(ctx, id);
  if (!uids.length) return ctx.log.warn(`“${name}” bloğunun yeni nesnelerini önce seçin.`);
  const done = (p: Vec2 | null) => {
    if (!p) return;
    const out = edit(ctx, { operation: 'redefine', block: id, uids, base: { x: p.x, y: p.y } });
    if (out?.changed.length) ctx.log.success(`“${name}” bloğu ${uids.length} nesneyle yeniden tanımlandı.`);
  };
  ctx.tools.run(new PickPointTool(ctx, `“${name}” için taban noktası`, done), `Yeniden tanımla: ${name}`);
}

/** Why a block cannot be deleted now (an insert places it), or null. */
export function removeRefusal(ctx: AppContext, id: string): string | null {
  const blocks = ctx.doc.blocks.value;
  const at = blocks.findIndex((b) => b.id === id);
  const p = at < 0 ? null : placements(blocks, ctx.doc.all())[at];
  if (!p || p.drawing + p.nested === 0) return null;
  const where = [p.drawing ? `çizimde ${p.drawing} yerleştirmesi` : '', p.nested ? `${p.nested} bloğun içinde yerleştirmesi` : ''].filter(Boolean).join(', ');
  return `“${nameOf(ctx, id)}” bloğu kullanılıyor (${where}); önce onları silin ya da patlatın.`;
}

/** The one block the selection's inserts place, or null (none, or inserts of several). */
export function selectedBlock(ctx: AppContext): string | null {
  const blocks = new Set<string>();
  for (const i of ctx.selection.ids.value) {
    const e = ctx.doc.get(i);
    if (e?.kind === 'insert') blocks.add(e.block);
  }
  return blocks.size === 1 ? [...blocks][0] : null;
}

/** The block commands: the Bloklar panel, Blok öznitelikleri and Blokları temizle (menus, the ribbon's Blok group, the command line). */
export function registerBlockCommands(ctx: AppContext): void {
  const cat = 'Blok';
  ctx.commands.register({
    id: 'block.panel',
    title: 'Bloklar',
    category: cat,
    icon: 'blocks',
    aliases: ['BLOKLAR', 'BLOCKS', 'BLOKLISTESI'],
    description: 'Çizimin bloklarını önizlemeleri ve yerleştirme sayılarıyla gösteren paneli açar: ekle, yeniden adlandır, taban noktası, yeniden tanımla, sil.',
    run: () => {
      ctx.ui.rightVisible.set(true);
      ctx.ui.dockTab.set('blocks');
    },
  });
  ctx.commands.register({
    id: 'block.attributes',
    title: 'Blok öznitelikleri',
    category: cat,
    icon: 'blockAttributes',
    aliases: ['BATTMAN', 'ATTDEF', 'OZNITELIKTANIMI'],
    description: 'Seçili yerleştirmenin bloğunun öznitelik tanımlarını düzenleyen pencereyi açar: etiket, soru, varsayılan değer, yazının yüksekliği, açısı ve yeri. Bloklar panelinde bir bloğun menüsünden de açılır.',
    isEnabled: () => selectedBlock(ctx) !== null,
    whyDisabled: () => (selectedBlock(ctx) ? null : 'Önce bir bloğun yerleştirmesini seçin (Bloklar panelinde bloğun menüsünden de açılır).'),
    watch: [ctx.selection.ids, ctx.doc.blocks],
    run: () => {
      const id = selectedBlock(ctx);
      if (id) ctx.blocks.attributes(id);
    },
  });
  ctx.commands.register({
    id: 'block.purge',
    title: 'Blokları temizle',
    category: cat,
    icon: 'cleanup',
    aliases: ['PURGE', 'BLOKTEMIZLE'],
    description: 'Hiçbir yerleştirmesi olmayan blok tanımlarını tek adımda siler.',
    isEnabled: () => ctx.doc.blocks.value.length > 0,
    whyDisabled: () => (ctx.doc.blocks.value.length ? null : 'Çizimde blok yok.'),
    watch: [ctx.doc.blocks],
    run: () => purgeBlocks(ctx),
  });
}
