import type { LabelPin } from '../contracts/generated/LabelPin';
import type { LabelStyle } from '../contracts/generated/LabelStyle';
import type { LayerLabels } from '../contracts/generated/LayerLabels';
import { trimName } from './blocks';
import type { LayerNode } from './layers';

/**
 * The label engine's rules (docs/adr/0212 §2), the web's copy of `kentos_contracts::labels`: the readers', the
 * commands' and the server's words, letter for letter. Whether an expression compiles is the command's
 * (`model/labelTexts.ts`).
 */

/** The most classes a layer's labelling has. */
export const LABEL_CLASSES_MAX = 64;
/** The longest expression (a class's text or condition), in characters. */
export const LABEL_EXPRESSION_MAX = 10_000;
/** The longest name (a class's), in characters. */
export const LABEL_NAME_MAX = 100;
/** The most words an abbreviation dictionary holds. */
export const LABEL_WORDS_MAX = 500;
/** The most pins an object holds. */
export const LABEL_PINS_MAX = 64;

const THEME_COLORS = new Set(['fg', 'fg-dim', 'label', 'ink', 'paper']);

/** Whether a colour is one a label may name: `#rrggbb`, `#rrggbbaa` or a theme name. */
export function labelColorOk(c: string): boolean {
  if (THEME_COLORS.has(c)) return true;
  return /^#([0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(c);
}

const letters = (s: string): number => [...s].length;

function number(what: string, x: number, lo: number, hi: number): string | null {
  if (!Number.isFinite(x)) return `${what} sayı değil`;
  return x < lo || x > hi ? `${what} ${lo}–${hi} arasında olmalı` : null;
}

function optional(what: string, x: number | undefined, lo: number, hi: number): string | null {
  return x === undefined ? null : number(what, x, lo, hi);
}

function color(what: string, c: string | undefined): string | null {
  return c !== undefined && !labelColorOk(c) ? `${what} “${c}” renk değil (#rrggbb ya da fg, fg-dim, label, ink, paper)` : null;
}

function expression(what: string, e: string | undefined): string | null {
  if (e === undefined) return null;
  if (trimName(e) !== e || !e) return `${what} boş ya da başında veya sonunda boşluk var`;
  return letters(e) > LABEL_EXPRESSION_MAX ? `${what} ${LABEL_EXPRESSION_MAX} karakterden uzun` : null;
}

/** Rust's `char::is_whitespace`: Unicode's White_Space (JavaScript's `\s` differs). */
function isWhiteSpace(ch: string): boolean {
  const c = ch.codePointAt(0) ?? 0;
  return (
    (c >= 0x09 && c <= 0x0d) || c === 0x20 || c === 0x85 || c === 0xa0 || c === 0x1680 || (c >= 0x2000 && c <= 0x200a) ||
    c === 0x2028 || c === 0x2029 || c === 0x202f || c === 0x205f || c === 0x3000
  );
}

/** What is wrong with a label style, when anything is (`style_problem`). */
export function labelStyleProblem(s: LabelStyle): string | null {
  const checks: (() => string | null)[] = [
    () => number('boy', s.size, 1, 200),
    () => optional('büyüme', s.grow, 0, 1000),
    () => optional('en büyük boy', s.maxSize, 1, 200),
    () => (s.weight !== undefined && !(s.weight >= 100 && s.weight <= 900) ? `kalınlık ${s.weight}: 100–900 olmalı` : null),
    () => optional('en küçük nesne', s.minFeaturePx, 0, 100_000),
    () => optional('en küçük ölçek', s.minScale, 0, 1e9),
    () => optional('en büyük ölçek', s.maxScale, 0, 1e9),
    () => expression('metnin ifadesi', s.text),
    () => color('yazının rengi', s.color),
    () => optional('uzaklık', s.distance, 0, 500),
    () => optional('yineleme', s.repeat, 20, 100_000),
    () => optional('harfler arası en büyük açı', s.maxAngle, 5, 90),
    () => optional('küçültme', s.shrink, 0.5, 1),
    () => (s.priority !== undefined && s.priority > 10 ? `öncelik ${s.priority}: 0–10 arasında olmalı` : null),
    () => optional('yinelenenlerin uzaklığı', s.duplicates, 1, 10_000),
    () => (s.halo ? (number('halenin genişliği', s.halo.width, 0, 10) ?? color('halenin rengi', s.halo.color)) : null),
    () => {
      const b = s.background;
      if (!b) return null;
      return color('zeminin dolgusu', b.fill) ?? color('zeminin çizgisi', b.stroke) ?? optional('zeminin payı', b.padding, 0, 50);
    },
    () => {
      const h = s.shadow;
      if (!h) return null;
      return (
        number('gölgenin kayması', h.dx, -50, 50) ??
        number('gölgenin kayması', h.dy, -50, 50) ??
        color('gölgenin rengi', h.color) ??
        optional('gölgenin opaklığı', h.opacity, 0, 1)
      );
    },
    () => {
      const c = s.callout;
      if (!c) return null;
      return color('çağrı çizgisinin rengi', c.color) ?? optional('çağrı çizgisinin kalınlığı', c.width, 0.1, 10) ?? optional('çağrı çizgisinin en kısası', c.minLength, 0, 1000);
    },
    () => {
      const k = s.stack;
      if (!k) return null;
      if (!(k.chars >= 2 && k.chars <= 500)) return `yığmanın satırı ${k.chars} harf: 2–500 olmalı`;
      if (k.at === undefined) return null;
      return !k.at || letters(k.at) > 20 ? 'yığmanın bölme karakterleri 1–20 harf olmalı' : null;
    },
    () => {
      const a = s.abbreviate;
      if (!a) return null;
      if (!a.words.length || a.words.length > LABEL_WORDS_MAX) return `kısaltma sözlüğü 1–${LABEL_WORDS_MAX} sözcük olmalı`;
      const bad = (t: string): boolean => !t || letters(t) > 100 || [...t].some(isWhiteSpace);
      for (const [i, w] of a.words.entries()) {
        if (bad(w.word) || bad(w.short)) return `kısaltma sözlüğünün ${i + 1}. satırı: sözcük ve kısası 1–100 harf, boşluksuz olmalı`;
        if (a.words.slice(0, i).some((v) => v.word === w.word)) return `kısaltma sözlüğünde “${w.word}” iki kez var`;
      }
      return null;
    },
  ];
  for (const check of checks) {
    const p = check();
    if (p) return p;
  }
  return null;
}

