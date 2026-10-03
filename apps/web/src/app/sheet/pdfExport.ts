import type { CrsDef } from '../../geo/crs';
import type { DisplayList } from '../../contracts/generated/sheet/DisplayList';
import type { Finding } from '../../contracts/generated/sheet/Finding';
import type { MapContent } from '../../contracts/generated/sheet/MapContent';
import type { MapLayerContent } from '../../contracts/generated/sheet/MapLayerContent';
import type { MapPath } from '../../contracts/generated/sheet/MapPath';
import type { MapPrim } from '../../contracts/generated/sheet/MapPrim';
import type { PdfAsset } from '../../contracts/generated/sheet/PdfAsset';
import type { PdfFace } from '../../contracts/generated/sheet/PdfFace';
import type { PdfInputs } from '../../contracts/generated/sheet/PdfInputs';
import type { PdfMap } from '../../contracts/generated/sheet/PdfMap';
import type { PdfOptions } from '../../contracts/generated/sheet/PdfOptions';
import type { RenderInputs } from '../../contracts/generated/sheet/RenderInputs';
import type { TmCrs } from '../../contracts/generated/sheet/TmCrs';
import { errorText, type BookText, type SheetEngine } from '../../product/sheet/engine';
import { toBase64 } from '../../product/sheet/store';
import { saveExport } from '../../ui/io/save';
import type { PdfAction } from '../../ui/sheet/exportPdf';
import type { AppContext } from '../context';
import type { FileKind } from '../fileIO';
import { fileName } from './exporting';
import { crsInfo } from './inputs';
import { vectorMap, type VecLayer } from './mapContent';
import type { VecPath } from './mapVectors';
import { fontFiles } from './pdfFonts';
import type { SheetService } from './service';

/**
 * A PDF of sheets (docs/sheet/design.md §9a), written by the core's writer
 * (`toPdf`, the desktop's too): the sheets chosen, a page each; every map
 * frame's content from the web's own drawing pipeline, as vectors (each
 * drawing layer an optional content group) or, where a layer draws what
 * has no vector form here, as a picture at the fallback resolution; the
 * pictures' bytes, an SVG one with the PNG the browser draws of it at the
 * pixels the core asks (`pdfSvgSizes`: the core draws no SVG); the faces
 * the core asks for (`pdfFonts`), fetched now and only now (pdfFonts.ts:
 * the desktop's files); and, for GeoPDF, the project's system as WKT
 * (`tmWkt`). What the PDF then writes differently from the screen
 * (`pdfFindings`) is said in the log, as the desktop says it.
 */

export interface PdfChoice {
  /** The sheets, in this order, a page each. */
  readonly sheets: readonly string[];
  /** The fallback pictures' resolution (and the export's, for the preflight). */
  readonly dpi: number;
  /** A geographic viewport for every placed map (needs the project's system). */
  readonly geo: boolean;
  /** A vector map's layers as the viewer's layer list. */
  readonly layers: boolean;
}

/** How a map frame goes into the PDF: vectors, or a picture and why. */
export interface MapWay {
  readonly sheet: string;
  readonly item: string;
  readonly name: string;
  readonly vector: boolean;
  /** For a picture: the layers drawing what has no vector form, and what. */
  readonly why: string;
}

/** The faces' file names (apps/desktop/assets/fonts/drawing, ADR 0055), by the table's id. */
const FILE_STEM: Readonly<Record<string, string>> = {
  'architects-daughter': 'ArchitectsDaughter',
  arimo: 'Arimo',
  barlow: 'Barlow',
  'courier-prime': 'CourierPrime',
  overpass: 'Overpass',
  'plex-mono': 'IBMPlexMono',
  quicksand: 'Quicksand',
};

/** A face's file: `<Family>-<weight>[-italic].ttf` (the desktop's names). */
export const faceFile = (f: PdfFace): string => `${FILE_STEM[f.font] ?? f.font}-${f.weight}${f.italic ? '-italic' : ''}.ttf`;

