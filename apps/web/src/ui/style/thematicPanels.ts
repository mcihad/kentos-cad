import type { AppContext } from '../../app/context';
import type {
  BivariateRenderer,
  ChartRenderer,
  ClusterRenderer,
  DisplacementRenderer,
  DotDensityRenderer,
  HeatmapRenderer,
  InvertedRenderer,
  ProportionalRenderer,
  RendererField,
  SymbolSet,
  UnclassedRenderer,
} from '../../model/style';
import { equalCount, equalInterval, QUALITATIVE, RAMPS } from '../../style/classify';
import type { GeometryClass } from '../../style/geometry';
import { BIVARIATE_SCHEMES, bivariateColors, HEAT_RAMPS } from '../../style/thematic';
import { h, type Child } from '../dom';
import { icon } from '../icons';
import { segmented } from '../widgets/controls';
import { symbolSetSlots, symbolSlot } from './symbolSlot';

/**
 * Katman stili's forms of the thematic renderers (docs/adr/0213 §4): each
 * builds its panel from the draft and gives the next draft to `set`. The
 * window (LayerStyleDialog.ts) holds the drafts, the expression fields and
 * the objects' values; the desktop's twin is apps/desktop/src/style/layer_style/thematic.rs.
 */

/** What the panels take from the window. */
export interface PanelHost {
  ctx: AppContext;
  classes: readonly GeometryClass[];
  present: Record<GeometryClass, number>;
  /** The layer's simple look, what a class without a symbol falls back to. */
  simple: SymbolSet;
  /** An expression field with its error line (Alanlar, Değişkenler, İşlevler). */
  exprField(value: string, onChange: (v: string) => void, placeholder: string): { el: HTMLElement; error: HTMLElement };
  /** The objects' numbers of an expression (null: no value), or why it does not compile. */
  numbers(expr: string): { values: (number | null)[]; error?: string };
  /** The layer's fields, the most used first. */
  fieldNames(): string[];
  say(text: string, kind?: 'ok' | 'warn'): void;
}

const row = (label: string, ...controls: Child[]) => h('div', { class: 'lsty__row' }, h('span', { class: 'lsty__label lsty__label--w' }, label), ...controls);

/** A number field: comma or point, kept when it reads; `min`, `max` bound it. */
function numberField(label: string, value: number | undefined, onChange: (v: number) => void, opts: { min?: number; max?: number; placeholder?: string } = {}): HTMLInputElement {
  const input = h('input', { class: 'field num lsty__n2', value: value === undefined || Number.isNaN(value) ? '' : String(value), inputmode: 'decimal', placeholder: opts.placeholder ?? '', 'aria-label': label, spellcheck: 'false' }) as HTMLInputElement;
  input.addEventListener('change', () => {
    const x = Number(input.value.trim().replace(',', '.'));
    if (input.value.trim() !== '' && Number.isFinite(x) && x >= (opts.min ?? -Infinity) && x <= (opts.max ?? Infinity)) onChange(x);
    else input.value = value === undefined ? '' : String(value);
  });
  return input;
}

/** A ramp's picture: its stops side by side, mixed (data colours, not the theme's). */
function rampStrip(stops: readonly string[]): HTMLElement {
  const css = stops.map((c, i) => `${cssColor(c)} ${((i / Math.max(1, stops.length - 1)) * 100).toFixed(1)}%`).join(', ');
  return h('span', { class: 'lsty__ramp', style: `background: linear-gradient(90deg, ${css}), repeating-conic-gradient(#ccc 0 25%, #fff 0 50%) 0 0 / 8px 8px` });
}

/** `#RRGGBBAA` as CSS takes it. */
function cssColor(c: string): string {
  return /^#[0-9a-f]{8}$/i.test(c) ? `rgba(${parseInt(c.slice(1, 3), 16)}, ${parseInt(c.slice(3, 5), 16)}, ${parseInt(c.slice(5, 7), 16)}, ${(parseInt(c.slice(7, 9), 16) / 255).toFixed(3)})` : c;
}

/** The ramps a renderer may take, by name; `stops` matches one when it is its colours (or reversed). */
function rampSelect(list: Record<string, { label: string; stops: readonly string[] }>, stops: readonly string[], onChange: (s: readonly string[]) => void): Child[] {
  const same = (a: readonly string[], b: readonly string[]) => a.length === b.length && a.every((c, i) => c.toUpperCase() === b[i].toUpperCase());
  const found = Object.entries(list).find(([, r]) => same(r.stops, stops) || same([...r.stops].reverse(), stops));
  const reversed = !!found && !same(found[1].stops, stops);
  const sel = h('select', { class: 'field', 'aria-label': 'Renkler' }, Object.entries(list).map(([k, r]) => h('option', { value: k, selected: found?.[0] === k }, r.label)), found ? null : h('option', { value: '', selected: true }, 'Bu stilin renkleri'));
  sel.addEventListener('change', () => {
    const r = list[sel.value];
    if (r) onChange(reversed ? [...r.stops].reverse() : r.stops);
  });
  const rev = h('input', { type: 'checkbox', checked: reversed, 'aria-label': 'Ters' }) as HTMLInputElement;
  rev.addEventListener('change', () => onChange([...stops].reverse()));
  return [sel, h('label', { class: 'lsty__check' }, rev, 'Ters'), rampStrip(stops)];
}

