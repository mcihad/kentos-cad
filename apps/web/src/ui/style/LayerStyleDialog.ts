import type { AppContext } from '../../app/context';
import type { Entity } from '../../model/entities';
import { compileExpression, expressionError } from '../../model/expression/expression';
import { exprCatalog } from '../../model/expression/expressionLib';
import type {
  BivariateRenderer,
  ChartRenderer,
  ClusterRenderer,
  DisplacementRenderer,
  DotDensityRenderer,
  HeatmapRenderer,
  InnerRenderer,
  InvertedRenderer,
  LayerRenderer,
  ProportionalRenderer,
  Rule,
  Symbol,
  SymbolSet,
  UnclassedRenderer,
} from '../../model/style';
import {
  categoriesOf,
  classCount,
  classesPresent,
  CLASSIFY_TEXTS,
  DEFAULT_RAMP,
  CLASS_COUNT,
  graduatedOf,
  newCategory,
  OTHER_COLOR,
  plainSymbols,
  QUALITATIVE,
  RAMPS,
  uniqueValues,
  valuesOf,
} from '../../style/classify';
import { symbolsOfLayerStyle } from '../../style/fromLayer';
import type { GeometryClass } from '../../style/geometry';
import { BIVARIATE_SCHEMES, bivariateColors, HEAT_RAMPS } from '../../style/thematic';
import { layersRenderer } from '../../product/layersRenderer';
import { rendererProblem } from '../../wasm/core';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { fieldToken } from '../processing/fieldPlan';
import { Dialog } from '../widgets/Dialog';
import { Dropdown } from '../widgets/Dropdown';
import { askUnsaved } from '../widgets/confirm';
import { note } from '../widgets/controls';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { rulesEditor } from './rulesEditor';
import { symbolSetSlots } from './symbolSlot';
import { categoryTally, classTally, drawable, numbersPerObject, shadowed } from './tally';
import {
  bivariatePanel,
  chartPanel,
  clusterPanel,
  displacementPanel,
  dotDensityPanel,
  heatmapPanel,
  invertedPanel,
  proportionalPanel,
  unclassedPanel,
  type InnerKind,
  type PanelHost,
} from './thematicPanels';
import { drawNow } from './thumbs';

/**
 * Katman stili (docs/STYLE.md §4): how a layer's objects are drawn, as in
 * QGIS's layer styling. Basit is the layer's own colour and line type;
 * Tek sembol gives every object one symbol per geometry; Kategorili picks
 * by an attribute's value, Aralıklı by a number's class; Kurallar by
 * expressions and scale ranges. Classes are made from the data and then
 * edited; nothing reaches the map until Uygula or Tamam. The Nesne column
 * says what each category, class and rule will draw (./tally.ts, held to
 * fixtures/style/v1/tally.json with the desktop's window).
 */

type Kind = 'simple' | LayerRenderer['type'];

/** The renderers in the list, by group (docs/adr/0213 §4): their names, icons and what each does. */
const KINDS: { value: Kind; label: string; icon: string; detail: string; group?: string }[] = [
  { value: 'simple', label: 'Basit', icon: 'rendererSimple', detail: 'Katmanın kendi rengi, çizgi tipi ve dolgusu.' },
  { value: 'single', label: 'Tek sembol', icon: 'rendererSingle', detail: 'Her nesne aynı sembolle.' },
  { value: 'categorized', label: 'Kategorili', icon: 'rendererCategorized', detail: 'Değere göre bir sembol.', group: 'Değere göre' },
  { value: 'graduated', label: 'Aralıklı', icon: 'rendererGraduated', detail: 'Sayının sınıfına göre bir sembol.' },
  { value: 'unclassed', label: 'Sürekli renk', icon: 'rendererUnclassed', detail: 'Sayı, rampada sürekli bir renk.' },
  { value: 'proportional', label: 'Orantılı sembol', icon: 'rendererProportional', detail: 'Sayı, sembolün boyu.' },
  { value: 'bivariate', label: 'İki değişkenli renk', icon: 'rendererBivariate', detail: 'İki sayının sınıfları, bir renk ızgarası.' },
  { value: 'rules', label: 'Kurallar', icon: 'rendererRules', detail: 'İfadeler ve ölçek aralıklarıyla.', group: 'Kurallar' },
  { value: 'dotDensity', label: 'Nokta yoğunluğu', icon: 'rendererDotDensity', detail: 'Alanın içinde değerle orantılı noktalar.', group: 'Tematik' },
  { value: 'chart', label: 'Grafik', icon: 'rendererChart', detail: 'Nesnenin üstünde pasta ya da çubuk.' },
  { value: 'heatmap', label: 'Isı haritası', icon: 'rendererHeatmap', detail: 'Noktaların yoğunluğu, renkli resim.' },
  { value: 'cluster', label: 'Kümeleme', icon: 'rendererCluster', detail: 'Yakın noktalar tek işaret ve sayısı.', group: 'Noktalar' },
  { value: 'displacement', label: 'Yayma', icon: 'rendererDisplacement', detail: 'Üst üste binen noktalar çevreye dağılır.' },
  { value: 'inverted', label: 'Ters alan', icon: 'rendererInverted', detail: 'Alanların dışı boyanır.', group: 'Alanlar' },
];

