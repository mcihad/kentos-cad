import type { AssetHref } from '../../contracts/generated/sheet/AssetHref';
import type { AssetWithBytes } from '../../contracts/generated/sheet/AssetWithBytes';
import type { DisplayList } from '../../contracts/generated/sheet/DisplayList';
import type { MapImage } from '../../contracts/generated/sheet/MapImage';
import type { MapPrim } from '../../contracts/generated/sheet/MapPrim';
import type { Op } from '../../contracts/generated/sheet/Op';
import type { SheetBook } from '../../contracts/generated/sheet/SheetBook';
import { errorText, type BookText, type SheetEngine } from '../../product/sheet/engine';
import { fromBase64, toBase64 } from '../../product/sheet/store';
import { fontsOf, paintList } from '../../render/sheet/painter';
import { saveExport } from '../../ui/io/save';
import { confirmDialog } from '../../ui/widgets/confirm';
import type { FileKind } from '../fileIO';
import type { AppContext } from '../context';
import type { SheetService } from './service';

/**
 * A sheet out of the program and a book into it (docs/sheet/design.md §9, §10):
 *
 * - SVG is the engine's writer (`toSvg`); each map frame's content goes in as a
 *   PNG at the export's dpi (drawn by the render pipeline, mapFrames.ts), the
 *   pictures and the legends' symbols as data URLs.
 * - PNG is the same plan painted on a canvas of the paper's size at the dpi,
 *   with every width as it is (no hairline is thickened).
 * - `.kpafta` is the book and the pictures' bytes, written and read by the
 *   core's one codec (`encodeKpafta` / `decodeKpafta`, the desktop's too):
 *   no format of its own here; an import adds the sheets with new ids, or
 *   puts them in the project's place.
 *
 * Nothing here decides what is drawn: the engine's plan (export mode, only
 * what prints) is the picture.
 */

const SVG: FileKind = { description: 'SVG resmi', accept: { 'image/svg+xml': ['.svg'] } };
const PNG: FileKind = { description: 'PNG resmi', accept: { 'image/png': ['.png'] } };
export const KPAFTA: FileKind = { description: 'KentOS pafta dosyası', accept: { 'application/json': ['.kpafta'] } };

/** The largest PNG drawn: a side and an area a browser's canvas reliably takes. */
export const PNG_MAX_SIDE = 16_384;
export const PNG_MAX_PIXELS = 120_000_000;

/** The pixels a sheet makes at a dpi. */
export function pngSize(book: SheetBook, sheetId: string, dpi: number): { width: number; height: number } | null {
  const s = book.sheets.find((x) => x.id === sheetId);
  if (!s) return null;
  return { width: Math.round((s.page.size.width / 25_400) * dpi), height: Math.round((s.page.size.height / 25_400) * dpi) };
}

/** Why a PNG of that size cannot be drawn; null when it can. */
export function pngTooLarge(size: { width: number; height: number }): string | null {
  if (size.width > PNG_MAX_SIDE || size.height > PNG_MAX_SIDE || size.width * size.height > PNG_MAX_PIXELS)
    return `${size.width} × ${size.height} piksel tarayıcının çizebileceğinden büyük. Daha düşük bir dpi seçin ya da SVG olarak aktarın.`;
  return null;
}