/** The objects with and without a value of an expression. */
function valuesNote(values: readonly (number | null)[], other: boolean): HTMLElement {
  const n = values.filter((v) => v !== null).length;
  const rest = values.length - n;
  return h('p', { class: 'lsty__help' }, `${n} nesnenin değeri var` + (rest ? `; ${rest} nesnenin yok: ${other ? '“Değeri olmayanlar” sembolüyle çizilir' : 'çizilmez'}.` : '.'));
}

/** The smallest and largest of numbers, or null. */
function rangeOf(values: readonly (number | null)[]): [number, number] | null {
  let lo = Infinity;
  let hi = -Infinity;
  for (const v of values) if (v !== null) (lo = Math.min(lo, v)), (hi = Math.max(hi, v));
  return lo <= hi ? [lo, hi] : null;
}

function otherRow(host: PanelHost, other: SymbolSet | undefined, set: (o: SymbolSet | undefined) => void): HTMLElement {
  const on = h('input', { type: 'checkbox', checked: !!other, 'aria-label': 'Değeri olmayanlar çizilsin' }) as HTMLInputElement;
  on.addEventListener('change', () => set(on.checked ? host.simple : undefined));
  return row('Değeri olmayanlar', h('label', { class: 'lsty__check' }, on, 'Çizilsin'), other ? symbolSetSlots(host.ctx, other, host.classes, set, 'Değeri olmayanlar', host.simple) : null);
}

// ── Sürekli renk ────────────────────────────────────────────────────────

export function unclassedPanel(host: PanelHost, r: UnclassedRenderer, set: (r: UnclassedRenderer) => void): HTMLElement {
  const upd = (p: Partial<UnclassedRenderer>) => set({ ...r, ...p });
  const nums = r.expr ? host.numbers(r.expr) : { values: [] as (number | null)[] };
  const expr = host.exprField(r.expr, (v) => upd({ expr: v }), 'Sayı veren ifade: Nüfus, $alan');
  if (nums.error) expr.error.textContent = nums.error;
  const fromData = h('button', { class: 'btn btn--small', type: 'button', disabled: !rangeOf(nums.values) }, 'Verilerden al');
  fromData.addEventListener('click', () => {
    const m = rangeOf(nums.values);
    if (m) upd({ min: m[0], max: m[1] });
  });
  return h(
    'div',
    { class: 'lsty__panel' },
    h('p', { class: 'lsty__help' }, 'Değer, en küçük ile en büyük arasında rampanın sürekli bir rengine döner; sembolün ana rengi o olur (çerçeveler kalır).'),
    row('Değer', expr.el),
    expr.error,
    row('Aralık', numberField('En küçük', r.min, (v) => upd({ min: v })), h('span', { class: 'lsty__muted' }, '–'), numberField('En büyük', r.max, (v) => upd({ max: v })), fromData),
    row('Renkler', ...rampSelect(RAMPS, r.ramp, (s) => upd({ ramp: s }))),
    row('Sembol', symbolSetSlots(host.ctx, r.symbols, host.classes, (s) => upd({ symbols: s }), 'Sürekli renk', host.simple)),
    otherRow(host, r.other, (o) => upd({ other: o })),
    r.expr && !nums.error ? valuesNote(nums.values, !!r.other) : null,
  );
}

// ── Orantılı sembol ─────────────────────────────────────────────────────