/** The registry's names as the core's WKT writer reads them (`WGS84` is written `WGS 84`). */
const named = (s: string) => (s === 'WGS84' ? 'WGS 84' : s);

/** The project's system for `tmWkt`; null when it is not a transverse Mercator one. */
export function tmCrsOf(crs: CrsDef): TmCrs | null {
  const info = crsInfo(crs);
  if (!info.tm || !crs.ellipsoid) return null;
  return { name: crs.name, datum: named(crs.datum), ellipsoid: named(crs.ellipsoid), epsg: crs.srid, tm: info.tm };
}

const color = (c: string, opacity: number) => (opacity >= 0.999 ? c : `${c}${Math.round(Math.max(0, opacity) * 255).toString(16).padStart(2, '0')}`);
const pairs = (pts: readonly number[]): [number, number][] => Array.from({ length: pts.length / 2 }, (_, i) => [pts[2 * i], pts[2 * i + 1]]);
/** Twice a ring's signed area: positive counter-clockwise. */
const signed = (p: readonly number[]) => {
  let a = 0;
  for (let i = 0, n = p.length; i < n; i += 2) a += p[i] * p[(i + 3) % n] - p[(i + 2) % n] * p[i + 1];
  return a;
};
const inside = (x: number, y: number, p: readonly number[]) => {
  let at = false;
  for (let i = 0, j = p.length - 2; i < p.length; j = i, i += 2) if (p[i + 1] > y !== p[j + 1] > y && x < ((p[j] - p[i]) * (y - p[i + 1])) / (p[j + 1] - p[i + 1]) + p[i]) at = !at;
  return at;
};

/**
 * One of our paths as the core's: a filled one as rings with their holes, a line as it is. A fill covers
 * every part with an area, closed or not, as a canvas fills it (our shapes close theirs; a full circle is
 * closed by `Outline.rings`). Non-zero rings: the outers turn counter-clockwise, the holes clockwise, each
 * hole given to the outer it lies in. Even-odd: each shape (`groups`) one path, its first ring outside and
 * the rest its holes, which the core fills even-odd as the canvas fills that shape (a marker's hole stays
 * empty; two markers overlapping both stay filled).
 */
export function corePaths(p: VecPath): MapPath[] {
  const stroke = p.stroke ? { color: color(p.stroke.color, p.stroke.opacity), width: p.stroke.width, dash: p.stroke.dash ? [...p.stroke.dash] : [], cap: p.stroke.cap, join: p.stroke.join } : undefined;
  if (!p.fill) return p.parts.map((x) => ({ points: pairs(x.points), closed: x.closed, holes: [], ...(stroke && { stroke }) }));
  const fill = color(p.fill.color, p.fill.opacity);
  const area = (x: VecPath['parts'][number]) => x.points.length >= 6;
  const out: MapPath[] = [];
  if (p.fill.rule === 'evenodd') {
    const starts = p.groups?.length ? p.groups : [0];
    starts.forEach((s, i) => {
      const rings = p.parts.slice(s, starts[i + 1] ?? p.parts.length).filter(area);
      if (rings.length) out.push({ points: pairs(rings[0].points), closed: true, holes: rings.slice(1).map((x) => pairs(x.points)), fill, ...(stroke && { stroke }) });
    });
  } else {
    const rings = p.parts.filter(area);
    const outers = rings.filter((x) => signed(x.points) > 0);
    const holes = rings.filter((x) => signed(x.points) <= 0);
    out.push(...outers.map((x) => ({ points: pairs(x.points), closed: true, holes: [] as [number, number][][], fill, ...(stroke && { stroke }) })));
    for (const h of holes) {
      const owner = outers.findIndex((o) => inside(h.points[0], h.points[1], o.points));
      if (owner >= 0) out[owner].holes.push(pairs(h.points));
    }
  }
  // A part too short to cover anything (two points) is drawn as a line.
  for (const x of p.parts.filter((y) => !area(y))) if (stroke) out.push({ points: pairs(x.points), closed: false, holes: [], stroke });
  return out;
}

