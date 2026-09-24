import type { AppContext } from '../../app/context';
import type { Entity } from '../../model/entities';
import type { LayerRenderer, Rule, SymbolSet } from '../../model/style';
import { classesPresent, classLabel, equalCount, equalInterval, numbersOf, plainSymbols, QUALITATIVE, RAMPS, rampColors, uniqueValues, valuesOf } from '../../style/classify';
import { symbolsOfLayerStyle } from '../../style/fromLayer';
import type { GeometryClass } from '../../style/geometry';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { note, segmented } from '../widgets/controls';
import { rulesEditor } from './rulesEditor';
import { symbolSetSlots } from './symbolSlot';

/**
 * Katman stili (docs/STYLE.md §4): how a layer's objects are drawn, as in
 * QGIS's layer styling. Basit is the layer's own colour and line type;
 * Tek sembol gives every object one symbol per geometry; Kategorili picks
 * by an attribute's value, Aralıklı by a number's class; Kurallar by
 * expressions and scale ranges. Classes are made from the data and then
 * edited; nothing reaches the map until Uygula or Tamam.
 */

type Kind = 'simple' | LayerRenderer['type'];

const KINDS: { value: Kind; label: string }[] = [
  { value: 'simple', label: 'Basit' },
  { value: 'single', label: 'Tek sembol' },
  { value: 'categorized', label: 'Kategorili' },
  { value: 'graduated', label: 'Aralıklı' },
  { value: 'rules', label: 'Kurallar' },
];

type Categorized = Extract<LayerRenderer, { type: 'categorized' }>;
type Graduated = Extract<LayerRenderer, { type: 'graduated' }>;

export function openLayerStyle(ctx: AppContext, layerId: string): void {
  const node = ctx.doc.layers.get(layerId);
  if (!node || node.type !== 'layer') {
    ctx.log.warn('Katman stili yalnızca katmanlar için açılır; bir grup seçili.');
    return;
  }
  new LayerStyleDialog(ctx, layerId);
}

class LayerStyleDialog {
  private readonly ctx: AppContext;
  private readonly layerId: string;
  private readonly dialog: Dialog;
  private readonly body: HTMLElement;
  private readonly status: HTMLElement;
  private readonly entities: readonly Entity[];
  private readonly present: Record<GeometryClass, number>;
  private readonly classes: GeometryClass[];
  private kind: Kind;
  private single: SymbolSet;
  private categorized: Categorized;
  private graduated: Graduated;
  private rules: Rule[];
  private gradMethod: 'interval' | 'count' = 'interval';
  private gradCount = 5;
  private ramp = 'sariKirmizi';
  private applied: string;

  constructor(ctx: AppContext, layerId: string) {
    this.ctx = ctx;
    this.layerId = layerId;
    const node = ctx.doc.layers.get(layerId)!;
    this.entities = ctx.doc.byLayer(layerId);
    this.present = classesPresent(this.entities);
    this.classes = (['fill', 'line', 'marker'] as const).filter((c) => this.present[c]);
    if (!this.classes.length) this.classes = ['fill', 'line', 'marker'];
    const current = node.style.renderer;
    this.kind = current?.type ?? 'simple';
    // Every kind keeps its own draft, so switching back and forth loses nothing.
    const simple = symbolsOfLayerStyle(node.style, node.style.color);
    this.single = current?.type === 'single' ? current.symbols : pick(simple, this.classes);
    this.categorized = current?.type === 'categorized' ? current : { type: 'categorized', expr: this.guessField(), categories: [] };
    this.graduated = current?.type === 'graduated' ? current : { type: 'graduated', expr: '$alan', classes: [] };
    this.rules = current?.type === 'rules' ? [...current.rules] : [{ id: 'r1', label: 'Bütün nesneler', symbols: pick(simple, this.classes) }];
    this.applied = JSON.stringify(current ?? null);

    this.body = h('div', { class: 'lsty__body' });
    this.status = h('div', { class: 'lsty__status', role: 'status' });
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    cancel.addEventListener('click', () => this.dialog.close());
    const apply = h('button', { class: 'btn', type: 'button', title: 'Haritada göster, pencere açık kalsın' }, 'Uygula');
    apply.addEventListener('click', () => this.apply());
    const ok = h('button', { class: 'btn btn--primary', type: 'button' }, icon('check', 16), 'Tamam');
    ok.addEventListener('click', () => this.apply() && this.dialog.close());
    this.dialog = new Dialog({
      title: `Katman stili: ${node.name}`,
      width: 980,
      className: 'dialog--lstyle',
      content: [this.body],
      footer: [this.status, h('div', { class: 'dialog__foot-spacer' }), cancel, apply, ok],
    });
    this.render();
  }