export function proportionalPanel(host: PanelHost, r: ProportionalRenderer, set: (r: ProportionalRenderer) => void): HTMLElement {
  const upd = (p: Partial<ProportionalRenderer>) => set({ ...r, ...p });
  const nums = r.expr ? host.numbers(r.expr) : { values: [] as (number | null)[] };
  const expr = host.exprField(r.expr, (v) => upd({ expr: v }), 'Sayı veren ifade: Nüfus, $alan');
  if (nums.error) expr.error.textContent = nums.error;
  const fromData = h('button', { class: 'btn btn--small', type: 'button', disabled: !rangeOf(nums.values) }, 'Verilerden al');
  fromData.addEventListener('click', () => {
    const m = rangeOf(nums.values);
    if (m) upd({ minValue: m[0], maxValue: m[1] });
  });
  // Points and areas take the marker symbol (an area at its inside point, over its own fill), lines the line symbol.
  const classes: GeometryClass[] = [...(host.present.marker || host.present.fill ? (['marker'] as const) : []), ...(host.present.line ? (['line'] as const) : [])];
  return h(
    'div',
    { class: 'lsty__panel' },
    h('p', { class: 'lsty__help' }, 'Değer sembolün boyunu verir: nokta ve alanın iç noktasında işaretin boyu, çizgide kalınlık.'),
    row('Değer', expr.el),
    expr.error,
    row('Değerler', numberField('En küçük değer', r.minValue, (v) => upd({ minValue: v })), h('span', { class: 'lsty__muted' }, '–'), numberField('En büyük değer', r.maxValue, (v) => upd({ maxValue: v })), fromData),
    row(
      'Boylar',
      numberField('En küçük boy', r.minSize, (v) => upd({ minSize: v }), { min: 0.01, max: 200 }),
      h('span', { class: 'lsty__muted' }, '–'),
      numberField('En büyük boy', r.maxSize, (v) => upd({ maxSize: v }), { min: 0.01, max: 200 }),
      segmented({ label: 'Birim', options: [{ value: 'mm', label: 'mm' }, { value: 'px', label: 'px' }], value: r.unit ?? 'mm', onChange: (v) => upd({ unit: v }) }),
    ),
    row(
      'Ölçekleme',
      segmented({
        label: 'Ölçekleme',
        options: [
          { value: 'area', label: 'Alan', hint: 'Sembolün alanı değerle orantılı (karekök).' },
          { value: 'radius', label: 'Yarıçap', hint: 'Sembolün boyu değerle doğru orantılı.' },
          { value: 'flannery', label: 'Flannery', hint: 'Gözün büyük daireleri küçük görmesine göre düzeltilmiş (üs 0,57).' },
        ],
        value: r.scaling ?? 'area',
        onChange: (v) => upd({ scaling: v }),
      }),
    ),
    row('Sembol', symbolSetSlots(host.ctx, r.symbols, classes.length ? classes : ['marker'], (s) => upd({ symbols: { ...r.symbols, ...s } }), 'Orantılı sembol', host.simple)),
    host.present.fill ? row('Alanın zemini', symbolSetSlots(host.ctx, { fill: r.symbols.fill }, ['fill'], (s) => upd({ symbols: { ...r.symbols, fill: s.fill } }), 'Alanın zemini', host.simple)) : null,
    otherRow(host, r.other, (o) => upd({ other: o })),
    r.expr && !nums.error ? valuesNote(nums.values, !!r.other) : null,
  );
}

// ── İki değişkenli renk ─────────────────────────────────────────────────

/** An axis's breaks from its values: equal count (Aralıklı's slices), else equal interval when values repeat too much. */
function breaksOf(values: readonly number[], n: number, method: 'count' | 'interval'): number[] | null {
  const pick = (cls: { min: number }[]) => cls.slice(1).map((c) => c.min);
  let b = method === 'count' ? pick(equalCount(values, n)) : pick(equalInterval(values, n));
  if (b.length !== n - 1 && method === 'count') b = pick(equalInterval(values, n));
  return b.length === n - 1 ? b : null;
}