type CoreText = MapLayerContent['texts'][number];

const coreText = (t: VecLayer['texts'][number]): CoreText => ({
  at: [t.x, t.y],
  text: t.text,
  size: t.size,
  rotation: t.rotation,
  color: t.color,
  font: t.font.family,
  weight: t.font.weight,
  italic: t.font.italic,
  anchor: `${t.align === 'center' ? 'center' : 'left'}${t.baseline === 'middle' ? 'Middle' : 'Baseline'}` as CoreText['anchor'],
  ...(t.halo && { halo: t.halo.color }),
});

/**
 * A vector map's layers as the core's, in two passes as the map frames draw
 * them: every layer's lines and areas bottom first, then every layer's text
 * masks and texts over the whole drawing. A layer is one optional content
 * group either way (the core keys the groups by id, in the order a layer is
 * first met: the first pass names every layer, so the viewer's list is the
 * drawing's order).
 */
export function coreLayers(layers: readonly VecLayer[]): MapLayerContent[] {
  const shapes = layers.map((l) => ({ id: l.id, name: l.name, paths: l.paths.flatMap(corePaths), texts: [] }));
  const writing = layers.filter((l) => l.masks.length || l.texts.length).map((l) => ({ id: l.id, name: l.name, paths: l.masks.flatMap(corePaths), texts: l.texts.map(coreText) }));
  return [...shapes, ...writing];
}

/** The map frames of display lists, each once (a master page's map is on every sheet that uses it). */
function mapsOf(lists: readonly { sheet: string; list: DisplayList }[]): { sheet: string; prim: MapPrim }[] {
  const seen = new Set<string>();
  const out: { sheet: string; prim: MapPrim }[] = [];
  for (const { sheet, list } of lists)
    for (const p of list.prims)
      if (p.type === 'map' && !seen.has(p.item)) {
        seen.add(p.item);
        out.push({ sheet, prim: p });
      }
  return out;
}

/** The sheets' inputs as one (each sheet's tables, lists, legends and maps are its items'; the project's are one). */
function merged(all: readonly RenderInputs[]): RenderInputs {
  const first = all[0];
  const uniq = <T extends { item: string }>(xs: readonly T[]) => [...new Map(xs.map((x) => [x.item, x])).values()];
  return {
    ...first,
    maps: uniq(all.flatMap((x) => x.maps)),
    tables: uniq(all.flatMap((x) => x.tables)),
    legends: uniq(all.flatMap((x) => x.legends)),
    coordinates: uniq(all.flatMap((x) => x.coordinates)),
  };
}

/** The display lists of the sheets chosen, at the export's resolution, with the inputs each was made with. */
function plans(s: SheetService, engine: SheetEngine, book: BookText, choice: PdfChoice) {
  return choice.sheets.map((id) => {
    const sheet = book.book.sheets.find((x) => x.id === id);
    if (!sheet) throw new Error('Pafta kitapta yok.');
    const inputs = s.paint.inputs(engine, book, sheet, 'export', { dpi: choice.dpi });
    return { sheet: id, inputs, list: engine.displayList(book, id, inputs) };
  });
}

/** How each map frame of the sheets goes into the PDF (the export window says it before anything is written). */
export function mapWays(ctx: AppContext, s: SheetService, choice: Pick<PdfChoice, 'sheets' | 'dpi'>): MapWay[] {
  const engine = s.engine();
  const book = s.book();
  if (!engine || !book) return [];
  const names = new Map(book.book.sheets.flatMap((x) => x.items.map((i) => [i.id, i.name] as const)).concat(book.book.masters.flatMap((m) => m.items.map((i) => [i.id, i.name] as const))));
  return mapsOf(plans(s, engine, book, { ...choice, geo: false, layers: false })).map(({ sheet, prim }) => {
    const vm = vectorMap(ctx, prim);
    const why = vm?.unsupported.map((u) => `${u.layer} (${u.what.join(', ')})`).join('; ') ?? 'haritanın yeri seçilmedi';
    return { sheet, item: prim.item, name: names.get(prim.item) ?? 'Harita', vector: !!vm && !vm.unsupported.length, why: vm && !vm.unsupported.length ? '' : why };
  });
}