  /** The attribute most objects have, a good first guess for categories. */
  private guessField(): string {
    const counts = new Map<string, number>();
    for (const e of this.entities) for (const k of Object.keys(e.attrs)) counts.set(k, (counts.get(k) ?? 0) + 1);
    return [...counts.entries()].sort((a, b) => b[1] - a[1])[0]?.[0] ?? '';
  }

  private fields(): string[] {
    const set = new Set<string>();
    for (const e of this.entities) for (const k of Object.keys(e.attrs)) set.add(k);
    return [...set].sort((a, b) => a.localeCompare(b, 'tr'));
  }

  private renderer(): LayerRenderer | null {
    switch (this.kind) {
      case 'simple':
        return null;
      case 'single':
        return { type: 'single', symbols: this.single };
      case 'categorized':
        return this.categorized;
      case 'graduated':
        return this.graduated;
      case 'rules':
        return { type: 'rules', rules: this.rules };
    }
  }

  private apply(): boolean {
    const r = this.renderer();
    this.ctx.doc.setLayerStyle(this.layerId, { renderer: r ?? undefined }, r ? 'Katman stili' : 'Basit katman stili');
    this.applied = JSON.stringify(r);
    this.say(r ? 'Stil haritaya uygulandı.' : 'Katman basit görünüşüne döndü.');
    return true;
  }

  private say(text: string, kind: 'ok' | 'warn' = 'ok'): void {
    this.status.textContent = text;
    this.status.dataset.kind = kind;
  }

  private layerName = (id: string) => this.ctx.doc.layers.get(id)?.name ?? id;
  /** Expressions read their geometry values from the drawing's geometry store, for the layer's objects at once. */
  private exprScope = { layerName: this.layerName, measures: (list: readonly Entity[]) => this.ctx.view.measures(list.map((e) => e.id)) };

  // ── Rendering ────────────────────────────────────────────────────────

  private render(): void {
    const top = h(
      'div',
      { class: 'lsty__top' },
      segmented({ label: 'İşleyici', options: KINDS, value: this.kind, onChange: (v) => ((this.kind = v), this.render()) }),
      h('span', { class: 'lsty__count' }, `${this.entities.length} nesne: ${[this.present.fill && `${this.present.fill} alan`, this.present.line && `${this.present.line} çizgi`, this.present.marker && `${this.present.marker} nokta`].filter(Boolean).join(', ') || 'çizilecek nesne yok'}`),
    );
    let panel: Child;
    switch (this.kind) {
      case 'simple':
        panel = note('info', h('b', null, 'Katmanın kendi görünüşü. '), 'Renk, çizgi tipi, kalınlık ve dolgu Katmanlar panelinden gelir; nesnelere verilen semboller yine önce gelir.');
        break;
      case 'single':
        panel = h(
          'div',
          { class: 'lsty__single' },
          h('p', { class: 'lsty__help' }, 'Her nesne geometrisine göre bu sembolle çizilir. Sembolü değiştirmek için resmine tıklayın.'),
          symbolSetSlots(this.ctx, this.single, this.classes, (s) => ((this.single = s), this.render()), 'Tek sembol'),
        );
        break;
      case 'categorized':
        panel = this.categorizedPanel();
        break;
      case 'graduated':
        panel = this.graduatedPanel();
        break;
      case 'rules':
        panel = rulesEditor(this.ctx, this.rules, this.classes, this.entities, (r) => ((this.rules = r), this.render()), this.layerName);
        break;
    }
    replaceChildren(this.body, top, panel);
    if (JSON.stringify(this.renderer()) !== this.applied) this.say('Değişiklikler henüz uygulanmadı.', 'warn');
  }

  private exprField(value: string, onChange: (v: string) => void, placeholder: string): { el: HTMLElement; error: HTMLElement } {
    const listId = `lsty-fields-${this.layerId}`;
    const input = h('input', { class: 'field mono lsty__expr', value, list: listId, placeholder, 'aria-label': 'Değer ifadesi', spellcheck: 'false' });
    const error = h('div', { class: 'lsty__error' });
    input.addEventListener('change', () => onChange(input.value.trim()));
    return { el: h('div', { class: 'lsty__exprrow' }, input, h('datalist', { id: listId }, this.fields().map((f) => h('option', { value: /^[\p{L}_][\p{L}\p{N}_]*$/u.test(f) ? f : `"${f}"` })))), error };
  }