type Categorized = Extract<LayerRenderer, { type: 'categorized' }>;
type Graduated = Extract<LayerRenderer, { type: 'graduated' }>;

/** A marker the thematic renderers start with: a circle in the layer's colour. */
const circleOf = (color: string): Symbol => ({ type: 'marker', layers: [{ id: 'c', type: 'shape', shape: 'circle', size: 4, fill: color, stroke: '#FFFFFF', strokeWidth: 0.2 }] });

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
  /** Whether the style engine draws each object (texts and dimensions it does not). */
  private readonly drawn: readonly boolean[];
  private readonly present: Record<GeometryClass, number>;
  private readonly classes: GeometryClass[];
  /** The layer's simple look: what a class without a symbol draws. */
  private readonly simple: SymbolSet;
  private kind: Kind;
  private single: SymbolSet;
  private categorized: Categorized;
  private graduated: Graduated;
  private rules: Rule[];
  private gradMethod: 'interval' | 'count' = 'interval';
  private gradCount: number = CLASS_COUNT.default;
  private ramp = DEFAULT_RAMP;
  private unclassed: UnclassedRenderer;
  private proportional: ProportionalRenderer;
  private bivariate: BivariateRenderer;
  private bivariateState = { method: 'count' as 'count' | 'interval', scheme: 'pembeMavi' };
  private dotDensity: DotDensityRenderer;
  private chart: ChartRenderer;
  private heatmap: HeatmapRenderer;
  private cluster: ClusterRenderer;
  private displacement: DisplacementRenderer;
  private inverted: InvertedRenderer;
  /** What a cluster's or a displacement's single points draw with: another tab's draft. */
  private inner: InnerKind = 'simple';
  private applied: string;

  constructor(ctx: AppContext, layerId: string) {
    this.ctx = ctx;
    this.layerId = layerId;
    const node = ctx.doc.layers.get(layerId)!;
    this.entities = ctx.doc.byLayer(layerId);
    this.drawn = drawable(this.entities);
    this.present = classesPresent(this.entities);
    this.classes = (['fill', 'line', 'marker'] as const).filter((c) => this.present[c]);
    if (!this.classes.length) this.classes = ['fill', 'line', 'marker'];
    const current = node.style.renderer;
    this.kind = current?.type ?? 'simple';
    // Every kind keeps its own draft, so switching back and forth loses nothing.
    const simple = symbolsOfLayerStyle(node.style, node.style.color);
    this.simple = pick(simple, this.classes);
    this.single = current?.type === 'single' ? current.symbols : pick(simple, this.classes);
    this.categorized = current?.type === 'categorized' ? current : { type: 'categorized', expr: this.guessField(), categories: [] };
    this.graduated = current?.type === 'graduated' ? current : { type: 'graduated', expr: '$alan', classes: [] };
    this.rules = current?.type === 'rules' ? [...current.rules] : [{ id: 'r1', label: 'Bütün nesneler', symbols: pick(simple, this.classes) }];
    // The thematic renderers' drafts (docs/adr/0213): the current one, else a start from the layer's look.
    const color = node.style.color;
    const field = this.guessField();
    const fields = this.fields().map((f) => f.name);
    const plain = pick(simple, this.classes);
    this.unclassed = current?.type === 'unclassed' ? current : { type: 'unclassed', expr: field || '$alan', min: 0, max: 100, ramp: RAMPS[DEFAULT_RAMP].stops, symbols: plain };
    this.proportional =
      current?.type === 'proportional'
        ? current
        : { type: 'proportional', expr: field || '$alan', minValue: 0, maxValue: 100, minSize: 2, maxSize: 10, unit: 'mm', scaling: 'area', symbols: { ...plain, marker: plain.marker ?? circleOf(color), ...(this.present.fill ? { fill: plain.fill } : {}) } };
    this.bivariate =
      current?.type === 'bivariate'
        ? current
        : { type: 'bivariate', exprX: fields[0] ?? '', exprY: fields[1] ?? '', breaksX: [1, 2], breaksY: [1, 2], colors: bivariateColors(BIVARIATE_SCHEMES.pembeMavi.corners, 3), symbols: plain };
    this.dotDensity =
      current?.type === 'dotDensity' ? current : { type: 'dotDensity', fields: [{ expr: field, color: QUALITATIVE[0] }], dotValue: 1, dotSize: 1, unit: 'mm', seed: 0, ...(plain.line && { symbols: { line: plain.line } }) };
    this.chart =
      current?.type === 'chart'
        ? current
        : { type: 'chart', kind: 'pie', fields: (fields.length ? fields.slice(0, 3) : ['']).map((f, i) => ({ expr: f, color: QUALITATIVE[i] })), size: 8, unit: 'mm', outline: { color: '#FFFFFF', width: 0.2 }, symbols: plain };
    this.heatmap = current?.type === 'heatmap' ? current : { type: 'heatmap', radius: 20, unit: 'px', ramp: HEAT_RAMPS.isi.stops, quality: 2, opacity: 1 };
    this.cluster = current?.type === 'cluster' ? current : { type: 'cluster', distance: 40, unit: 'px' };
    this.displacement = current?.type === 'displacement' ? current : { type: 'displacement', tolerance: 4, unit: 'px', placement: 'ring', circle: { color: '#7D7D7D', width: 1 } };
    this.inverted =
      current?.type === 'inverted'
        ? current
        : { type: 'inverted', symbols: { fill: { type: 'fill', layers: [{ id: 'f', type: 'simpleFill', color: '#FFFFFFB3' }, { id: 'l', type: 'simpleLine', color, width: 0.5 }] } } };
    // A cluster's or a displacement's renderer of single points goes to its own tab's draft.
    const wrapped = current?.type === 'cluster' || current?.type === 'displacement' ? current.renderer : undefined;
    if (wrapped) {
      this.inner = wrapped.type;
      if (wrapped.type === 'single') this.single = wrapped.symbols;
      else if (wrapped.type === 'categorized') this.categorized = wrapped;
      else if (wrapped.type === 'graduated') this.graduated = wrapped;
      else if (wrapped.type === 'rules') this.rules = [...wrapped.rules];
      else if (wrapped.type === 'unclassed') this.unclassed = wrapped;
      else if (wrapped.type === 'proportional') this.proportional = wrapped;
      else if (wrapped.type === 'bivariate') this.bivariate = wrapped;
    }
    this.applied = JSON.stringify(current ?? null);

    this.body = h('div', { class: 'lsty__body' });
    this.status = h('div', { class: 'lsty__status', role: 'status' });
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    cancel.addEventListener('click', () => this.dialog.request());
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
      beforeClose: () => this.confirmClose(node.name),
    });
    this.render();
  }

  /** Changes not applied are asked about in a window over this one (DESIGN.md §7.9.1), as the symbol designer asks. */
  private asking = false;
  private confirmClose(name: string): boolean {
    if (JSON.stringify(this.renderer()) === this.applied) return true;
    if (!this.asking) {
      this.asking = true;
      void askUnsaved({ name: `${name} katman stili`, after: 'Pencere kapanırsa bu değişiklikler kaybolur.', verb: 'kapat', apply: true }).then((a) => {
        this.asking = false;
        if (a === 'discard') this.dialog.close();
        else if (a === 'save' && this.apply()) this.dialog.close();
      });
    }
    return false;
  }

  /** The attribute most objects have, a good first guess for categories. */
  private guessField(): string {
    const counts = new Map<string, number>();
    for (const e of this.entities) for (const k of Object.keys(e.attrs)) counts.set(k, (counts.get(k) ?? 0) + 1);
    return [...counts.entries()].sort((a, b) => b[1] - a[1])[0]?.[0] ?? '';
  }

  /** The layer's attribute names with how many objects have each, in Turkish order. */
  private fields(): { name: string; count: number }[] {
    const counts = new Map<string, number>();
    for (const e of this.entities) for (const k of Object.keys(e.attrs)) counts.set(k, (counts.get(k) ?? 0) + 1);
    return [...counts.entries()].map(([name, count]) => ({ name, count })).sort((a, b) => a.name.localeCompare(b.name, 'tr'));
  }

  private renderer(): LayerRenderer | null {
    return this.draftOf(this.kind);
  }

  private draftOf(kind: Kind): LayerRenderer | null {
    switch (kind) {
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
      case 'unclassed':
        return this.unclassed;
      case 'proportional':
        return this.proportional;
      case 'bivariate':
        return this.bivariate;
      case 'dotDensity':
        return this.dotDensity;
      case 'chart':
        return this.chart;
      case 'heatmap':
        return this.heatmap;
      case 'inverted':
        return this.inverted;
      case 'cluster':
      case 'displacement': {
        const inner = (this.draftOf(this.inner) ?? undefined) as InnerRenderer | undefined;
        const r = kind === 'cluster' ? this.cluster : this.displacement;
        return { ...r, renderer: inner };
      }
    }
  }

  private apply(): boolean {
    const r = this.renderer();
    const why = r ? rendererProblem(r) : null;
    if (why) {
      this.say(`Uygulanmadı: ${why}`, 'warn');
      return false;
    }
    const done = layersRenderer.execute({ doc: this.ctx.doc }, { layer: this.layerId, renderer: r });
    if (done.status !== 'completed') {
      this.say(`Uygulanmadı: ${'error' in done ? done.error.message : 'çizim değişti'}`, 'warn');
      return false;
    }
    this.applied = JSON.stringify(r);
    this.say(r ? 'Stil haritaya uygulandı.' : 'Katman basit görünüşüne döndü.');
    return true;
  }

  /** What the thematic panels take from the window. */
  private host(): PanelHost {
    return {
      ctx: this.ctx,
      classes: this.classes,
      present: this.present,
      simple: this.simple,
      exprField: (v, f, p) => this.exprField(v, f, p),
      numbers: (expr) => numbersPerObject(this.entities, expr, this.exprScope),
      fieldNames: () => this.fields().map((f) => f.name),
      say: (t, k) => this.say(t, k),
    };
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
    const chosen = KINDS.find((k) => k.value === this.kind) ?? KINDS[0];
    const picker = new Dropdown({
      label: 'İşleyici',
      width: 300,
      className: 'lsty__kinds',
      ariaLabel: 'İşleyici',
      items: () =>
        KINDS.flatMap((k): MenuItem[] => [
          ...(k.group ? [{ kind: 'header' as const, label: k.group }] : []),
          { label: k.label, icon: k.icon, detail: k.detail, radio: true, checked: k.value === this.kind, run: () => ((this.kind = k.value), this.render()) },
        ]),
    });
    picker.set(icon(chosen.icon, 16), chosen.label);
    const top = h(
      'div',
      { class: 'lsty__top' },
      picker.el,
      h('span', { class: 'lsty__count' }, `${this.entities.length} nesne: ${[this.present.fill && `${this.present.fill} alan`, this.present.line && `${this.present.line} çizgi`, this.present.marker && `${this.present.marker} nokta`].filter(Boolean).join(', ') || 'çizilecek nesne yok'}`),
    );
    let panel: Child;
    switch (this.kind) {
      case 'simple':
        panel = h(
          'div',
          { class: 'lsty__panel' },
          note('info', h('b', null, 'Katmanın kendi görünüşü. '), 'Renk, çizgi tipi, kalınlık ve dolgu Katmanlar panelinden gelir; nesnelere verilen semboller yine önce gelir.'),
          this.simplePictures(),
        );
        break;
      case 'single':
        panel = h(
          'div',
          { class: 'lsty__single' },
          h('p', { class: 'lsty__help' }, 'Her nesne geometrisine göre bu sembolle çizilir. Sembolü değiştirmek için resmine tıklayın.'),
          symbolSetSlots(this.ctx, this.single, this.classes, (s) => ((this.single = s), this.render()), 'Tek sembol', this.simple),
        );
        break;
      case 'categorized':
        panel = this.categorizedPanel();
        break;
      case 'graduated':
        panel = this.graduatedPanel();
        break;
      case 'rules':
        panel = rulesEditor(this.ctx, this.rules, this.classes, this.entities, (r) => ((this.rules = r), this.render()), this.layerName, this.simple, this.fields());
        break;
      case 'unclassed':
        panel = unclassedPanel(this.host(), this.unclassed, (r) => ((this.unclassed = r), this.render()));
        break;
      case 'proportional':
        panel = proportionalPanel(this.host(), this.proportional, (r) => ((this.proportional = r), this.render()));
        break;
      case 'bivariate':
        panel = bivariatePanel(this.host(), this.bivariate, (r) => ((this.bivariate = r), this.render()), this.bivariateState);
        break;
      case 'dotDensity':
        panel = dotDensityPanel(this.host(), this.dotDensity, (r) => ((this.dotDensity = r), this.render()));
        break;
      case 'chart':
        panel = chartPanel(this.host(), this.chart, (r) => ((this.chart = r), this.render()));
        break;
      case 'heatmap':
        panel = heatmapPanel(this.host(), this.heatmap, (r) => ((this.heatmap = r), this.render()));
        break;
      case 'cluster':
        panel = clusterPanel(this.host(), this.cluster, (r) => ((this.cluster = r), this.render()), this.inner, (k) => ((this.inner = k), this.render()));
        break;
      case 'displacement':
        panel = displacementPanel(this.host(), this.displacement, (r) => ((this.displacement = r), this.render()), this.inner, (k) => ((this.inner = k), this.render()));
        break;
      case 'inverted':
        panel = invertedPanel(this.host(), this.inverted, (r) => ((this.inverted = r), this.render()));
        break;
    }
    replaceChildren(this.body, top, panel);
    if (JSON.stringify(this.renderer()) !== this.applied) this.say('Değişiklikler henüz uygulanmadı.', 'warn');
  }

  /** The pictures of the layer's simple look, one per geometry class it has. */
  private simplePictures(): HTMLElement {
    const CLASS_LABEL: Record<GeometryClass, string> = { fill: 'Alan', line: 'Çizgi', marker: 'Nokta' };
    const pictures = this.classes.flatMap((c) => {
      const s = this.simple[c];
      if (!s || 'ref' in s) return [];
      const canvas = h('canvas', { class: 'slot__pic', width: '96', height: '54', style: 'width:96px;height:54px' });
      queueMicrotask(() => drawNow(this.ctx, canvas, s));
      return [h('div', { class: 'lsty__look' }, canvas, h('span', { class: 'lsty__muted' }, CLASS_LABEL[c]))];
    });
    return h('div', { class: 'lsty__row' }, h('span', { class: 'lsty__label' }, 'Şimdiki görünüşü'), pictures);
  }

  /**
   * An expression field: the text in mono, and menus that put in the layer's
   * fields (with how many objects have each; a name that is not one word goes
   * in brackets, as "…" would be a text), the variables and the functions.
   */
  private exprField(value: string, onChange: (v: string) => void, placeholder: string): { el: HTMLElement; error: HTMLElement } {
    const input = h('input', { class: 'field mono lsty__expr', value, placeholder, 'aria-label': 'Değer ifadesi', spellcheck: 'false' });
    const error = h('div', { class: 'lsty__error' });
    input.addEventListener('change', () => onChange(input.value.trim()));
    const insert = (text: string) => {
      const before = input.value;
      const pad = before && !/[\s(,]$/.test(before) ? ' ' : '';
      onChange(`${before}${pad}${text}`.trim());
    };
    const menu = (label: string, items: () => MenuItem[]) => {
      const b = h('button', { class: 'btn btn--small btn--ghost lsty__menu', type: 'button', 'aria-haspopup': 'menu' }, label, icon('chevronDown', 12));
      b.addEventListener('click', () => PopupMenu.open(items(), b.getBoundingClientRect(), { owner: b }));
      return b;
    };
    const fields = this.fields();
    return {
      el: h(
        'div',
        { class: 'lsty__exprrow' },
        input,
        menu('Alanlar', () => (fields.length ? fields.map((f) => ({ label: f.name, hint: `${f.count} nesne`, run: () => insert(fieldToken(f.name)) })) : [{ label: 'Bu katmanın nesnelerinde öznitelik alanı yok', disabled: true }])),
        menu('Değişkenler', () => exprCatalog().variables.map((v) => ({ label: `$${v.name}`, detail: v.description, run: () => insert(`$${v.name}`) }))),
        menu('İşlevler', () => exprCatalog().functions.map((f) => ({ label: f.signature, detail: f.description, run: () => insert(`${f.name}(`) }))),
      ),
      error,
    };
  }

  private categorizedPanel(): Child {
    const c = this.categorized;
    const set = (next: Partial<Categorized>) => {
      this.categorized = { ...c, ...next };
      this.render();
    };
    const { values, error } = c.expr ? valuesOf(this.entities, c.expr, this.exprScope) : { values: [] as (string | null)[], error: undefined };
    const tally = categoryTally(values, this.drawn, c.categories);
    const expr = this.exprField(c.expr, (v) => set({ expr: v }), 'Alan adı ya da ifade: Nitelik');
    if (error) expr.error.textContent = errorWithPlace(c.expr, error);
    const classify = h('button', { class: 'btn btn--small', type: 'button', disabled: !c.expr || !!error }, 'Değerlerden sınıfla');
    classify.addEventListener('click', () => {
      const found = uniqueValues(values);
      if (!found.length) return this.say(CLASSIFY_TEXTS.noValues, 'warn');
      set({ categories: categoriesOf(found, this.present, c.categories) });
      this.say(CLASSIFY_TEXTS.found(found.length));
    });
    const add = h('button', { class: 'btn btn--small', type: 'button' }, icon('plus', 14), 'Kategori ekle');
    add.addEventListener('click', () => set({ categories: [...c.categories, newCategory(c.categories.length, this.present)] }));
    const clear = h('button', { class: 'btn btn--small btn--ghost', type: 'button', disabled: !c.categories.length }, 'Hepsini sil');
    clear.addEventListener('click', () => set({ categories: [] }));
    const rows = c.categories.map((k, i) => {
      const upd = (patch: Partial<Categorized['categories'][number]>) => set({ categories: c.categories.map((x, j) => (j === i ? { ...x, ...patch } : x)) });
      const on = h('input', { type: 'checkbox', checked: k.enabled !== false, 'aria-label': 'Çizilsin' });
      on.addEventListener('change', () => upd({ enabled: on.checked ? undefined : false }));
      const value = h('input', { class: 'field', value: k.value, 'aria-label': 'Değer', spellcheck: 'false' });
      value.addEventListener('change', () => upd({ value: value.value }));
      const shadow = shadowed(c.categories, i) ? h('span', { class: 'lsty__shadow', title: 'Bu değer yukarıda da var; yalnız ilki çizer.' }, icon('warning', 14)) : null;
      const label = h('input', { class: 'field', value: k.label, 'aria-label': 'Etiket', spellcheck: 'false' });
      label.addEventListener('change', () => upd({ label: label.value }));
      const del = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Kategoriyi sil' }, icon('trash', 15));
      del.addEventListener('click', () => set({ categories: c.categories.filter((_, j) => j !== i) }));
      return h('tr', null, h('td', null, on), h('td', null, symbolSetSlots(this.ctx, k.symbols, this.classes, (s) => upd({ symbols: s }), k.label || k.value, this.simple)), h('td', null, h('div', { class: 'lsty__value' }, value, shadow)), h('td', null, label), h('td', { class: 'num' }, String(tally.counts[i] ?? 0)), h('td', null, del));
    });
    const otherOn = h('input', { type: 'checkbox', checked: !!c.other, 'aria-label': 'Diğer değerler çizilsin' });
    otherOn.addEventListener('change', () => set({ other: otherOn.checked ? plainSymbols(OTHER_COLOR, this.present) : undefined }));
    const rest = tally.rest;
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
          h('tr', { class: 'lsty__other' }, h('td', null, otherOn), h('td', null, c.other ? symbolSetSlots(this.ctx, c.other, this.classes, (s) => set({ other: s }), CLASSIFY_TEXTS.other, this.simple) : h('span', { class: 'lsty__muted' }, 'çizilmez')), h('td', { colspan: '2' }, CLASSIFY_TEXTS.other), h('td', { class: 'num' }, String(rest)), h('td', null, '')),
        ),
      ),
      c.categories.length ? null : h('p', { class: 'lsty__help' }, CLASSIFY_TEXTS.categoriesHelp),
      h('div', { class: 'lsty__tools' }, add, clear),
    );
  }

  private graduatedPanel(): Child {
    const g = this.graduated;
    const set = (next: Partial<Graduated>) => {
      this.graduated = { ...g, ...next };
      this.render();
    };
    const perObject = g.expr ? numbersPerObject(this.entities, g.expr, this.exprScope) : { values: [] as (number | null)[], error: undefined };
    const nums = perObject.values.filter((v): v is number => v !== null);
    const error = perObject.error;
    const tally = classTally(perObject.values, this.drawn, g.classes);
    const expr = this.exprField(g.expr, (v) => set({ expr: v }), 'Sayı veren ifade: $alan, Kat');
    if (error) expr.error.textContent = error;
    else if (g.expr && !nums.length) expr.error.textContent = CLASSIFY_TEXTS.noNumbers;
    const method = h('select', { class: 'field', 'aria-label': 'Yöntem' }, h('option', { value: 'interval', selected: this.gradMethod === 'interval' }, 'Eşit aralık'), h('option', { value: 'count', selected: this.gradMethod === 'count' }, 'Eşit sayı (dilimler)'));
    method.addEventListener('change', () => (this.gradMethod = method.value as 'interval' | 'count'));
    const n = h('input', { class: 'field num lsty__n', value: String(this.gradCount), inputmode: 'numeric', 'aria-label': 'Sınıf sayısı' });
    n.addEventListener('change', () => (this.gradCount = classCount(n.value)));
    const ramp = h('select', { class: 'field', 'aria-label': 'Renk rampası' }, Object.entries(RAMPS).map(([k, r]) => h('option', { value: k, selected: k === this.ramp }, r.label)));
    ramp.addEventListener('change', () => (this.ramp = ramp.value));
    const classify = h('button', { class: 'btn btn--small', type: 'button', disabled: !nums.length }, 'Sınıfla');
    classify.addEventListener('click', () => {
      const classes = graduatedOf(nums, this.gradMethod, this.gradCount, this.ramp, this.present);
      set({ classes });
      this.say(CLASSIFY_TEXTS.classified(classes.length, nums.length));
    });
    const rows = g.classes.map((c, i) => {
      const upd = (patch: Partial<Graduated['classes'][number]>) => set({ classes: g.classes.map((x, j) => (j === i ? { ...x, ...patch } : x)) });
      const numIn = (v: number, key: 'min' | 'max') => {
        // Shown as the labels round it; the class keeps the bound whole until one is typed.
        const el = h('input', { class: 'field num', value: String(Math.round(v * 100) / 100), inputmode: 'decimal', 'aria-label': key === 'min' ? 'Alt sınır' : 'Üst sınır' });
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
      return h('tr', null, h('td', null, symbolSetSlots(this.ctx, c.symbols, this.classes, (s) => upd({ symbols: s }), c.label, this.simple)), h('td', null, numIn(c.min, 'min')), h('td', null, numIn(c.max, 'max')), h('td', null, label), h('td', { class: 'num' }, String(tally.counts[i] ?? 0)), h('td', null, del));
    });
    // What no class takes (no number, or outside every class): not drawn.
    const outside = g.classes.length && tally.rest ? h('tr', { class: 'lsty__other' }, h('td', null, ''), h('td', { colspan: '3' }, 'Sınıfların dışında kalanlar: çizilmez'), h('td', { class: 'num' }, String(tally.rest)), h('td', null, '')) : null;
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
        h('tbody', null, rows, outside),
      ),
      g.classes.length ? h('p', { class: 'lsty__help' }, CLASSIFY_TEXTS.classesHelp) : h('p', { class: 'lsty__help' }, CLASSIFY_TEXTS.classesEmptyHelp),
    );
  }
}

/** An expression's error as the window shows it: “12. karakterde: …” when it is not about the start. */
function errorWithPlace(expr: string, error: string): string {
  const c = compileExpression(expr);
  return c.ok ? error : expressionError(c);
}

/** Only the classes the layer has. */
function pick(set: SymbolSet, classes: readonly GeometryClass[]): SymbolSet {
  const out: { -readonly [K in GeometryClass]?: SymbolSet[K] } = {};
  for (const c of classes) if (set[c]) out[c] = set[c];
  return out;
}