/** A file name from the sheet's: letters, digits, spaces and dashes kept. */
export function fileName(name: string, ext: string): string {
  const base = name.replace(/[\\/:*?"<>|]+/g, ' ').replace(/\s+/g, ' ').trim() || 'pafta';
  return `${base}${ext}`;
}

function plan(engine: SheetEngine, s: SheetService, book: BookText, sheetId: string, dpi: number): DisplayList {
  const sheet = book.book.sheets.find((x) => x.id === sheetId);
  if (!sheet) throw new Error('Pafta kitapta yok.');
  return engine.displayList(book, sheetId, s.paint.inputs(engine, book, sheet, 'export', { dpi }));
}

/** Each map frame's content at the dpi, by item (null where a map has no place or WebGL2 is not there). */
async function mapPictures(s: SheetService, list: DisplayList, dpi: number): Promise<Map<string, HTMLCanvasElement>> {
  const out = new Map<string, HTMLCanvasElement>();
  for (const p of list.prims) {
    if (p.type !== 'map') continue;
    const c = await s.paint.maps.exportPicture(p as MapPrim, dpi / 25_400);
    if (c) out.set(p.item, c);
  }
  return out;
}

const MIME: Record<string, string> = { png: 'image/png', jpeg: 'image/jpeg', svg: 'image/svg+xml' };

/** The book's pictures this device has, as data URLs. */
async function assetHrefs(s: SheetService, book: SheetBook): Promise<AssetHref[]> {
  const out: AssetHref[] = [];
  for (const meta of book.assets) {
    const rec = await s.assets.get(meta.sha256);
    if (rec) out.push({ sha256: meta.sha256, href: `data:${MIME[meta.kind] ?? 'application/octet-stream'};base64,${toBase64(rec.bytes)}` });
  }
  return out;
}

export async function exportSvg(ctx: AppContext, s: SheetService, sheetId: string, dpi: number): Promise<string | null> {
  const engine = await s.ensureEngine();
  const book = s.book();
  if (!book) return null;
  try {
    const list = plan(engine, s, book, sheetId, dpi);
    const maps: MapImage[] = [...(await mapPictures(s, list, dpi))].map(([item, c]) => ({ item, href: c.toDataURL('image/png') }));
    const assets = await assetHrefs(s, book.book);
    for (const p of list.prims) if (p.type === 'image' && p.asset.startsWith('lejant:') && !assets.some((a) => a.sha256 === p.asset)) {
      const href = s.paint.symbolHref(p.asset);
      if (href) assets.push({ sha256: p.asset, href });
    }
    const name = book.book.sheets.find((x) => x.id === sheetId)!.name;
    const svg = engine.toSvg(list, { maps, assets, placeholders: false, title: name });
    return await saveExport(ctx, new TextEncoder().encode(svg), fileName(name, '.svg'), SVG);
  } catch (e) {
    s.report('SVG olarak aktarılamadı', e);
    return null;
  }
}

/** Paints a sheet at a dpi on a canvas of the paper's size: the PNG's picture (also the tests'). */
export async function paintPng(s: SheetService, engine: SheetEngine, book: BookText, sheetId: string, dpi: number): Promise<HTMLCanvasElement> {
  const size = pngSize(book.book, sheetId, dpi)!;
  const tooLarge = pngTooLarge(size);
  if (tooLarge) throw new Error(tooLarge);
  const list = plan(engine, s, book, sheetId, dpi);
  const maps = await mapPictures(s, list, dpi);
  // Pictures decoded and typefaces loaded before the canvas is painted once.
  const wanted = list.prims.filter((p) => p.type === 'image').map((p) => (p as { asset: string }).asset);
  for (let i = 0; i < 40 && wanted.some((a) => !s.paint.image(a)); i++) await new Promise((r) => setTimeout(r, 25));
  if (document.fonts) await Promise.all(fontsOf(list, (f) => s.paint.family(f)).map((f) => document.fonts.load(f).catch(() => [])));
  const canvas = document.createElement('canvas');
  canvas.width = size.width;
  canvas.height = size.height;
  const g = canvas.getContext('2d');
  if (!g) throw new Error('Tuval açılamadı (bellek yetmedi).');
  paintList(g, list, { scale: dpi / 25_400, x: 0, y: 0, minLinePx: 0, minTextPx: 0, placeholderFont: 'sans-serif' }, { image: (a) => s.paint.image(a), map: (p) => maps.get(p.item) ?? null, family: (f) => s.paint.family(f) });
  return canvas;
}

export async function exportPng(ctx: AppContext, s: SheetService, sheetId: string, dpi: number): Promise<string | null> {
  const engine = await s.ensureEngine();
  const book = s.book();
  if (!book) return null;
  try {
    const canvas = await paintPng(s, engine, book, sheetId, dpi);
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
    if (!blob) throw new Error('PNG yazılamadı (bellek yetmedi). Daha düşük bir dpi deneyin.');
    const name = book.book.sheets.find((x) => x.id === sheetId)!.name;
    return await saveExport(ctx, new Uint8Array(await blob.arrayBuffer()), fileName(name, '.png'), PNG);
  } catch (e) {
    s.report('PNG olarak aktarılamadı', e);
    return null;
  }
}

/**
 * A `.kpafta` file's text (design §10): the book and the bytes this device
 * has of its pictures, written by the core's codec (`encodeKpafta`, the
 * desktop's too: one format, no second writer here).
 */
export async function kpaftaOf(s: SheetService, book: SheetBook): Promise<string> {
  const engine = await s.ensureEngine();
  const assets: AssetWithBytes[] = [];
  for (const meta of book.assets) {
    const rec = await s.assets.get(meta.sha256);
    if (rec) assets.push({ meta, data: toBase64(rec.bytes) });
  }
  return engine.encodeKpafta(book, assets);
}

export async function exportKpafta(ctx: AppContext, s: SheetService): Promise<string | null> {
  const book = s.book();
  if (!book) return null;
  try {
    const text = await kpaftaOf(s, book.book);
    const name = ctx.doc.name.value || 'paftalar';
    return await saveExport(ctx, new TextEncoder().encode(text), fileName(name, '.kpafta'), KPAFTA);
  } catch (e) {
    s.report('.kpafta olarak aktarılamadı', e);
    return null;
  }
}

/**
 * The ids of a book given anew (`newId` each), every reference following
 * (a sheet's master, an item's group and the map it reads): so a book read
 * from a file joins this one without meeting its ids.
 */
export function renewIds(book: SheetBook, newId: () => string): SheetBook {
  const ids = new Set<string>();
  for (const s of book.sheets) {
    ids.add(s.id);
    s.items.forEach((i) => ids.add(i.id));
    s.guides.forEach((g) => ids.add(g.id));
  }
  for (const m of book.masters) {
    ids.add(m.id);
    m.items.forEach((i) => ids.add(i.id));
    m.guides.forEach((g) => ids.add(g.id));
  }
  let text = JSON.stringify(book);
  for (const id of ids) text = text.split(JSON.stringify(id)).join(JSON.stringify(newId()));
  return JSON.parse(text) as SheetBook;
}

/**
 * Reads a `.kpafta` file into the project: its sheets added beside the
 * project's (new ids), or in their place, as one undo step. The pictures'
 * bytes are kept on this device first.
 */
export async function importKpafta(ctx: AppContext, s: SheetService, text: string, fileLabel: string): Promise<boolean> {
  const engine = await s.ensureEngine();
  const now = s.book();
  if (!now) return false;
  let read: { book: SheetBook; assets: readonly AssetWithBytes[] };
  try {
    // The core's codec reads it whole or refuses it with its code (bad_json, unknown_schema, newer_schema, bad_asset …).
    read = engine.decodeKpafta(text);
  } catch (e) {
    ctx.log.error(`“${fileLabel}” okunamadı: ${errorText(e)}`);
    return false;
  }
  if (!read.book.sheets.length) {
    ctx.log.warn(`“${fileLabel}” dosyasında pafta yok.`);
    return false;
  }
  let replace = false;
  if (now.book.sheets.length) {
    const a = await confirmDialog({
      title: '.kpafta dosyasından',
      message: `“${fileLabel}” dosyasında ${read.book.sheets.length} pafta var. Bu projenin ${now.book.sheets.length} paftasının yanına mı eklensin, yerine mi?`,
      details: ['Yerine konursa projenin paftaları kalkar; Geri al (Ctrl+Z) onları geri getirir.'],
      answers: [
        { value: 'cancel', label: 'Vazgeç', aside: true },
        { value: 'replace', label: 'Yerine koy' },
        { value: 'add', label: 'Yanına ekle', kind: 'primary' },
      ],
      cancel: 'cancel',
      focus: 'add',
    });
    if (a === 'cancel') return false;
    replace = a === 'replace';
  }
  for (const a of read.assets) {
    try {
      await s.assets.put(a.meta, fromBase64(a.data));
    } catch (e) {
      ctx.log.warn(`“${a.meta.name}” resmi alınamadı: ${errorText(e)}`);
    }
  }
  const incoming = replace ? read.book : renewIds(read.book, () => s.newId());
  const have = new Set(now.book.assets.map((a) => a.sha256));
  const ops: Op[] = [
    ...(replace ? [...now.book.sheets.map((x) => ({ op: 'removeSheet' as const, id: x.id })), ...now.book.masters.map((m) => ({ op: 'removeMaster' as const, id: m.id }))] : []),
    ...incoming.masters.map((master) => ({ op: 'addMaster' as const, master })),
    ...incoming.sheets.map((sheet) => ({ op: 'addSheet' as const, sheet })),
    ...(incoming.assets.some((a) => replace || !have.has(a.sha256)) ? [{ op: 'addAssets' as const, assets: incoming.assets.filter((a) => replace || !have.has(a.sha256)) }] : []),
    ...(replace && incoming.variables.length ? [{ op: 'saveVariables' as const, variables: incoming.variables }] : []),
  ];
  if (!s.apply(ops, `İçe aktar: ${fileLabel}`)) return false;
  void s.paint.refreshAssets();
  s.openSheet(incoming.sheets[0].id);
  ctx.log.success(`“${fileLabel}” dosyasından ${incoming.sheets.length} pafta ${replace ? 'projenin paftalarının yerine kondu' : 'eklendi'}.`);
  return true;
}