  private categorizedPanel(): Child {
    const c = this.categorized;
    const set = (next: Partial<Categorized>) => {
      this.categorized = { ...c, ...next };
      this.render();
    };
    const { values, error } = c.expr ? valuesOf(this.entities, c.expr, this.exprScope) : { values: [] as (string | null)[], error: undefined };
    const counts = new Map(uniqueValues(values).map((v) => [v.value, v.count]));
    const matched = c.categories.reduce((s, k) => s + (counts.get(k.value) ?? 0), 0);
    const expr = this.exprField(c.expr, (v) => set({ expr: v }), 'Alan adı ya da ifade: Nitelik');
    if (error) expr.error.textContent = error;
    const classify = h('button', { class: 'btn btn--small', type: 'button', disabled: !c.expr || !!error }, 'Değerlerden sınıfla');
    classify.addEventListener('click', () => {
      const found = uniqueValues(values);
      if (!found.length) return this.say('Bu ifade nesnelerde değer vermiyor.', 'warn');
      const old = new Map(c.categories.map((k) => [k.value, k]));
      set({ categories: found.map((v, i) => old.get(v.value) ?? { value: v.value, label: v.value, symbols: plainSymbols(QUALITATIVE[i % QUALITATIVE.length], this.present) }) });
      this.say(`${found.length} değer bulundu.`);
    });
    const add = h('button', { class: 'btn btn--small', type: 'button' }, icon('plus', 14), 'Kategori ekle');
    add.addEventListener('click', () => set({ categories: [...c.categories, { value: '', label: 'Yeni kategori', symbols: plainSymbols(QUALITATIVE[c.categories.length % QUALITATIVE.length], this.present) }] }));
    const clear = h('button', { class: 'btn btn--small btn--ghost', type: 'button', disabled: !c.categories.length }, 'Hepsini sil');
    clear.addEventListener('click', () => set({ categories: [] }));
    const rows = c.categories.map((k, i) => {
      const upd = (patch: Partial<Categorized['categories'][number]>) => set({ categories: c.categories.map((x, j) => (j === i ? { ...x, ...patch } : x)) });
      const on = h('input', { type: 'checkbox', checked: k.enabled !== false, 'aria-label': 'Çizilsin' });
      on.addEventListener('change', () => upd({ enabled: on.checked ? undefined : false }));
      const value = h('input', { class: 'field', value: k.value, 'aria-label': 'Değer', spellcheck: 'false' });
      value.addEventListener('change', () => upd({ value: value.value }));
      const label = h('input', { class: 'field', value: k.label, 'aria-label': 'Etiket', spellcheck: 'false' });
      label.addEventListener('change', () => upd({ label: label.value }));
      const del = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Kategoriyi sil' }, icon('trash', 15));
      del.addEventListener('click', () => set({ categories: c.categories.filter((_, j) => j !== i) }));
      return h('tr', null, h('td', null, on), h('td', null, symbolSetSlots(this.ctx, k.symbols, this.classes, (s) => upd({ symbols: s }), k.label || k.value)), h('td', null, value), h('td', null, label), h('td', { class: 'num' }, String(counts.get(k.value) ?? 0)), h('td', null, del));
    });
    const otherOn = h('input', { type: 'checkbox', checked: !!c.other, 'aria-label': 'Diğer değerler çizilsin' });
    otherOn.addEventListener('change', () => set({ other: otherOn.checked ? plainSymbols('#BAB0AC', this.present) : undefined }));
    const rest = this.entities.length - matched;
    return h(
      'div',
      { class: 'lsty__panel' },
      h('div', { class: 'lsty__row' }, h('label', { class: 'lsty__label' }, 'Değer'), expr.el, classify),
      expr.error,
      h(
        'table',
        { class: 'lsty__table' },
        h('thead', null, h('tr', null, h('th', null, ''), h('th', null, 'Sembol'), h('th', null, 'Değer'), h('th', null, 'Etiket'), h('th', { class: 'num' }, 'Nesne'), h('th', null, ''))),
        h(
          'tbody',
          null,
          rows,
          h('tr', { class: 'lsty__other' }, h('td', null, otherOn), h('td', null, c.other ? symbolSetSlots(this.ctx, c.other, this.classes, (s) => set({ other: s }), 'Diğer değerler') : h('span', { class: 'lsty__muted' }, 'çizilmez')), h('td', { colspan: '2' }, 'Diğer değerler'), h('td', { class: 'num' }, String(rest)), h('td', null, '')),
        ),
      ),
      c.categories.length ? null : h('p', { class: 'lsty__help' }, 'Bir alan adı yazıp “Değerlerden sınıfla”ya basın: her farklı değer bir kategori olur.'),
      h('div', { class: 'lsty__tools' }, add, clear),
    );
  }