/** A map's content: vectors when every layer has a vector form, otherwise its picture at the fallback resolution. */
async function contentOf(ctx: AppContext, s: SheetService, prim: MapPrim, dpi: number): Promise<MapContent | null> {
  const vm = vectorMap(ctx, prim);
  if (vm && !vm.unsupported.length) return { type: 'vector', layers: coreLayers(vm.layers) };
  // No paper under it (straight alpha, as the desktop's): the frame's own background shows through.
  const pic = await s.paint.maps.exportPicture(prim, dpi / 25_400, true);
  if (!pic) return null;
  const data = pic.toDataURL('image/png');
  return { type: 'raster', png: data.slice(data.indexOf(',') + 1) };
}

/** Draws an SVG picture's bytes as a PNG of `width` × `height` pixels (tests give their own). */
export type SvgDraw = (bytes: Uint8Array, width: number, height: number) => Promise<Uint8Array>;

/**
 * An SVG picture as the PNG the PDF embeds in its place: the browser draws it at the pixels the core
 * asks, stretched over them as the paper draws a picture over its rectangle (render/sheet/painter.ts).
 * Thrown when the browser cannot read it.
 */
export async function svgPng(bytes: Uint8Array, width: number, height: number): Promise<Uint8Array> {
  const url = URL.createObjectURL(new Blob([bytes as Uint8Array<ArrayBuffer>], { type: 'image/svg+xml' }));
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    const c = new OffscreenCanvas(width, height);
    const g = c.getContext('2d');
    if (!g) throw new Error('tuval açılamadı');
    g.drawImage(img, 0, 0, width, height);
    return new Uint8Array(await (await c.convertToBlob({ type: 'image/png' })).arrayBuffer());
  } finally {
    URL.revokeObjectURL(url);
  }
}

/** The SVG pictures' PNGs drawn lately, by who drew them, the picture and its size: the export window's notes draw them, the PDF reuses them. */
const drawnSvgs = new WeakMap<SvgDraw, Map<string, string>>();
/** At most this much of them is kept (base64 characters): a large frame at a high dpi is megabytes. */
const DRAWN_BUDGET = 64 * 1024 * 1024;

/** Gives each SVG picture of `inputs` the PNG drawn of it at the pixels the core asks; one that cannot be drawn goes without. */
async function withSvgRasters(s: SheetService, engine: SheetEngine, book: BookText, inputs: PdfInputs, options: PdfOptions, dpi: number, draw: SvgDraw): Promise<void> {
  let kept = drawnSvgs.get(draw);
  if (!kept) drawnSvgs.set(draw, (kept = new Map()));
  for (const size of engine.pdfSvgSizes(book, inputs, options, dpi)) {
    const asset = inputs.assets.find((a) => a.sha256 === size.sha256);
    if (!asset) continue;
    const key = `${size.sha256}|${size.width}x${size.height}`;
    let png = kept.get(key);
    if (png === undefined) {
      const rec = await s.assets.get(size.sha256);
      const bytes = rec ? await draw(rec.bytes, size.width, size.height).catch(() => null) : null;
      if (!bytes) continue;
      png = toBase64(bytes);
      kept.set(key, png);
      let total = 0;
      for (const v of kept.values()) total += v.length;
      for (const [k, v] of kept) {
        if (total <= DRAWN_BUDGET || k === key) break;
        kept.delete(k);
        total -= v.length;
      }
    }
    asset.raster = png;
  }
}

/**
 * What the PDF will write differently from the screen, in the core's words (`pdfFindings`): an SVG picture
 * as the PNG drawn of it (its pixels), or the missing picture's box where it cannot be drawn. For the export
 * window, before anything is written; the PNGs drawn here are the ones the PDF then embeds. None when the
 * sheets cannot be planned (the window says why elsewhere).
 */
