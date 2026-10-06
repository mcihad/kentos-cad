import type { HatchAssoc, HatchPattern } from './entities';

/**
 * What is wrong with a hatch's pattern and its tie (docs/adr/0186 §1, §6): the field and the refusal's words, none
 * when they may be written. The commands refuse with them (`invalid_hatch`) and the drawing's reader too; the
 * desktop's are `kentos_contracts::hatch`'s `HatchPattern::problem` and `HatchAssoc::problem`, word for word.
 */

/** The most line families a pattern may have, and dashes a family. */
export const MAX_PATTERN_LINES = 64;
export const MAX_PATTERN_DASHES = 16;
/** The longest a pattern's name may be. */
export const MAX_PATTERN_NAME = 64;
/** The largest number a pattern's definition may hold, and its scale. */
export const MAX_PATTERN_SIZE = 1e6;
/** The most objects a hatch's tie may name. */
export const MAX_ASSOC_OBJECTS = 100_000;

const fits = (x: number) => Number.isFinite(x) && Math.abs(x) <= MAX_PATTERN_SIZE;

/** Whether `color` is `#RRGGBB`. */
export const isHexColour = (color: string) => /^#[0-9a-fA-F]{6}$/.test(color);

/** Whether it is more than the first three kinds wrote: a pattern, a gradient or one of their fields (`.kcad` schema 23). */
export function hasDefinition(p: HatchPattern): boolean {
  return p.type === 'pattern' || p.type === 'gradient' || p.name !== undefined || p.scale !== undefined || p.lines !== undefined || p.gradient !== undefined;
}

/** Each kind its own fields only: a pattern's name, scale and families within their bounds, each family's lines apart; a gradient's second colour. */
export function patternProblem(p: HatchPattern): [string, string] | null {
  const kind = p.type;
  const pattern = kind === 'pattern';
  if (!pattern && (p.name !== undefined || p.scale !== undefined || p.lines !== undefined))
    return ['pattern', 'Yalnız desen türündeki taramanın adı, ölçeği ve çizgi aileleri olur.'];
  if (kind !== 'gradient' && p.gradient !== undefined) return ['pattern.gradient', 'Yalnız degrade taramanın ikinci rengi ve biçimi olur.'];
  if ((kind === 'lines' || kind === 'cross') && !(p.spacing > 0 && p.spacing <= MAX_PATTERN_SIZE))
    return ['pattern.spacing', `Taramanın çizgi aralığı ${p.spacing}; sıfırdan büyük ve sonlu olmalı.`];
  if (pattern) {
    const name = p.name ?? '';
    if (name.trim() === '' || [...name].length > MAX_PATTERN_NAME) return ['pattern.name', `Desenin adı boş olamaz ve en çok ${MAX_PATTERN_NAME} harf olabilir.`];
    const scale = p.scale ?? Number.NaN;
    if (!(scale > 0 && scale <= MAX_PATTERN_SIZE)) return ['pattern.scale', `Desenin ölçeği ${Number.isNaN(scale) ? 'NaN' : scale}; sıfırdan büyük ve sonlu olmalı.`];
    const lines = p.lines ?? [];
    if (lines.length === 0 || lines.length > MAX_PATTERN_LINES)
      return ['pattern.lines', `Desenin ${lines.length} çizgi ailesi var; en az 1, en çok ${MAX_PATTERN_LINES} olmalı.`];
    for (const [k, l] of lines.entries()) {
      const n = k + 1;
      const dashes = l.dashes ?? [];
      if (![l.angle, ...l.origin, ...l.offset].every(fits) || !dashes.every(fits))
        return ['pattern.lines', `Desenin ${n}. çizgi ailesinde sonlu olmayan ya da çok büyük bir sayı var.`];
      if (l.offset[1] === 0) return ['pattern.lines', `Desenin ${n}. çizgi ailesinin çizgileri arası 0; aileler sıfırdan büyük aralıklı olmalı.`];
      if (dashes.length > MAX_PATTERN_DASHES) return ['pattern.lines', `Desenin ${n}. çizgi ailesinde ${dashes.length} kesik var; en çok ${MAX_PATTERN_DASHES} olmalı.`];
      if (dashes.length > 0 && dashes.every((d) => d === 0)) return ['pattern.lines', `Desenin ${n}. çizgi ailesinin kesiklerinin hepsi 0; en az birinin uzunluğu olmalı.`];
    }
  }
  if (kind === 'gradient') {
    const g = p.gradient;
    if (!g) return ['pattern.gradient', 'Degrade taramanın ikinci rengi ve biçimi verilmeli.'];
    if (!isHexColour(g.color2)) return ['pattern.gradient.color2', `Degradenin ikinci rengi “${g.color2}”; #RRGGBB biçiminde olmalı.`];
  }
  return null;
}

/** Its seed finite, its lists within their bounds, no object named twice. */
export function assocProblem(a: HatchAssoc): [string, string] | null {
  if (!(Number.isFinite(a.seed.x) && Number.isFinite(a.seed.y))) return ['assoc.seed', 'Taramanın tohum noktası sonlu olmalı.'];
  const named = [a.outer, ...(a.islands ?? []), ...(a.cutouts ?? [])];
  if (named.length - 1 > MAX_ASSOC_OBJECTS) return ['assoc', `Taramanın ilişkisi ${named.length - 1} nesne gösteriyor; en çok ${MAX_ASSOC_OBJECTS} olmalı.`];
  if (new Set(named).size !== named.length) return ['assoc', 'Taramanın ilişkisi bir nesneyi iki kez gösteriyor; her nesne bir kez gösterilmeli.'];
  return null;
}