  private graduatedPanel(): Child {
    const g = this.graduated;
    const set = (next: Partial<Graduated>) => {
      this.graduated = { ...g, ...next };
      this.render();
    };
    const { values: nums, error } = g.expr ? numbersOf(this.entities, g.expr, this.exprScope) : { values: [] as number[], error: undefined };
    const countIn = (min: number, max: number, last: boolean) => nums.filter((v) => v >= min && (v < max || (last && v <= max))).length;
    const expr = this.exprField(g.expr, (v) => set({ expr: v }), 'Sayı veren ifade: $alan, "Kat"');
    if (error) expr.error.textContent = error;
    else if (g.expr && !nums.length) expr.error.textContent = 'Bu ifade nesnelerde sayı vermiyor.';
    const method = h('select', { class: 'field', 'aria-label': 'Yöntem' }, h('option', { value: 'interval', selected: this.gradMethod === 'interval' }, 'Eşit aralık'), h('option', { value: 'count', selected: this.gradMethod === 'count' }, 'Eşit sayı (dilimler)'));
    method.addEventListener('change', () => (this.gradMethod = method.value as 'interval' | 'count'));
    const n = h('input', { class: 'field num lsty__n', value: String(this.gradCount), inputmode: 'numeric', 'aria-label': 'Sınıf sayısı' });
    n.addEventListener('change', () => (this.gradCount = Math.min(20, Math.max(1, Math.round(Number(n.value)) || 5))));
    const ramp = h('select', { class: 'field', 'aria-label': 'Renk rampası' }, Object.entries(RAMPS).map(([k, r]) => h('option', { value: k, selected: k === this.ramp }, r.label)));
    ramp.addEventListener('change', () => (this.ramp = ramp.value));
    const classify = h('button', { class: 'btn btn--small', type: 'button', disabled: !nums.length }, 'Sınıfla');
    classify.addEventListener('click', () => {
      const cls = this.gradMethod === 'interval' ? equalInterval(nums, this.gradCount) : equalCount(nums, this.gradCount);
      const colors = rampColors(RAMPS[this.ramp].stops, cls.length);
      set({ classes: cls.map((c, i) => ({ ...c, label: classLabel(c), symbols: plainSymbols(colors[i], this.present) })) });
      this.say(`${cls.length} sınıf, ${nums.length} sayısal değerden.`);
    });
    const rows = g.classes.map((c, i) => {
      const upd = (patch: Partial<Graduated['classes'][number]>) => set({ classes: g.classes.map((x, j) => (j === i ? { ...x, ...patch } : x)) });
      const numIn = (v: number, key: 'min' | 'max') => {
        const el = h('input', { class: 'field num', value: String(v), inputmode: 'decimal', 'aria-label': key === 'min' ? 'Alt sınır' : 'Üst sınır' });
        el.addEventListener('change', () => {
          const x = Number(el.value.replace(',', '.'));
          if (Number.isFinite(x)) upd({ [key]: x });
        });
        return el;
      };
      const label = h('input', { class: 'field', value: c.label, 'aria-label': 'Etiket', spellcheck: 'false' });
      label.addEventListener('change', () => upd({ label: label.value }));
      const del = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Sınıfı sil' }, icon('trash', 15));
      del.addEventListener('click', () => set({ classes: g.classes.filter((_, j) => j !== i) }));
      return h('tr', null, h('td', null, symbolSetSlots(this.ctx, c.symbols, this.classes, (s) => upd({ symbols: s }), c.label)), h('td', null, numIn(c.min, 'min')), h('td', null, numIn(c.max, 'max')), h('td', null, label), h('td', { class: 'num' }, String(countIn(c.min, c.max, i === g.classes.length - 1))), h('td', null, del));
    });
    return h(
      'div',
      { class: 'lsty__panel' },
      h('div', { class: 'lsty__row' }, h('label', { class: 'lsty__label' }, 'Değer'), expr.el),
      expr.error,
      h('div', { class: 'lsty__row' }, h('label', { class: 'lsty__label' }, 'Yöntem'), method, h('label', { class: 'lsty__label' }, 'Sınıf'), n, h('label', { class: 'lsty__label' }, 'Renkler'), ramp, classify),
      h(
        'table',
        { class: 'lsty__table' },
        h('thead', null, h('tr', null, h('th', null, 'Sembol'), h('th', null, 'Alt (dahil)'), h('th', null, 'Üst'), h('th', null, 'Etiket'), h('th', { class: 'num' }, 'Nesne'), h('th', null, ''))),
        h('tbody', null, rows),
      ),
      g.classes.length ? h('p', { class: 'lsty__help' }, 'Bir değer alt sınıra eşitse o sınıfa girer; son sınıf üst sınırını da içerir.') : h('p', { class: 'lsty__help' }, 'Sayı veren bir ifade yazıp “Sınıfla”ya basın.'),
    );
  }
}

/** Only the classes the layer has. */
function pick(set: SymbolSet, classes: readonly GeometryClass[]): SymbolSet {
  const out: { -readonly [K in GeometryClass]?: SymbolSet[K] } = {};
  for (const c of classes) if (set[c]) out[c] = set[c];
  return out;
}