export async function pdfNotes(s: SheetService, choice: Pick<PdfChoice, 'sheets' | 'dpi'>, draw: SvgDraw = svgPng): Promise<Finding[]> {
  const engine = await s.ensureEngine();
  const book = s.book();
  if (!book || !choice.sheets.length) return [];
  try {
    const made = plans(s, engine, book, { ...choice, geo: false, layers: false });
    const options: PdfOptions = { sheets: [...choice.sheets], geo: false, layers: false };
    const inputs: PdfInputs = { render: merged(made.map((m) => m.inputs)), fonts: [], assets: await picturesOf(s, made.map((m) => m.list)), maps: [] };
    await withSvgRasters(s, engine, book, inputs, options, choice.dpi, draw);
    return engine.pdfFindings(book, inputs, options);
  } catch {
    return [];
  }
}

/** The pictures the plans draw: the book's (by their SHA-256) and the legends' symbols (by their keys). */
async function picturesOf(s: SheetService, lists: readonly DisplayList[]): Promise<PdfAsset[]> {
  const keys = new Set<string>();
  for (const l of lists) for (const p of l.prims) if (p.type === 'image') keys.add(p.asset);
  const out: PdfAsset[] = [];
  for (const key of keys) {
    const rec = await s.assets.get(key);
    if (rec) {
      out.push({ sha256: key, data: toBase64(rec.bytes) });
      continue;
    }
    const href = s.paint.symbolHref(key);
    if (href) out.push({ sha256: key, data: href.slice(href.indexOf(',') + 1) });
  }
  return out;
}

/** A sheet's export file name: its `export.fileName` (`[% … %]` text, by default `[% @pafta_adi %]`) as the engine writes it. */
export function exportName(_s: SheetService, engine: SheetEngine, book: BookText, sheetId: string, inputs: RenderInputs): string {
  try {
    return engine.exportName(book, sheetId, inputs).trim() || 'pafta';
  } catch {
    return book.book.sheets.find((x) => x.id === sheetId)?.name ?? 'pafta';
  }
}

/** The PDF's file name: the sheet's export name for one sheet, the drawing's for several. */
export function pdfName(ctx: AppContext, s: SheetService, choice: PdfChoice): string {
  const engine = s.engine();
  const book = s.book();
  if (!engine || !book || !choice.sheets.length) return 'paftalar';
  if (choice.sheets.length > 1) return `${(ctx.doc.name.value || 'çizim').replace(/\.[a-z0-9]+$/i, '')} paftaları`;
  const sheet = book.book.sheets.find((x) => x.id === choice.sheets[0]);
  if (!sheet) return 'pafta';
  return exportName(s, engine, book, sheet.id, s.paint.inputs(engine, book, sheet, 'export', { dpi: choice.dpi }));
}

/**
 * Writes the PDF: maps, pictures, faces and the system gathered, then the
 * core's writer; what it writes differently from the screen is said in the
 * log. Thrown with the reason when it cannot. `load` fetches the faces'
 * files, `draw` draws an SVG picture (tests give their own).
 */
