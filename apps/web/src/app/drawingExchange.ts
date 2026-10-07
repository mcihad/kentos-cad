import { uuidv7 } from '../core/uuid';
import { packDrawing, type PageEntity } from '../io/columns';
import { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import { fileBlock, layerTake, selectionDrawing, takeFrom, TAKEN_SETTINGS, type Json, type Picks, type Same } from '../model/exchange';
import { LayerStore, type LayerInit, type LayerStyle } from '../model/layers';
import { DOCUMENT_EXTENSION, DOCUMENT_MIME, toSnapshotV2 } from '../model/snapshot';
import { BlockInsertTool } from '../tools/blockTools';
import { h } from '../ui/dom';
import { treeLocked } from '../ui/layers/treeRights';
import type { AppContext } from './context';
import { describeDropped, readDrawing } from './drawingFile';
import { writeAccess, writeFailure, writeFile } from './fileAccess';
import type { FileKind } from './fileIO';

/**
 * Çizimler arası alışveriş on the page (docs/adr/0193; the desktop's `exchange.rs`): Seçilenleri dosyaya kaydet writes
 * the selection's drawing as a new `.kcad`; Başka çizimden al and Dosyadan blok ekle read another drawing and put what
 * the rules give into this one (model/exchange.ts): layers and blocks as one undo step, styles, the library, layer
 * states and settings as settings (not undo steps, docs/adr/0092, 0183 §1).
 */

/** The drawings the commands open and write. */
export const DRAWING_FILES: FileKind = { description: 'KentOS çizimi', accept: { [DOCUMENT_MIME]: [DOCUMENT_EXTENSION] } };

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
const stem = (name: string) => name.replace(/\.kcad$/i, '');

/** The open drawing in the contract's JSON form, its objects with their persistent ids. */
export const drawingJson = (ctx: AppContext): Json => toSnapshotV2(ctx.doc);

/** A `.kcad`'s bytes as a drawing in the contract's JSON form, or why not (the caller names the file). */
export async function otherDrawing(ctx: AppContext, bytes: Uint8Array): Promise<{ ok: true; json: Json } | { ok: false; error: string }> {
  const read = await readDrawing(bytes, { codec: ctx.files.kcad, identities: ctx.files.identities });
  if (!read.ok) return read;
  const other = new CadDocument({ name: read.content.name, layers: new LayerStore([], ''), origin: read.content.origin });
  other.replaceWith(read.content);
  return { ok: true, json: toSnapshotV2(other) };
}

/** The coordinate system's words when another drawing's differs from this one's; null when the same. */
export function otherSystem(ctx: AppContext, json: Json): string | null {
  const ours = drawingJson(ctx).settings;
  const same = ours.srid === json.settings.srid && JSON.stringify(ours.customCrs ?? null) === JSON.stringify(json.settings.customCrs ?? null);
  return same ? null : 'Öbür çizimin koordinat sistemi bu çizimdekinden başka; koordinatlar dönüştürülmez.';
}

// ── Seçilenleri dosyaya kaydet ────────────────────────────────────────

/** The selected objects as a new `.kcad` (docs/adr/0193 §1); the open drawing is not touched. */
export async function saveSelection(ctx: AppContext): Promise<boolean> {
  const uids = [...ctx.selection.ids.value].map((id) => ctx.doc.uidOf(id)).filter((u): u is string => u !== undefined);
  if (!uids.length) {
    ctx.log.warn('Seçilenleri dosyaya kaydet: önce kaydedilecek nesneleri seçin.');
    return false;
  }
  const suggested = `${ctx.doc.name.value || 'Çizim'} - seçim${DOCUMENT_EXTENSION}`;
  let handle;
  try {
    handle = await ctx.files.picker.save(suggested);
  } catch (e) {
    ctx.log.error(`Kaydetme penceresi açılamadı: ${message(e)}. Tarayıcının dosya iznini denetleyin.`);
    return false;
  }
  if (handle === null) return false;
  const name = handle ? stem(handle.name) : stem(suggested);
  const drawing = selectionDrawing(drawingJson(ctx), uids, name || 'Seçim');
  const { uids: ids, entities, ...head } = drawing;
  let bytes: Uint8Array;
  let dropped: string | null;
  try {
    const packed = packDrawing(head, (entities as PageEntity[]).map((e, i) => ({ ...e, uid: ids[i] })));
    bytes = await (await ctx.files.kcad()).encode(packed.drawing);
    dropped = describeDropped(packed.dropped);
  } catch (e) {
    ctx.log.error(`Seçilenler yazılamadı: ${message(e)}`);
    return false;
  }
  const said = `Seçilenler kaydedildi: ${entities.length} nesne, ${drawing.blocks?.length ?? 0} blok`;
  if (handle === undefined) {
    // Without file access, a download (nothing confirms it was kept).
    const url = URL.createObjectURL(new Blob([bytes as Uint8Array<ArrayBuffer>], { type: DOCUMENT_MIME }));
    const a = h('a', { href: url, download: suggested });
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    ctx.log.success(`${said}; “${suggested}” indirme olarak verildi.`);
    return true;
  }
  const denied = await writeAccess(handle);
  if (denied) {
    ctx.log.error(denied);
    return false;
  }
  try {
    await writeFile(handle, bytes);
  } catch (e) {
    ctx.log.error(writeFailure(handle.name, e));
    return false;
  }
  if (dropped) ctx.log.warn(`“${handle.name}”: KCAD v2'nin tanımadığı alanlar yazılmadı: ${dropped}.`);
  ctx.log.success(`${said} → “${handle.name}”. Açık çizim değişmedi.`);
  return true;
}

// ── What the rules give, into the open drawing ────────────────────────

/** The style `next` as a patch over `before`: its fields, and the ones it has not taken away. */
function stylePatch(before: LayerStyle, next: Json): Partial<LayerStyle> {
  const patch: Record<string, unknown> = { ...next };
  for (const k of Object.keys(before)) if (!(k in next)) patch[k] = undefined;
  return patch as Partial<LayerStyle>;
}

/** Whether the style `next` differs from `before`, as `setLayerStyle` compares them. */
function restyles(before: LayerStyle, next: Json): boolean {
  const after: Record<string, unknown> = { ...before, ...stylePatch(before, next) };
  for (const k of Object.keys(after)) if (after[k] === undefined) delete after[k];
  return JSON.stringify(before) !== JSON.stringify(after);
}

/** Whether `nodes` would add a node to the drawing's tree or change a node's look. */
function changesTree(ctx: AppContext, nodes: readonly Json[]): boolean {
  return nodes.some((n) => {
    const mine = ctx.doc.layers.get(n.id);
    return !mine || restyles(mine.style, n.style) || changesTree(ctx, n.children ?? []);
  });
}

/** The layers and blocks of `result` that this drawing has not, and their changes, as part of the open transaction. */
function putLayersAndBlocks(ctx: AppContext, result: Json, blockIds: ReadonlyMap<string, string> = new Map()): { layers: number; blocks: number; redefined: number } {
  const doc = ctx.doc;
  let layers = 0;
  const visit = (nodes: readonly Json[], parent: string | null) => {
    for (const n of nodes) {
      const mine = doc.layers.get(n.id);
      if (!mine) {
        doc.addLayer(n as LayerInit, parent);
        layers += 1 + countNodes(n.children ?? []);
        continue;
      }
      if (restyles(mine.style, n.style)) doc.setLayerStyle(n.id, stylePatch(mine.style, n.style), 'Başka çizimden al');
      visit(n.children ?? [], n.id);
    }
  };
  visit(result.layers ?? [], null);
  // New blocks, and met ones redefined, once every block they place is there: a definition may place only blocks the
  // drawing has (docs/adr/0144).
  const pending = (result.blocks ?? []).map((b: Json) => ({ ...b, id: blockIds.get(b.id) ?? b.id, entities: b.entities.map((e: Json) => (e.kind === 'insert' ? { ...e, block: blockIds.get(e.block) ?? e.block } : e)) }));
  let blocks = 0;
  let redefined = 0;
  for (let guard = pending.length + 1; pending.length && guard; guard--)
    for (let i = 0; i < pending.length; i++) {
      const b = pending[i];
      const placed = b.entities.filter((e: Json) => e.kind === 'insert').map((e: Json) => e.block as string);
      if (!placed.every((id: string) => doc.block(id) || id === b.id)) continue;
      pending.splice(i--, 1);
      if (doc.block(b.id)) {
        if (doc.updateBlock(b)) redefined++;
      } else {
        doc.addBlock(b);
        blocks++;
      }
    }
  return { layers, blocks, redefined };
}

const countNodes = (nodes: readonly Json[]): number => nodes.reduce((n, c) => n + 1 + countNodes(c.children ?? []), 0);

/** The settings `result` holds, as settings (not an undo step): styles, layer states and, with `units`, the units and scale. */
function putSettings(ctx: AppContext, result: Json, units: boolean): void {
  const s = result.settings;
  const patch: Record<string, unknown> = { textStyles: s.textStyles ?? [], dimensionStyles: s.dimensionStyles ?? [], layerStates: s.layerStates ?? [] };
  if (units)
    for (const key of TAKEN_SETTINGS) {
      if (key in s) patch[key] = s[key];
      else if (key === 'survey') patch.survey = null;
      else if (key === 'drawingUnit') patch.drawingUnit = 'm';
    }
  ctx.doc.settings.assign(patch);
  ctx.doc.styles.set(result.styles);
}

/** Başka çizimden al's `picks` of `theirs` into the open drawing (docs/adr/0193 §2); the counts in words, or why not. */
export function takeInto(ctx: AppContext, theirs: Json, picks: Picks, same: Same, file: string): boolean {
  const result = takeFrom(drawingJson(ctx), theirs, picks, same);
  const locked = treeLocked(ctx);
  if (locked && changesTree(ctx, result.layers ?? [])) {
    ctx.log.warn(`Başka çizimden al: ${locked}`);
    return false;
  }
  let counts = { layers: 0, blocks: 0, redefined: 0 };
  try {
    ctx.doc.transact('Başka çizimden al', () => {
      counts = putLayersAndBlocks(ctx, result);
    });
  } catch (e) {
    ctx.log.warn(`Başka çizimden al: ${message(e)} Hiçbir şey alınmadı.`);
    return false;
  }
  putSettings(ctx, result, !!picks.settings);
  const parts = [
    counts.layers ? `${counts.layers} katman ya da grup` : '',
    counts.blocks ? `${counts.blocks} blok` : '',
    counts.redefined ? `${counts.redefined} blok yeniden tanımlandı` : '',
  ].filter(Boolean);
  const settings = [picks.textStyles?.length || picks.dimensionStyles?.length ? 'stiller' : '', picks.library?.length ? 'kitaplık' : '', picks.layerStates?.length ? 'katman durumları' : '', picks.settings ? 'proje ayarları' : ''].filter(Boolean);
  ctx.log.success(
    `“${file}” çiziminden alındı${parts.length ? `: ${parts.join(', ')} (tek adımda geri alınır)` : ''}${settings.length ? `; ${settings.join(', ')} ayar olarak (geri alma adımı değil)` : ''}.`,
  );
  return true;
}

/** Dosyadan blok ekle (docs/adr/0193 §3): `theirs` as one block of the open drawing, then Blok ekle with it. */
export function insertFileBlock(ctx: AppContext, theirs: Json, file: string): boolean {
  const made = fileBlock(drawingJson(ctx), theirs, stem(file));
  if (!made) {
    ctx.log.warn(`Dosyadan blok ekle: “${stem(file)}” içinde blok yapılacak nesne yok; resimler ve tablolar bloğa konamaz.`);
    return false;
  }
  const locked = treeLocked(ctx);
  if (locked && changesTree(ctx, made.drawing.layers ?? [])) {
    ctx.log.warn(`Dosyadan blok ekle: ${locked}`);
    return false;
  }
  const id = uuidv7();
  const block = made.drawing.blocks[made.drawing.blocks.length - 1];
  const before = new Set(ctx.doc.blocks.value.map((b) => b.id));
  let counts = { layers: 0, blocks: 0, redefined: 0 };
  try {
    ctx.doc.transact('Dosyadan blok ekle', () => {
      counts = putLayersAndBlocks(ctx, made.drawing, new Map([['$new', id]]));
    });
  } catch (e) {
    ctx.log.warn(`Dosyadan blok ekle: ${message(e)} Hiçbir şey eklenmedi.`);
    return false;
  }
  putSettings(ctx, made.drawing, false);
  const nested = counts.blocks - 1;
  const left = [made.images ? `${made.images} resim` : '', made.tables ? `${made.tables} tablo` : ''].filter(Boolean);
  ctx.log.success(
    `“${file}” blok oldu: “${block.name}”, ${block.entities.length} nesne${nested > 0 ? `, ${nested} iç blok` : ''}${counts.layers ? `, ${counts.layers} yeni katman` : ''}${left.length ? `; ${left.join(' ve ')} bloğa konamadığı için alınmadı` : ''}. Yerleştirme noktasına tıklayın.`,
  );
  if (!before.has(id)) BlockInsertTool.block = id;
  void ctx.commands.execute('tool.blockInsert');
  return true;
}

/**
 * Kaynaklar's Katman olarak ekle (docs/adr/0199 §7): `theirs`'s layer at `path` with its objects into the open drawing
 * (model/exchange.ts `layerTake`): the layers, blocks and objects as one undo step, the styles and library items as
 * settings; `from` names where it came from. The objects take new ids; a layer of ours that is locked takes none.
 */
export function takeLayerInto(ctx: AppContext, theirs: Json, path: string, from: string): boolean {
  const made = layerTake(drawingJson(ctx), theirs, path);
  if (!made) {
    ctx.log.warn(`Katman olarak ekle: “${path}” alınamadı: kaynakta böyle bir katman yok ya da bu çizimde aynı yolda katman olmayan bir düğüm var.`);
    return false;
  }
  const locked = treeLocked(ctx);
  if (locked && changesTree(ctx, made.drawing.layers ?? [])) {
    ctx.log.warn(`Katman olarak ekle: ${locked}`);
    return false;
  }
  const shut = [...new Set(made.objects.map((e) => e.layerId as string))].find((id) => ctx.doc.layers.get(id) && ctx.doc.layers.isLocked(id));
  if (shut !== undefined) {
    ctx.log.warn(`Katman olarak ekle: “${ctx.doc.layers.path(shut)}” katmanı kilitli; kilidini açıp yeniden deneyin.`);
    return false;
  }
  let counts = { layers: 0, blocks: 0, redefined: 0 };
  try {
    ctx.doc.transact('Katman olarak ekle', () => {
      counts = putLayersAndBlocks(ctx, made.drawing);
      if (made.objects.length) ctx.doc.addMany(made.objects.map(({ id: _id, ...e }) => e as NewEntity), 'Katman olarak ekle');
    });
  } catch (e) {
    ctx.log.warn(`Katman olarak ekle: ${message(e)} Hiçbir şey alınmadı.`);
    return false;
  }
  putSettings(ctx, made.drawing, false);
  const parts = [`${made.objects.length} nesne`, counts.layers ? `${counts.layers} yeni katman ya da grup` : '', counts.blocks ? `${counts.blocks} blok` : ''].filter(Boolean);
  ctx.log.success(`“${from}” içinden “${path}” alındı: ${parts.join(', ')} (tek adımda geri alınır).`);
  return true;
}

/** Asks for a drawing file and reads it; null (said) when cancelled or unreadable. */
export async function pickDrawing(ctx: AppContext, title: string): Promise<{ name: string; json: Json } | null> {
  const picked = await ctx.files.pickForImport(DRAWING_FILES);
  if (!picked) return null;
  const read = await otherDrawing(ctx, picked.bytes);
  if (!read.ok) {
    ctx.log.warn(`${title}: “${picked.name}” okunamadı: ${read.error}`);
    return null;
  }
  const system = otherSystem(ctx, read.json);
  if (system) ctx.log.warn(`${title}: ${system}`);
  return { name: picked.name, json: read.json };
}

/** Seçilenleri dosyaya kaydet, Başka çizimden al and Dosyadan blok ekle (menus, the ribbon, the command line). */
export function registerExchangeCommands(ctx: AppContext): void {
  const busy = () => ctx.files.busy.value;
  ctx.commands.register({
    id: 'file.saveSelection',
    title: 'Seçilenleri dosyaya kaydet…',
    short: 'Seçilenleri kaydet',
    category: 'Dosya',
    icon: 'saveSelection',
    aliases: ['SECILENLERIKAYDET', 'WBLOCK', 'BLOKYAZ', 'SECIMKAYDET'],
    description: 'Seçili nesneleri, kullandıkları katmanlar, bloklar, kitaplık öğeleri ve projenin ayarlarıyla yeni bir .kcad dosyasına yazar; açık çizim değişmez.',
    isEnabled: () => ctx.selection.ids.value.size > 0 && !busy(),
    whyDisabled: () => (ctx.selection.ids.value.size ? null : 'Önce kaydedilecek nesneleri seçin.'),
    watch: [ctx.selection.ids, ctx.files.busy],
    run: () => void saveSelection(ctx),
  });
  ctx.commands.register({
    id: 'file.takeFrom',
    title: 'Başka çizimden al…',
    short: 'Başka çizimden al',
    category: 'Dosya',
    icon: 'takeFrom',
    aliases: ['BASKACIZIMDENAL', 'ADCENTER', 'OZELLIKEKLE', 'CIZIMDENAL'],
    description: 'Bir .kcad dosyasının katmanlarını, bloklarını, yazı ve ölçü stillerini, kitaplık öğelerini, katman durumlarını ve proje ayarlarını bu çizime alır; aynı adlı olanlar atlanır ya da değiştirilir.',
    isEnabled: () => !busy(),
    watch: [ctx.files.busy],
    run: () => void import('../ui/io/TakeFromDialog').then((m) => m.openTakeFrom(ctx)),
  });
  ctx.commands.register({
    id: 'block.insertFile',
    title: 'Dosyadan blok ekle…',
    short: 'Dosyadan blok',
    category: 'Blok',
    icon: 'blockInsertFile',
    aliases: ['DOSYADANBLOK', 'CIZIMIBLOKEKLE', 'INSERTFILE'],
    description: 'Bir .kcad dosyasının nesnelerini tek blok yapar (katmanları, blokları ve stilleriyle) ve Blok ekle ile yerleştirir; taban noktası nesnelerin sol alt köşesidir.',
    isEnabled: () => !busy(),
    watch: [ctx.files.busy],
    run: () =>
      void pickDrawing(ctx, 'Dosyadan blok ekle').then((got) => {
        if (got) insertFileBlock(ctx, got.json, got.name);
      }),
  });
}