export function bivariatePanel(host: PanelHost, r: BivariateRenderer, set: (r: BivariateRenderer) => void, state: { method: 'count' | 'interval'; scheme: string }): HTMLElement {
  const upd = (p: Partial<BivariateRenderer>) => set({ ...r, ...p });
  const n = r.breaksX.length + 1;
  const xs = r.exprX ? host.numbers(r.exprX) : { values: [] as (number | null)[] };
  const ys = r.exprY ? host.numbers(r.exprY) : { values: [] as (number | null)[] };
  const ex = host.exprField(r.exprX, (v) => upd({ exprX: v }), 'Yatay eksenin değeri');
  const ey = host.exprField(r.exprY, (v) => upd({ exprY: v }), 'Düşey eksenin değeri');
  if (xs.error) ex.error.textContent = xs.error;
  if (ys.error) ey.error.textContent = ys.error;
  const classify = (count: number) => {
    const bx = breaksOf(xs.values.filter((v): v is number => v !== null), count, state.method);
    const by = breaksOf(ys.values.filter((v): v is number => v !== null), count, state.method);
    if (!bx || !by) return host.say('Değerler bu kadar sınıfa ayrılamıyor: çoğu aynı. Daha az sınıf deneyin.', 'warn');
    const scheme = BIVARIATE_SCHEMES[state.scheme] ?? Object.values(BIVARIATE_SCHEMES)[0];
    upd({ breaksX: bx, breaksY: by, colors: bivariateColors(scheme.corners, count) });
    host.say(`${count} × ${count} sınıf.`);
  };
  const go = h('button', { class: 'btn btn--small', type: 'button', disabled: !r.exprX || !r.exprY || !!xs.error || !!ys.error }, 'Sınıfla');
  go.addEventListener('click', () => classify(n));
  const scheme = h('select', { class: 'field', 'aria-label': 'Renkler' }, Object.entries(BIVARIATE_SCHEMES).map(([k, s]) => h('option', { value: k, selected: k === state.scheme }, s.label)));
  scheme.addEventListener('change', () => {
    state.scheme = scheme.value;
    const s = BIVARIATE_SCHEMES[scheme.value];
    if (s) upd({ colors: bivariateColors(s.corners, n) });
  });
  // The grid: Y's classes upward, X's rightward, each cell's colour (data colours).
  const grid = h(
    'div',
    { class: 'lsty__grid', style: `grid-template-columns: repeat(${n}, 22px)` },
    Array.from({ length: n * n }, (_, k) => {
      const j = n - 1 - Math.floor(k / n);
      const i = k % n;
      return h('span', { class: 'lsty__cell', style: `background: ${r.colors[j * n + i] ?? 'transparent'}`, title: `${r.exprX || 'X'} ${i + 1}. sınıf, ${r.exprY || 'Y'} ${j + 1}. sınıf` });
    }),
  );
  return h(
    'div',
    { class: 'lsty__panel' },
    h('p', { class: 'lsty__help' }, 'İki değerin sınıfları bir renk ızgarasında bir renge döner: X sağa, Y yukarı artar. Sembolün ana rengi o olur.'),
    row('X değeri', ex.el),
    ex.error,
    row('Y değeri', ey.el),
    ey.error,
    row(
      'Sınıflar',
      segmented({ label: 'Sınıf sayısı', options: [2, 3, 4].map((k) => ({ value: String(k), label: `${k} × ${k}` })), value: String(n), onChange: (v) => classify(Number(v)) }),
      segmented({ label: 'Yöntem', options: [{ value: 'count', label: 'Eşit sayı' }, { value: 'interval', label: 'Eşit aralık' }], value: state.method, onChange: (v) => ((state.method = v), classify(n)) }),
      go,
    ),
    row('Renkler', scheme, grid),
    row('Sınırlar', h('span', { class: 'lsty__muted' }, `X: ${r.breaksX.map(fmt).join(' · ') || '—'}   Y: ${r.breaksY.map(fmt).join(' · ') || '—'}`)),
    row('Sembol', symbolSetSlots(host.ctx, r.symbols, host.classes, (s) => upd({ symbols: s }), 'İki değişkenli renk', host.simple)),
    otherRow(host, r.other, (o) => upd({ other: o })),
  );
}

const fmt = (x: number) => String(Math.round(x * 100) / 100);

// ── Value lists (Nokta yoğunluğu, Grafik) ───────────────────────────────

function fieldsTable(host: PanelHost, fields: readonly RendererField[], set: (f: RendererField[]) => void): HTMLElement {
  const upd = (i: number, p: Partial<RendererField>) => set(fields.map((f, k) => (k === i ? { ...f, ...p } : f)));
  const rows = fields.map((f, i) => {
    const color = h('input', { type: 'color', class: 'lsty__swatch', value: f.color.slice(0, 7), 'aria-label': `${f.label || f.expr} rengi` }) as HTMLInputElement;
    color.addEventListener('change', () => upd(i, { color: color.value.toUpperCase() }));
    const expr = host.exprField(f.expr, (v) => upd(i, { expr: v }), 'Alan adı ya da ifade');
    const label = h('input', { class: 'field', value: f.label ?? '', placeholder: f.expr, 'aria-label': 'Etiket', spellcheck: 'false' }) as HTMLInputElement;
    label.addEventListener('change', () => upd(i, { label: label.value.trim() || undefined }));
    const tool = (name: string, title: string, disabled: boolean, run: () => void) => {
      const b = h('button', { class: 'ibtn', type: 'button', title, 'aria-label': title, disabled }, icon(name, 15));
      b.addEventListener('click', run);
      return b;
    };
    const swap = (a: number, b: number) => set(fields.map((x, k) => (k === a ? fields[b] : k === b ? fields[a] : x)));
    return h(
      'tr',
      null,
      h('td', null, color),
      h('td', { class: 'lsty__exprcell' }, expr.el, expr.error),
      h('td', null, label),
      h(
        'td',
        { class: 'lsty__rowtools' },
        h(
          'div',
          null,
          tool('chevronUp', 'Yukarı', i === 0, () => swap(i, i - 1)),
          tool('chevronDown', 'Aşağı', i === fields.length - 1, () => swap(i, i + 1)),
          tool('trash', 'Sil', fields.length <= 1, () => set(fields.filter((_, k) => k !== i))),
        ),
      ),
    );
  });
  const add = h('button', { class: 'btn btn--small', type: 'button', disabled: fields.length >= 12 }, icon('plus', 14), 'Değer ekle');
  add.addEventListener('click', () => {
    const used = new Set(fields.map((f) => f.expr));
    const next = host.fieldNames().find((n) => !used.has(n)) ?? '';
    set([...fields, { expr: next, color: QUALITATIVE[fields.length % QUALITATIVE.length] }]);
  });
  return h(
    'div',
    { class: 'lsty__fields' },
    h('table', { class: 'lsty__table' }, h('thead', null, h('tr', null, h('th', null, 'Renk'), h('th', null, 'Değer'), h('th', null, 'Etiket'), h('th', null, ''))), h('tbody', null, rows)),
    h('div', { class: 'lsty__tools' }, add),
  );
}