/** What is wrong with a layer's labelling, when anything is (`layer_labels_problem`). */
export function layerLabelsProblem(l: LayerLabels): string | null {
  const classes = l.classes ?? [];
  if (l.mode === 'rules') {
    if (!classes.length || classes.length > LABEL_CLASSES_MAX) return `kurallı etiketlemede 1–${LABEL_CLASSES_MAX} sınıf olmalı`;
  } else if (classes.length) {
    return 'sınıflar yalnız kurallı etiketlemede olur';
  }
  for (const [i, c] of classes.entries()) {
    const name = trimName(c.name);
    if (!name || name !== c.name || letters(name) > LABEL_NAME_MAX)
      return `${i + 1}. sınıfın adı boş, ${LABEL_NAME_MAX} harften uzun ya da başında veya sonunda boşluk var`;
    if (classes.slice(0, i).some((d) => d.name === c.name)) return `“${name}” adlı sınıf iki kez var`;
    const p = expression('koşul', c.when) ?? labelStyleProblem(c.style);
    if (p) return `“${name}” sınıfı: ${p}`;
  }
  const o = l.obstacle;
  return o && !(o.weight >= 1 && o.weight <= 10) ? `engelin ağırlığı ${o.weight}: 1–10 olmalı` : null;
}

/** What is wrong with an object's pins, when anything is (`pins_problem`). */
export function labelPinsProblem(pins: readonly LabelPin[]): string | null {
  if (pins.length > LABEL_PINS_MAX) return `nesnenin ${pins.length} etiket iğnesi var; en çok ${LABEL_PINS_MAX}`;
  for (const [i, p] of pins.entries()) {
    const c = p.class;
    if (c !== undefined && (trimName(c) !== c || !c || letters(c) > LABEL_NAME_MAX)) return `etiket iğnesinin sınıfı “${c}” geçerli bir ad değil`;
    if (pins.slice(0, i).some((q) => q.class === p.class)) return 'aynı sınıfın iki etiket iğnesi var';
    if (p.at === undefined && p.rotation !== undefined) return 'etiket iğnesinin açısı yerle birlikte verilir';
    if (p.at === undefined && p.hidden !== true) return 'etiket iğnesi ya bir yer ya da gizli olur';
    if (p.hidden === false) return "etiket iğnesinin hidden'ı yalnız true yazılır";
    const a = p.at;
    if (a && (!Number.isFinite(a.x) || !Number.isFinite(a.y) || Math.abs(a.x) > 1e7 || Math.abs(a.y) > 1e7))
      return "etiket iğnesinin yeri sonlu ve 10 000 km'den yakın olmalı";
    const r = p.rotation;
    if (r !== undefined && (!Number.isFinite(r) || Math.abs(r) > 360)) return 'etiket iğnesinin açısı −360–360 derece olmalı';
  }
  return null;
}

const ENGINE_FIELDS = [
  'text', 'color', 'italic', 'align', 'point', 'line', 'area', 'position', 'distance', 'repeat', 'maxAngle', 'curved', 'mergeLines', 'inside',
  'outside', 'halo', 'background', 'shadow', 'callout', 'stack', 'abbreviate', 'shrink', 'priority', 'overlap', 'duplicates',
] as const;

/** Whether a style has any of the engine's fields: only such a style is checked whole (`has_engine_fields`). */
export function hasEngineFields(s: LabelStyle): boolean {
  return ENGINE_FIELDS.some((k) => s[k] !== undefined);
}

/** What is wrong with the tree's labelling, when anything is (`labels_problem`). */
export function labelsProblem(tree: readonly LayerNode[]): string | null {
  for (const n of tree) {
    const s = n.style;
    if (s.labels && n.type === 'group') return `“${n.name}” bir grup; grubun etiketlemesi olmaz, etiketleme katmanındır`;
    const label = s.label && hasEngineFields(s.label) ? labelStyleProblem(s.label) : null;
    if (label) return `“${n.name}” katmanının etiketi: ${label}`;
    const labels = s.labels ? layerLabelsProblem(s.labels) : null;
    if (labels) return `“${n.name}” katmanının etiketlemesi: ${labels}`;
    const p = labelsProblem(n.children);
    if (p) return p;
  }
  return null;
}