export async function makePdf(ctx: AppContext, s: SheetService, choice: PdfChoice, load: (names: readonly string[]) => Promise<Map<string, Uint8Array>> = fontFiles, draw: SvgDraw = svgPng): Promise<Uint8Array> {
  const engine = await s.ensureEngine();
  const book = s.book();
  if (!book) throw new Error('Pafta kitabı yok.');
  const made = plans(s, engine, book, choice);
  const maps: PdfMap[] = [];
  for (const { prim } of mapsOf(made)) {
    const content = await contentOf(ctx, s, prim, choice.dpi);
    if (content) maps.push({ item: prim.item, content });
  }
  const crsDef = ctx.doc.crs.value;
  const tm = choice.geo && crsDef ? tmCrsOf(crsDef) : null;
  const options: PdfOptions = { sheets: [...choice.sheets], geo: choice.geo && !!tm, layers: choice.layers };
  const inputs: PdfInputs = {
    render: merged(made.map((m) => m.inputs)),
    fonts: [],
    assets: await picturesOf(s, made.map((m) => m.list)),
    maps,
    ...(tm && { crs: { wkt: engine.tmWkt(tm), epsg: tm.epsg } }),
  };
  // An SVG picture goes as the PNG the browser draws of it at the pixels the core asks (its largest frame at
  // the export's dpi). One the browser cannot draw goes without: the core prints the missing picture's box and
  // its findings say so.
  await withSvgRasters(s, engine, book, inputs, options, choice.dpi, draw);
  // The faces the core will write with, fetched now (the desktop's files, never another).
  const faces = engine.pdfFonts(book, inputs, options);
  const files = await load(faces.map(faceFile));
  // A face without its file is left out: the core then names it (`pdf_font_missing`).
  inputs.fonts = faces.flatMap((f) => {
    const data = files.get(faceFile(f));
    return data ? [{ font: f.font, weight: f.weight, italic: f.italic, data: toBase64(data) }] : [];
  });
  const bytes = engine.toPdf(book, inputs, options);
  // What the PDF writes differently from the screen, as the desktop says it: a note as information, the rest
  // as a warning with its fix.
  for (const f of engine.pdfFindings(book, inputs, options)) {
    if (f.severity === 'info') ctx.log.info(f.message);
    else ctx.log.warn(`${f.message} ${f.fix}`);
  }
  return bytes;
}

/** The PDF in the browser's print window (a hidden frame that prints itself), else in a new tab to print from there. */
export function printPdf(bytes: Uint8Array): void {
  const url = URL.createObjectURL(new Blob([bytes as Uint8Array<ArrayBuffer>], { type: 'application/pdf' }));
  const frame = document.createElement('iframe');
  frame.style.cssText = 'position:fixed;right:0;bottom:0;width:0;height:0;border:0;visibility:hidden';
  frame.src = url;
  frame.addEventListener('load', () => {
    try {
      frame.contentWindow?.focus();
      frame.contentWindow?.print();
    } catch {
      window.open(url, '_blank');
    }
    // The print window holds what it needs; the frame and the address go a minute later.
    setTimeout(() => {
      frame.remove();
      URL.revokeObjectURL(url);
    }, 60_000);
  });
  document.body.append(frame);
}

/** Opens the PDF in a tab: `target`, a tab opened at the click (a browser blocks one opened later), else a new one. */
export function openPdf(bytes: Uint8Array, target: Window | null): void {
  const url = URL.createObjectURL(new Blob([bytes as Uint8Array<ArrayBuffer>], { type: 'application/pdf' }));
  if (target) target.location.href = url;
  else window.open(url, '_blank');
  setTimeout(() => URL.revokeObjectURL(url), 5 * 60_000);
}

const PDF: FileKind = { description: 'PDF belgesi', accept: { 'application/pdf': ['.pdf'] } };

/** Writes the PDF and saves it, prints it or opens it in `tab` (one opened at the click); its name, or null (said why in the log). */
export async function exportPdf(ctx: AppContext, s: SheetService, choice: PdfChoice, action: PdfAction, tab: Window | null): Promise<string | null> {
  const name = fileName(pdfName(ctx, s, choice), '.pdf');
  let bytes: Uint8Array;
  try {
    bytes = await makePdf(ctx, s, choice);
  } catch (e) {
    ctx.log.error(`PDF yazılamadı: ${errorText(e)}`);
    return null;
  }
  if (action === 'print') {
    printPdf(bytes);
    ctx.log.info(`“${name}” yazdırma penceresinde açıldı.`);
    return name;
  }
  if (action === 'open') {
    openPdf(bytes, tab);
    return name;
  }
  return saveExport(ctx, bytes, name, PDF);
}