// ── Nokta yoğunluğu ─────────────────────────────────────────────────────

export function dotDensityPanel(host: PanelHost, r: DotDensityRenderer, set: (r: DotDensityRenderer) => void): HTMLElement {
  const upd = (p: Partial<DotDensityRenderer>) => set({ ...r, ...p });
  return h(
    'div',
    { class: 'lsty__panel' },
    h('p', { class: 'lsty__help' }, 'Her alanın içine değeri / nokta değeri kadar nokta rastgele ama hep aynı yerlere konur; her değer kendi renginde.'),
    fieldsTable(host, r.fields, (f) => upd({ fields: f })),
    row(
      'Nokta',
      h('span', { class: 'lsty__muted' }, '1 nokta ='),
      numberField('Nokta değeri', r.dotValue, (v) => upd({ dotValue: v }), { min: 1e-9 }),
      h('span', { class: 'lsty__label' }, 'Boy'),
      numberField('Nokta boyu', r.dotSize ?? 1, (v) => upd({ dotSize: v }), { min: 0.01, max: 20 }),
      segmented({ label: 'Birim', options: [{ value: 'mm', label: 'mm' }, { value: 'px', label: 'px' }], value: r.unit ?? 'mm', onChange: (v) => upd({ unit: v }) }),
    ),
    row('Yerleşim', h('span', { class: 'lsty__label' }, 'Tohum'), numberField('Tohum', r.seed ?? 0, (v) => upd({ seed: Math.round(v) }), { min: 0, max: 2147483647 }), h('span', { class: 'lsty__muted' }, 'Başka bir sayı noktaları başka yerlere koyar.')),
    row('Zemin', symbolSetSlots(host.ctx, r.symbols ?? {}, host.classes, (s) => upd({ symbols: s }), 'Zemin', host.simple)),
  );
}

// ── Grafik ──────────────────────────────────────────────────────────────

