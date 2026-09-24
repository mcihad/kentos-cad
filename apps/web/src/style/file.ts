import { assetsOfSymbol, newItemId, type EditableSource, type StyleLibrary } from './library';
import type { LibraryAsset, LibraryCategory, LibraryItem, LibrarySymbol, ShapeName, Symbol } from '../model/style';

/**
 * The .kstil file: styles to export, import and share. Versioned JSON with
 * the assets its symbols draw with embedded, so a file is complete on its
 * own. Everything read from a file is checked (a shared file is untrusted
 * data) and SVG drawings are cleaned of scripts and outside references.
 */

export const STYLE_FORMAT = 'kentos-style';
export const STYLE_VERSION = 1;

export interface StyleFile {
  readonly format: typeof STYLE_FORMAT;
  readonly version: number;
  readonly exported: string;
  readonly items: readonly LibraryItem[];
  readonly categories?: readonly LibraryCategory[];
}

/** The chosen items, and the assets their symbols use (from any source), as a file. */
export function exportStyles(lib: StyleLibrary, ids: readonly string[]): StyleFile {
  const items = new Map<string, LibraryItem>();
  const put = (id: string) => {
    const it = lib.get(id);
    if (!it || items.has(id)) return;
    const { source: _s, ...plain } = it;
    items.set(id, JSON.parse(JSON.stringify(plain)) as LibraryItem);
    if (it.kind === 'symbol') assetsOfSymbol(it.symbol).forEach(put);
  };
  ids.forEach(put);
  return { format: STYLE_FORMAT, version: STYLE_VERSION, exported: new Date().toISOString(), items: [...items.values()] };
}

// ── Validation ─────────────────────────────────────────────────────────