export function chartPanel(host: PanelHost, r: ChartRenderer, set: (r: ChartRenderer) => void): HTMLElement {
  const upd = (p: Partial<ChartRenderer>) => set({ ...r, ...p });
  const kind = r.kind ?? 'pie';
  // The largest total (a pie's) or value (bars'), for Verilerden al.
  const largest = () => {
    const cols = r.fields.map((f) => host.numbers(f.expr).values);
    let top = 0;
    let lo = Infinity;
    for (let i = 0; i < (cols[0]?.length ?? 0); i++) {
      const vals = cols.map((c) => Math.max(0, c[i] ?? 0));
      const v = kind === 'bar' ? Math.max(...vals) : vals.reduce((a, b) => a + b, 0);
      top = Math.max(top, v);
      if (v > 0) lo = Math.min(lo, v);
    }
    return { top, lo: Number.isFinite(lo) ? lo : 0 };
  };
  const fromData = h('button', { class: 'btn btn--small', type: 'button' }, 'Verilerden al');
  fromData.addEventListener('click', () => {
    const { top, lo } = largest();
    if (!(top > 0)) return host.say('Değerlerin hiçbiri sıfırdan büyük değil.', 'warn');
    if (kind === 'pie') upd({ sizeBy: { minValue: lo, maxValue: top, minSize: r.sizeBy?.minSize ?? r.size / 2, maxSize: r.sizeBy?.maxSize ?? r.size } });
    else upd({ maxValue: top });
  });
  const sizeByOn = h('input', { type: 'checkbox', checked: !!r.sizeBy, 'aria-label': 'Toplama göre boy' }) as HTMLInputElement;
  sizeByOn.addEventListener('change', () => {
    if (!sizeByOn.checked) return upd({ sizeBy: undefined });
    const { top, lo } = largest();
    upd({ sizeBy: { minValue: lo, maxValue: Math.max(top, lo), minSize: r.size / 2, maxSize: r.size } });
  });
  const outlineOn = h('input', { type: 'checkbox', checked: !!r.outline, 'aria-label': 'Çerçeve' }) as HTMLInputElement;
  outlineOn.addEventListener('change', () => upd({ outline: outlineOn.checked ? { color: '#FFFFFF', width: 0.2 } : undefined }));
  const outlineColor = h('input', { type: 'color', class: 'lsty__swatch', value: (r.outline?.color ?? '#FFFFFF').slice(0, 7), 'aria-label': 'Çerçevenin rengi', disabled: !r.outline }) as HTMLInputElement;
  outlineColor.addEventListener('change', () => r.outline && upd({ outline: { ...r.outline, color: outlineColor.value.toUpperCase() } }));
  const sb = r.sizeBy;
  return h(
    'div',
    { class: 'lsty__panel' },
    h('p', { class: 'lsty__help' }, 'Her nesnenin üstüne (alanın iç noktasına, çizginin ortasına) değerlerinin grafiği çizilir.'),
    row(
      'Tür',
      segmented({
        label: 'Grafik türü',
        options: [
          { value: 'pie', label: 'Pasta' },
          { value: 'bar', label: 'Çubuk' },
          { value: 'stacked', label: 'Yığılmış çubuk' },
        ],
        value: kind,
        onChange: (v) => upd({ kind: v, ...(v !== 'pie' && !r.maxValue && { maxValue: largest().top || 1 }) }),
      }),
    ),
    fieldsTable(host, r.fields, (f) => upd({ fields: f })),
    row(
      kind === 'pie' ? 'Çap' : 'Yükseklik',
      numberField('Boy', r.size, (v) => upd({ size: v }), { min: 0.01, max: 200 }),
      segmented({ label: 'Birim', options: [{ value: 'mm', label: 'mm' }, { value: 'px', label: 'px' }], value: r.unit ?? 'mm', onChange: (v) => upd({ unit: v }) }),
      kind === 'pie' ? h('label', { class: 'lsty__check' }, sizeByOn, 'Toplama göre') : null,
      kind !== 'pie' ? h('span', { class: 'lsty__label' }, 'Genişlik') : null,
      kind !== 'pie' ? numberField('Çubuk genişliği', r.barWidth ?? r.size / 4, (v) => upd({ barWidth: v }), { min: 0.01, max: 50 }) : null,
    ),
    kind !== 'pie'
      ? row('En büyük değer', numberField('En büyük değer', r.maxValue, (v) => upd({ maxValue: v }), { min: 1e-9 }), h('span', { class: 'lsty__muted' }, `bu değer ${r.size} ${r.unit ?? 'mm'} yüksekliktir`), fromData)
      : sb
        ? row(
            'Toplam',
            numberField('En küçük toplam', sb.minValue, (v) => upd({ sizeBy: { ...sb, minValue: v } })),
            h('span', { class: 'lsty__muted' }, '–'),
            numberField('En büyük toplam', sb.maxValue, (v) => upd({ sizeBy: { ...sb, maxValue: v } })),
            h('span', { class: 'lsty__label' }, 'Çap'),
            numberField('En küçük çap', sb.minSize, (v) => upd({ sizeBy: { ...sb, minSize: v } }), { min: 0.01, max: 200 }),
            h('span', { class: 'lsty__muted' }, '–'),
            numberField('En büyük çap', sb.maxSize, (v) => upd({ sizeBy: { ...sb, maxSize: v } }), { min: 0.01, max: 200 }),
            fromData,
          )
        : null,
    row(
      'Çerçeve',
      h('label', { class: 'lsty__check' }, outlineOn, 'Çiz'),
      outlineColor,
      numberField('Çerçevenin kalınlığı', r.outline?.width ?? 0.2, (v) => r.outline && upd({ outline: { ...r.outline, width: v } }), { min: 0, max: 10 }),
    ),
    row('Zemin', symbolSetSlots(host.ctx, r.symbols ?? {}, host.classes, (s) => upd({ symbols: s }), 'Zemin', host.simple)),
  );
}

// ── Isı haritası ────────────────────────────────────────────────────────

export function heatmapPanel(host: PanelHost, r: HeatmapRenderer, set: (r: HeatmapRenderer) => void): HTMLElement {
  const upd = (p: Partial<HeatmapRenderer>) => set({ ...r, ...p });
  const weight = host.exprField(r.weight ?? '', (v) => upd({ weight: v || undefined }), 'Boş: her nokta bir');
  if (r.weight) {
    const nums = host.numbers(r.weight);
    if (nums.error) weight.error.textContent = nums.error;
  }
  const opacity = h('input', { type: 'range', class: 'lsty__range', min: '0', max: '100', value: String(Math.round((r.opacity ?? 1) * 100)), 'aria-label': 'Opaklık' }) as HTMLInputElement;
  opacity.addEventListener('change', () => upd({ opacity: Number(opacity.value) / 100 }));
  return h(
    'div',
    { class: 'lsty__panel' },
    h('p', { class: 'lsty__help' }, 'Noktaların yoğunluğu, görünümün renkli resmi olur. Yalnız noktalar sayılır; katmanın öbür nesneleri çizilmez.'),
    row(
      'Yarıçap',
      numberField('Yarıçap', r.radius, (v) => upd({ radius: v }), { min: 0.01, max: r.unit === 'm' ? Infinity : 500 }),
      segmented({ label: 'Birim', options: [{ value: 'px', label: 'px' }, { value: 'm', label: 'm' }], value: r.unit ?? 'px', onChange: (v) => upd({ unit: v }) }),
    ),
    row('Ağırlık', weight.el),
    weight.error,
    row(
      'En büyük değer',
      segmented({
        label: 'En büyük değer',
        options: [
          { value: 'dynamic', label: 'Dinamik', hint: 'Görünümdeki en yoğun yer en koyu renk.' },
          { value: 'fixed', label: 'Sabit', hint: 'Bu değer ve üstü en koyu renk; renkler yakınlaşınca değişmez.' },
        ],
        value: r.max === undefined ? 'dynamic' : 'fixed',
        onChange: (v) => upd({ max: v === 'dynamic' ? undefined : (r.max ?? 10) }),
      }),
      r.max !== undefined ? numberField('En büyük değer', r.max, (v) => upd({ max: v }), { min: 1e-9 }) : null,
    ),
    row('Renkler', ...rampSelect(HEAT_RAMPS, r.ramp, (s) => upd({ ramp: s }))),
    row(
      'Kalite',
      segmented({ label: 'Kalite', options: [1, 2, 3, 4, 5].map((q) => ({ value: String(q), label: q === 1 ? '1 (en iyi)' : String(q), hint: `Hücre ${q} piksel` })), value: String(r.quality ?? 2), onChange: (v) => upd({ quality: Number(v) }) }),
    ),
    row('Opaklık', opacity, h('span', { class: 'lsty__muted' }, `%${Math.round((r.opacity ?? 1) * 100)}`)),
  );
}

// ── Kümeleme ve Yayma ───────────────────────────────────────────────────

/** The renderer a cluster or a displacement draws single points with: its own tab's draft. */
export type InnerKind = 'simple' | 'single' | 'categorized' | 'graduated' | 'rules' | 'unclassed' | 'proportional' | 'bivariate';

export const INNER_KINDS: { value: InnerKind; label: string }[] = [
  { value: 'simple', label: 'Basit görünüş' },
  { value: 'single', label: 'Tek sembol' },
  { value: 'categorized', label: 'Kategorili' },
  { value: 'graduated', label: 'Aralıklı' },
  { value: 'unclassed', label: 'Sürekli renk' },
  { value: 'proportional', label: 'Orantılı sembol' },
  { value: 'bivariate', label: 'İki değişkenli renk' },
  { value: 'rules', label: 'Kurallar' },
];

function innerRow(inner: InnerKind, onChange: (k: InnerKind) => void): HTMLElement {
  const sel = h('select', { class: 'field', 'aria-label': 'Tek noktalar' }, INNER_KINDS.map((k) => h('option', { value: k.value, selected: k.value === inner }, k.label))) as HTMLSelectElement;
  sel.addEventListener('change', () => onChange(sel.value as InnerKind));
  return row('Tek noktalar', sel, h('span', { class: 'lsty__muted' }, inner === 'simple' ? 'katmanın görünüşü' : 'ayarları kendi türünün sayfasında'));
}

export function clusterPanel(host: PanelHost, r: ClusterRenderer, set: (r: ClusterRenderer) => void, inner: InnerKind, setInner: (k: InnerKind) => void): HTMLElement {
  const upd = (p: Partial<ClusterRenderer>) => set({ ...r, ...p });
  const check = (label: string, on: boolean, f: (v: boolean) => void) => {
    const c = h('input', { type: 'checkbox', checked: on, 'aria-label': label }) as HTMLInputElement;
    c.addEventListener('change', () => f(c.checked));
    return h('label', { class: 'lsty__check' }, c, label);
  };
  return h(
    'div',
    { class: 'lsty__panel' },
    h('p', { class: 'lsty__help' }, 'Ekranda birbirine bu uzaklıktan yakın noktalar tek bir küme işareti olur; sayıları içinde yazar. Yakınlaştıkça kümeler açılır.'),
    row(
      'Uzaklık',
      numberField('Uzaklık', r.distance, (v) => upd({ distance: v }), { min: 0.01, max: r.unit === 'm' ? Infinity : 500 }),
      segmented({ label: 'Birim', options: [{ value: 'px', label: 'px' }, { value: 'm', label: 'm' }], value: r.unit ?? 'px', onChange: (v) => upd({ unit: v }) }),
    ),
    row('Küme işareti', symbolSlot(host.ctx, r.symbol, 'marker', (s) => upd({ symbol: s }), 'Küme işareti'), h('span', { class: 'lsty__muted' }, r.symbol ? '' : 'katmanın renginde daire')),
    row('Seçenekler', check('Sayısını yaz', r.count !== false, (v) => upd({ count: v ? undefined : false })), check('Sayısıyla büyüsün', !!r.grow, (v) => upd({ grow: v || undefined }))),
    innerRow(inner, setInner),
  );
}