const UNITS = ['mm', 'px', 'm'];
// A record, so a shape added to the model cannot be left out of the check.
const SHAPE_NAMES: Record<ShapeName, true> = {
  circle: true, ring: true, square: true, rectangle: true, diamond: true, triangle: true, pentagon: true, hexagon: true, octagon: true,
  star: true, cross: true, x: true, line: true, arrow: true, arrowhead: true, chevron: true, semicircle: true, quartercircle: true,
  gear: true, arc: true,
};
const SHAPES = Object.keys(SHAPE_NAMES);
const PLACEMENTS = ['interval', 'vertex', 'innerVertex', 'first', 'last', 'center', 'segmentCenter'];
const LAYER_TYPES: Record<Symbol['type'], string[]> = {
  marker: ['shape', 'svg', 'text', 'raster'],
  line: ['simpleLine', 'markerLine'],
  fill: ['simpleFill', 'hatchFill', 'patternFill', 'imageFill', 'centroidMarker', 'simpleLine', 'markerLine'],
};
const COLOR = /^(#[0-9a-f]{6}([0-9a-f]{2})?|ink|paper|fg|fg-dim)$/i;

type O = Record<string, unknown>;
const isObj = (v: unknown): v is O => !!v && typeof v === 'object' && !Array.isArray(v);
const isExpr = (v: unknown) => isObj(v) && typeof v.expr === 'string';

/** Problems in a symbol, each with where it is ("katman 2 › işaret › katman 1: …"). */
export function validateSymbol(sym: unknown, where = 'sembol'): string[] {
  const issues: string[] = [];
  const bad = (w: string, m: string) => issues.push(`${w}: ${m}`);
  const num = (w: string, v: unknown, name: string, opts: { min?: number; optional?: boolean; dd?: boolean } = {}) => {
    if (v === undefined && opts.optional !== false) return;
    if (opts.dd && isExpr(v)) return;
    if (typeof v !== 'number' || !Number.isFinite(v)) return bad(w, `${name} bir sayı olmalı`);
    if (opts.min !== undefined && v < opts.min) bad(w, `${name} en az ${opts.min} olmalı`);
  };
  const color = (w: string, v: unknown, name: string, nullable = false) => {
    if (v === undefined || (nullable && v === null)) return;
    if (isExpr(v)) return;
    if (typeof v !== 'string' || !COLOR.test(v)) bad(w, `${name} geçerli bir renk değil (#RRGGBB, #RRGGBBAA, ink, paper, fg, fg-dim)`);
  };
  const walk = (s: unknown, w: string) => {
    if (!isObj(s)) return bad(w, 'nesne değil');
    const type = s.type as Symbol['type'];
    if (!LAYER_TYPES[type]) return bad(w, `bilinmeyen sembol türü “${String(s.type)}”`);
    if (!Array.isArray(s.layers)) return bad(w, 'katman listesi yok');
    s.layers.forEach((l: unknown, i: number) => {
      const lw = `${w} › katman ${i + 1}`;
      if (!isObj(l)) return bad(lw, 'nesne değil');
      if (typeof l.id !== 'string') bad(lw, 'kimlik yok');
      if (!LAYER_TYPES[type].includes(String(l.type))) return bad(lw, `“${type}” sembolünde “${String(l.type)}” katmanı olamaz`);
      if (l.unit !== undefined && !UNITS.includes(String(l.unit))) bad(lw, `bilinmeyen birim “${String(l.unit)}”`);
      num(lw, l.opacity, 'opaklık', { min: 0 });
      switch (l.type) {
        case 'shape':
          if (!SHAPES.includes(String(l.shape))) bad(lw, `bilinmeyen şekil “${String(l.shape)}”`);
          num(lw, l.size, 'boyut', { min: 0, dd: true, optional: false });
          color(lw, l.fill, 'dolgu', true);
          color(lw, l.stroke, 'çizgi rengi', true);
          num(lw, l.strokeWidth, 'çizgi kalınlığı', { min: 0 });
          num(lw, l.hole, 'delik', { min: 0 });
          num(lw, l.teeth, 'diş sayısı', { min: 3 });
          num(lw, l.teethDepth, 'diş derinliği', { min: 0 });
          num(lw, l.sweep, 'yay açıklığı', { min: 0 });
          break;
        case 'svg':
        case 'raster':
          if (typeof l.asset !== 'string') bad(lw, 'varlık kimliği yok');
          num(lw, l.size, 'boyut', { min: 0, dd: true, optional: false });
          break;
        case 'text':
          if (typeof l.text !== 'string' && !isExpr(l.text)) bad(lw, 'metin yok');
          num(lw, l.size, 'boyut', { min: 0, dd: true, optional: false });
          color(lw, l.color, 'renk');
          break;
        case 'simpleLine':
          color(lw, l.color, 'renk');
          num(lw, l.width, 'kalınlık', { min: 0, dd: true, optional: false });
          if (l.dash !== undefined && l.dash !== null) {
            if (!Array.isArray(l.dash) || l.dash.some((d) => typeof d !== 'number' || !Number.isFinite(d) || d < 0)) bad(lw, 'kesik deseni sıfır ya da pozitif sayılar olmalı');
            else if (l.dash.length && l.dash.every((d) => d === 0)) bad(lw, 'kesik deseninin toplamı sıfır olamaz');
          }
          num(lw, l.offset, 'kaydırma', { dd: true });
          num(lw, l.blur, 'yumuşatma', { min: 0 });
          if (l.shift !== undefined && (!Array.isArray(l.shift) || l.shift.length !== 2 || l.shift.some((v) => typeof v !== 'number' || !Number.isFinite(v)))) bad(lw, 'sayfa kaydırması iki sayı olmalı');
          if (l.wave !== undefined) {
            const w = l.wave;
            if (!isObj(w) || !['sine', 'zigzag', 'square'].includes(String(w.shape))) bad(lw, 'dalga biçimi sine, zigzag ya da square olmalı');
            else {
              num(lw, w.length, 'dalga boyu', { min: 0.0001, optional: false });
              num(lw, w.amplitude, 'dalga genliği', { min: 0, optional: false });
              num(lw, w.spacing, 'dalga tekrarı', { min: 0.0001 });
              num(lw, w.offsetAlong, 'ilk dalga uzaklığı');
            }
          }
          break;
        case 'markerLine':
          if (!PLACEMENTS.includes(String(l.placement))) bad(lw, `bilinmeyen yerleşim “${String(l.placement)}”`);
          if (l.placement === 'interval') num(lw, l.interval, 'aralık', { min: 0.0001, optional: false });
          num(lw, l.offsetAlong, 'başlangıç uzaklığı');
          num(lw, l.offset, 'kaydırma', { dd: true });
          walk(l.marker, `${lw} › işaret`);
          break;
        case 'simpleFill':
          color(lw, l.color, 'renk');
          break;
        case 'hatchFill':
          num(lw, l.angle, 'açı', { optional: false });
          num(lw, l.spacing, 'aralık', { min: 0.0001, optional: false });
          num(lw, l.width, 'kalınlık', { min: 0, optional: false });
          color(lw, l.color, 'renk');
          break;
        case 'patternFill':
          num(lw, l.spacingX, 'yatay aralık', { min: 0.0001, optional: false });
          num(lw, l.spacingY, 'düşey aralık', { min: 0.0001, optional: false });
          walk(l.marker, `${lw} › işaret`);
          break;
        case 'imageFill':
          if (typeof l.asset !== 'string') bad(lw, 'varlık kimliği yok');
          num(lw, l.tileSize, 'döşeme boyu', { min: 0.0001, optional: false });
          break;
        case 'centroidMarker':
          walk(l.marker, `${lw} › işaret`);
          break;
      }
      if (isObj(l.marker) && (l.marker as O).type !== 'marker') bad(lw, 'iç sembol bir işaret sembolü olmalı');
    });
  };
  walk(sym, where);
  return issues;
}

/**
 * An SVG drawing made safe to keep and draw: no scripts, no event
 * handlers, no foreign objects, no references outside the file.
 */
export function sanitizeSvg(svg: string): string {
  return svg
    .replace(/<\?xml[^>]*>/gi, '')
    .replace(/<!DOCTYPE[^>]*>/gi, '')
    .replace(/<script[\s\S]*?<\/script\s*>/gi, '')
    .replace(/<script[^>]*\/>/gi, '')
    .replace(/<foreignObject[\s\S]*?<\/foreignObject\s*>/gi, '')
    .replace(/\s+on[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)/gi, '')
    .replace(/\s+(?:xlink:)?href\s*=\s*("(?!#|data:image\/)[^"]*"|'(?!#|data:image\/)[^']*')/gi, '')
    .replace(/url\(\s*['"]?(?!#)[^)]*\)/gi, 'none')
    .trim();
}

function validateItem(it: unknown, i: number): string[] {
  const w = `öğe ${i + 1}`;
  if (!isObj(it)) return [`${w}: nesne değil`];
  const issues: string[] = [];
  if (typeof it.id !== 'string' || !it.id) issues.push(`${w}: kimlik yok`);
  if (typeof it.name !== 'string') issues.push(`${w}: ad yok`);
  if (!Array.isArray(it.path) || it.path.some((p) => typeof p !== 'string')) issues.push(`${w}: kategori yolu metin listesi olmalı`);
  if (it.kind === 'symbol') issues.push(...validateSymbol(it.symbol, `${w} (${String(it.name)})`));
  else if (it.kind === 'asset') {
    if (!['svg', 'png', 'jpeg'].includes(String(it.format))) issues.push(`${w}: bilinmeyen varlık biçimi`);
    if (typeof it.data !== 'string') issues.push(`${w}: varlık verisi yok`);
    else if (it.format === 'svg' && !/<svg[\s>]/i.test(it.data)) issues.push(`${w}: SVG çizimi değil`);
    else if (it.format !== 'svg' && !/^data:image\/(png|jpeg);base64,/.test(it.data)) issues.push(`${w}: görüntü verisi data: adresi olmalı`);
    if (typeof it.width !== 'number' || typeof it.height !== 'number' || !(it.width > 0) || !(it.height > 0)) issues.push(`${w}: boyut yok`);
  } else issues.push(`${w}: bilinmeyen öğe türü “${String(it.kind)}”`);
  return issues;
}

/** Parses and checks a file's text; `file` is set only when there is nothing wrong. */
export function parseStyleFile(text: string): { file?: StyleFile; issues: string[] } {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    return { issues: ['Dosya okunamadı: JSON değil.'] };
  }
  if (!isObj(data) || data.format !== STYLE_FORMAT) return { issues: ['Bu bir KentOS stil dosyası (.kstil) değil.'] };
  if (typeof data.version !== 'number' || data.version > STYLE_VERSION) return { issues: [`Dosya daha yeni bir KentOS sürümüyle yazılmış (sürüm ${String(data.version)}); güncelleyip yeniden deneyin.`] };
  if (!Array.isArray(data.items)) return { issues: ['Dosyada öğe listesi yok.'] };
  const issues = data.items.flatMap(validateItem);
  if (issues.length) return { issues };
  const items = (data.items as LibraryItem[]).map((it) => (it.kind === 'asset' && it.format === 'svg' ? { ...it, data: sanitizeSvg(it.data) } : it));
  return { file: { ...(data as unknown as StyleFile), items }, issues: [] };
}

export type ConflictMode = 'replace' | 'copy' | 'skip';

export interface ImportReport {
  added: number;
  replaced: number;
  skipped: number;
  /** Ids the file used that were renamed on "copy" (old → new). */
  renamed: Record<string, string>;
}

/**
 * Adds a file's items to the user's or the project's library. An id that
 * already exists is replaced (only in the same editable source), taken as
 * a copy under a new id (symbols then point at the renamed assets), or
 * skipped. System items are never replaced.
 */
export function importStyles(lib: StyleLibrary, file: StyleFile, to: EditableSource, mode: ConflictMode): ImportReport {
  const report: ImportReport = { added: 0, replaced: 0, skipped: 0, renamed: {} };
  const assetsFirst = [...file.items].sort((a, b) => (a.kind === b.kind ? 0 : a.kind === 'asset' ? -1 : 1));
  for (const raw of assetsFirst) {
    let item = raw;
    const existing = lib.get(item.id);
    if (existing) {
      if (mode === 'skip' || (mode === 'replace' && existing.source !== to)) {
        report.skipped++;
        continue;
      }
      if (mode === 'replace') {
        const { id: _i, kind: _k, ...patch } = item;
        lib.update(item.id, patch as Partial<Omit<LibrarySymbol, 'id' | 'kind'>>);
        report.replaced++;
        continue;
      }
      const id = newItemId(to === 'project' ? 'p' : 'u');
      report.renamed[item.id] = id;
      item = { ...item, id };
    }
    if (item.kind === 'symbol' && Object.keys(report.renamed).length) item = { ...item, symbol: renameAssets(item.symbol, report.renamed) };
    lib.add(to, item);
    report.added++;
  }
  for (const c of file.categories ?? []) lib.addCategory(to, c);
  return report;
}

function renameAssets(sym: Symbol, renamed: Record<string, string>): Symbol {
  const json = JSON.stringify(sym, (k, v) => (k === 'asset' && typeof v === 'string' && renamed[v] ? renamed[v] : v));
  return JSON.parse(json) as Symbol;
}

/** A new asset from an SVG text (cleaned) with its size from the viewBox or width/height. */
export function svgAsset(name: string, path: readonly string[], svg: string, id = newItemId('a')): LibraryAsset {
  const clean = sanitizeSvg(svg);
  const vb = /viewBox\s*=\s*["']\s*[-\d.]+[\s,]+[-\d.]+[\s,]+([\d.]+)[\s,]+([\d.]+)/i.exec(clean);
  const w = /\swidth\s*=\s*["']([\d.]+)/i.exec(clean);
  const h = /\sheight\s*=\s*["']([\d.]+)/i.exec(clean);
  const width = Number(vb?.[1] ?? w?.[1] ?? 100) || 100;
  const height = Number(vb?.[2] ?? h?.[1] ?? 100) || 100;
  return { kind: 'asset', id, name, path: [...path], format: 'svg', data: clean, width, height };
}