export function displacementPanel(host: PanelHost, r: DisplacementRenderer, set: (r: DisplacementRenderer) => void, inner: InnerKind, setInner: (k: InnerKind) => void): HTMLElement {
  const upd = (p: Partial<DisplacementRenderer>) => set({ ...r, ...p });
  const circleOn = h('input', { type: 'checkbox', checked: !!r.circle, 'aria-label': 'Halkayı çiz' }) as HTMLInputElement;
  circleOn.addEventListener('change', () => upd({ circle: circleOn.checked ? { color: '#7D7D7D', width: 1 } : undefined }));
  const circleColor = h('input', { type: 'color', class: 'lsty__swatch', value: (r.circle?.color ?? '#7D7D7D').slice(0, 7), 'aria-label': 'Halkanın rengi', disabled: !r.circle }) as HTMLInputElement;
  circleColor.addEventListener('change', () => r.circle && upd({ circle: { ...r.circle, color: circleColor.value.toUpperCase() } }));
  return h(
    'div',
    { class: 'lsty__panel' },
    h('p', { class: 'lsty__help' }, 'Üst üste binen noktalar ortalarının çevresine dağıtılır; her biri kendi sembolüyle görünür.'),
    row(
      'Tolerans',
      numberField('Tolerans', r.tolerance, (v) => upd({ tolerance: v }), { min: 0.01, max: r.unit === 'm' ? Infinity : 100 }),
      segmented({ label: 'Birim', options: [{ value: 'px', label: 'px' }, { value: 'm', label: 'm' }], value: r.unit ?? 'px', onChange: (v) => upd({ unit: v }) }),
    ),
    row(
      'Yerleşim',
      segmented({
        label: 'Yerleşim',
        options: [
          { value: 'ring', label: 'Halka' },
          { value: 'rings', label: 'İç içe halkalar' },
          { value: 'grid', label: 'Izgara' },
        ],
        value: r.placement ?? 'ring',
        onChange: (v) => upd({ placement: v }),
      }),
      h('span', { class: 'lsty__label' }, 'Aralık'),
      numberField('Aralık', r.spacing ?? 0, (v) => upd({ spacing: v }), { min: 0, max: 100 }),
      h('span', { class: 'lsty__muted' }, 'px'),
    ),
    row('Merkez işareti', symbolSlot(host.ctx, r.center, 'marker', (s) => upd({ center: s }), 'Merkez işareti')),
    row('Halka', h('label', { class: 'lsty__check' }, circleOn, 'Çiz'), circleColor, numberField('Halkanın kalınlığı', r.circle?.width ?? 1, (v) => r.circle && upd({ circle: { ...r.circle, width: v } }), { min: 0, max: 10 }), h('span', { class: 'lsty__muted' }, 'px')),
    innerRow(inner, setInner),
  );
}

// ── Ters alan ───────────────────────────────────────────────────────────

export function invertedPanel(host: PanelHost, r: InvertedRenderer, set: (r: InvertedRenderer) => void): HTMLElement {
  const merge = h('input', { type: 'checkbox', checked: !!r.merge, 'aria-label': 'Örtüşenler boş kalsın' }) as HTMLInputElement;
  merge.addEventListener('change', () => set({ ...r, merge: merge.checked || undefined }));
  return h(
    'div',
    { class: 'lsty__panel' },
    h('p', { class: 'lsty__help' }, 'Katmanın alanlarının dışı dolgu sembolüyle boyanır; alanlar boş kalır, kenarları sembolün çizgileriyle çizilir. Çalışma alanının dışını örtmek için.'),
    row('Dolgu', symbolSetSlots(host.ctx, { fill: r.symbols.fill }, ['fill'], (s) => set({ ...r, symbols: { fill: s.fill } }), 'Ters alan', host.simple)),
    row('Örtüşenler', h('label', { class: 'lsty__check' }, merge, 'Boş kalsın'), h('span', { class: 'lsty__muted' }, 'kapalıyken iki alanın örtüştüğü yer yeniden boyanır')),
  );
}
