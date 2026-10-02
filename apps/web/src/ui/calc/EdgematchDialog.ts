import type { AppContext } from '../../app/context';
import type { EntityEdit } from '../../contracts/generated/EntityEdit';
import type { EntityGeometry } from '../../contracts/generated/EntityGeometry';
import { fixed } from '../../core/displayNumber';
import type { Entity } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';
import { edgematchApply, edgematchLinks, type EdgeFound, type EdgeLink, type EdgeMember } from '../../model/ops/edgematch';
import type { PathElevations } from '../../model/ops/warp';
import { elevatedPaths } from '../../product/elevation';
import { entitiesEdit } from '../../product/entitiesEdit';
import { LookTool } from '../../tools/lookTool';
import { PickObjectsTool } from '../../tools/pickObjectsTool';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { select } from '../io/common';
import { segmented, textField } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { copyReport, field, Grid, readNumber, summary, type GridModel, type Row } from './common';
import { describe, LAYER_KEY, MEET_HINT, MEETS, METHOD_HINT, METHODS, summaryLines, TITLE } from './edgematchWords';

/**
 * Kenar eşleme (docs/adr/0159 §9): the line ends of two sheets put together
 * across their common edge. Kaynak is the selection or a layer, Komşu a
 * layer, Sınır an object shown on the drawing. At every change the core
 * (`edgematchLinks`) finds the links, the best continuation within the
 * search distance and the angle tolerance; the table lists them with
 * Kullan and Göster, and the core works out what Uygula would write
 * (`edgematchApply`), so the summary can say which links cannot be written.
 * Uygula writes through `cad.entities.edit` (one undo step, Kenar eşle) and
 * selects what it put right. What is typed stays for the session. The
 * desktop's is `apps/desktop/src/calc/edgematch/`.
 */
export function openEdgematch(ctx: AppContext): void {
  new EdgematchDialog(ctx);
}

type Scope = 'selection' | 'layer';

const state = {
  scope: 'layer' as Scope,
  source: null as string | null,
  adjacent: null as string | null,
  /** The border object's persistent id. */
  border: null as string | null,
  distance: '0.5',
  angle: '30',
  /** '' no criterion, LAYER_KEY the layers' names, else an attribute's name. */
  key: '',
  meet: 'adjacent' as (typeof MEETS)[number]['value'],
  method: 'move' as (typeof METHODS)[number]['value'],
  /** Links left out, by their ends (`linkKey`). */
  off: new Set<string>(),
};

/** What the core gives back for a line or a polyline put right. */
type PutShape = { kind: 'line'; a: Vec2; b: Vec2 } | { kind: 'polyline'; pts: Vec2[]; bulges?: number[] };

const COLUMNS = [
  { key: 'use', label: 'Kullan', check: true },
  { key: 'src', label: 'Kaynak' },
  { key: 'adj', label: 'Komşu' },
  { key: 'gap', label: 'Aralık', unit: 'mm', numeric: true },
  { key: 'angle', label: 'Açı farkı', unit: '°', numeric: true },
];

/** The kinds a sheet's line work has: lines and polylines take part, areas only as junctions. */
const takes = (e: Entity) => e.kind === 'line' || e.kind === 'polyline' || e.kind === 'polygon';

class EdgematchDialog {
  private readonly ctx: AppContext;
  private readonly setsBox = h('div', { class: 'io-row' });
  private readonly numbersBox = h('div', { class: 'io-row' });
  private readonly choicesBox = h('div', { class: 'calc-section' });
  private readonly summaryBox = h('div', { class: 'io-summary' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly apply = h('button', { class: 'btn btn--primary', type: 'button' }, 'Uygula');
  private readonly copy = h('button', { class: 'btn', type: 'button' }, 'Raporu kopyala');
  private readonly grid: Grid;
  private readonly dialog: Dialog;
  /** The table's rows: a link each, with its ends' key. */
  private rows: Row[] = [];
  /** The sets as the core took them, and what it found. */
  private sources: Entity[] = [];
  private adjacent: Entity[] = [];
  private members: { sources: EdgeMember<Entity>[]; adjacent: EdgeMember<Entity>[] } = { sources: [], adjacent: [] };
  private found: EdgeFound | null = null;
  private others = 0;
  private locked = 0;
  private problem: string | null = null;
  /** What Uygula writes now; none while nothing can be written. */
  private changes: EntityEdit[] = [];
  private written = 0;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    const model: GridModel = {
      columns: COLUMNS,
      rows: () => this.rows,
      addLabel: null,
      readonly: (_r, key) => key !== 'use',
      mark: (r) => this.mark(r),
      actions: (r) => [
        {
          icon: 'zoomSelection',
          label: `${r + 1}. bağı çizimde göster`,
          tip: 'Pencere kenara çekilir, bağın iki çizgisi seçilip bağa yakınlaşılır. Tıklayın ya da Enter’a basın: pencere geri gelir.',
          run: () => this.show(r),
        },
      ],
      canInsertAfter: () => false,
      insertAfter: () => {},
      canRemove: () => false,
      remove: () => {},
    };
    this.grid = new Grid(model, () => this.toggled());
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    this.dialog = new Dialog({
      title: TITLE,
      width: 980,
      className: 'dialog--io dialog--calc dialog--edgematch',
      content: [
        this.setsBox,
        this.numbersBox,
        this.choicesBox,
        h('div', { class: 'calc-section' }, h('h3', { class: 'calc-results__title' }, 'Bağlar'), this.grid.el),
        this.summaryBox,
      ],
      footer: [this.status, this.copy, close, this.apply],
    });
    close.addEventListener('click', () => this.dialog.close());
    this.apply.addEventListener('click', () => this.write());
    this.copy.addEventListener('click', () => this.report());
    this.defaults();
    this.renderControls();
    this.find();
  }

  /** The layers holding line work, in the tree's order, with how many lines and polylines each. */
  private lineLayers(): { value: string; label: string }[] {
    const { layers } = this.ctx.doc;
    const count = new Map<string, number>();
    for (const e of this.ctx.doc.all()) if (e.kind === 'line' || e.kind === 'polyline') count.set(e.layerId, (count.get(e.layerId) ?? 0) + 1);
    return layers
      .leaves()
      .filter((l) => count.has(l.id))
      .map((l) => ({ value: l.id, label: `${layers.path(l.id)} (${count.get(l.id)})` }));
  }

  /** Kaynak and Komşu on opening: kept while the drawing has them, else the first two layers holding line work. */
  private defaults(): void {
    const choices = this.lineLayers();
    const has = (id: string | null) => !!id && choices.some((c) => c.value === id);
    if (!has(state.source)) state.source = choices[0]?.value ?? null;
    if (!has(state.adjacent) || state.adjacent === state.source) state.adjacent = choices.find((c) => c.value !== state.source)?.value ?? null;
    if (state.scope === 'selection' && !this.ctx.selection.size) state.scope = 'layer';
    if (state.border && !this.ctx.doc.byUid(state.border)) state.border = null;
    if (state.meet === 'border' && !state.border) state.meet = 'adjacent';
  }

  private renderControls(): void {
    const { ctx } = this;
    const layers = ctx.doc.layers;
    const choices = this.lineLayers();
    const none = [{ value: '', label: 'Çizgisi olan katman yok' }];
    const scope = segmented<Scope>({
      label: 'Kaynak',
      options: [
        { value: 'selection', label: `Seçili (${ctx.selection.size})`, disabled: !ctx.selection.size },
        { value: 'layer', label: 'Katman' },
      ],
      value: state.scope,
      onChange: (v) => ((state.scope = v), this.changed()),
    });
    const source = select('Kaynak katmanı', choices.length ? choices : none, state.source ?? '', (v) => ((state.source = v || null), this.changed()), 'source');
    const adjacent = select('Komşu katmanı', choices.length ? choices : none, state.adjacent ?? '', (v) => ((state.adjacent = v || null), this.changed()), 'adjacent');
    const borderEntity = state.border ? ctx.doc.byUid(state.border) : undefined;
    const borderText = borderEntity ? describe(borderEntity, layers.path(borderEntity.layerId)) : 'Seçilmedi';
    const pick = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Sınırı çizimden seç' }, icon('target', 14));
    pick.title = 'Sahneden seç: pafta kenarı ya da çerçevesi (çizgi, çoklu çizgi ya da alan)';
    pick.addEventListener('click', () => this.pickBorder());
    const clear = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Sınırı kaldır', disabled: !borderEntity }, icon('close', 12));
    clear.addEventListener('click', () => ((state.border = null), state.meet === 'border' && (state.meet = 'adjacent'), this.changed()));
    replaceChildren(
      this.setsBox,
      field('Kaynak', scope, 'Düzeltilecek çizgiler'),
      state.scope === 'layer' ? field('Kaynak katmanı', source, null, 'grow') : null,
      field('Komşu katmanı', adjacent, 'Komşu paftanın çizgileri', 'grow'),
      field('Sınır', h('div', { class: 'calc-border', 'data-key': 'border' }, h('span', { class: 'calc-border__name' }, borderText), pick, clear), 'İsteğe bağlı: pafta kenarı', 'grow'),
    );
    const num = (label: string, key: 'distance' | 'angle', hint: string) => {
      const f = textField({ label, value: state[key], placeholder: key === 'distance' ? '0.5' : '30', onChange: (v) => ((state[key] = v), this.find()) });
      f.classList.add('calc-num');
      f.dataset.key = key;
      return field(label, f, hint);
    };
    const attributes = new Set<string>();
    for (const e of ctx.doc.all()) if ((e.layerId === state.source || e.layerId === state.adjacent) && e.attrs) for (const k of Object.keys(e.attrs)) attributes.add(k);
    const keys = [
      { value: '', label: 'Yok' },
      { value: LAYER_KEY, label: 'Katman adı' },
      ...[...attributes].sort((a, b) => a.localeCompare(b, 'tr')).map((k) => ({ value: k, label: `Öznitelik: ${k}` })),
    ];
    if (!keys.some((k) => k.value === state.key)) state.key = '';
    replaceChildren(
      this.numbersBox,
      num('Arama uzaklığı (m)', 'distance', 'Uçlar en çok bu kadar aralıklı'),
      num('Açı toleransı (°)', 'angle', 'Çizgilerin doğrultusu en çok bu kadar sapar'),
      field('Eşleşme ölçütü', select('Eşleşme ölçütü', keys, state.key, (v) => ((state.key = v), this.find()), 'key'), 'Aynı değeri taşıyanlar eşlenir', 'grow'),
    );
    const meet = segmented({
      label: 'Buluşma',
      options: MEETS.map((m) => ({ ...m, disabled: m.value === 'border' && !borderEntity })),
      value: state.meet,
      onChange: (v) => ((state.meet = v), this.renderControls(), this.find()),
    });
    const method = segmented({ label: 'Yöntem', options: METHODS, value: state.method, onChange: (v) => ((state.method = v), this.renderControls(), this.preview()) });
    replaceChildren(this.choicesBox, h('div', { class: 'io-row' }, field('Buluşma', meet, MEET_HINT[state.meet], 'grow'), field('Yöntem', method, METHOD_HINT[state.method], 'grow')));
  }

  /** A set or a choice changed: the controls and the links again. */
  private changed(): void {
    this.renderControls();
    this.find();
  }

  /** Why nothing can be looked for: the layers, the numbers. */
  private check(): string | null {
    if (state.scope === 'layer' && !state.source) return 'Kaynak katmanı seçin: düzeltilecek çizgilerin katmanı.';
    if (!state.adjacent) return 'Komşu katmanı seçin: komşu paftanın çizgilerinin katmanı.';
    if (state.scope === 'layer' && state.source === state.adjacent) return 'Kaynak ve komşu aynı katman. Komşu paftanın katmanını seçin.';
    const d = readNumber(state.distance);
    if (d === null || !Number.isFinite(d) || d <= 0) return 'Arama uzaklığı sıfırdan büyük bir uzunluk olmalı (metre).';
    const a = readNumber(state.angle);
    if (a === null || !Number.isFinite(a) || a <= 0 || a > 180) return 'Açı toleransı 0 ile 180 derece arasında olmalı.';
    return null;
  }

  /** The sets, the links and the table again. */
  private find(): void {
    const { ctx } = this;
    const { doc } = ctx;
    const layers = doc.layers;
    this.problem = this.check();
    this.found = null;
    this.rows = [];
    if (!this.problem) {
      const visible = (e: Entity) => layers.isVisible(e.layerId);
      const picked = state.scope === 'selection' ? [...ctx.selection.ids.value].flatMap((id) => doc.get(id) ?? []) : [...doc.all()].filter((e) => e.layerId === state.source);
      const sources = picked.filter(visible);
      const own = new Set(sources.map((e) => e.id));
      const neighbours = [...doc.all()].filter((e) => e.layerId === state.adjacent && !own.has(e.id) && visible(e));
      // A locked object is not put right: a source never, a neighbour when the ends meet halfway or on the border.
      const moves = state.meet !== 'adjacent';
      this.locked = sources.filter((e) => layers.isLocked(e.layerId)).length + (moves ? neighbours.filter((e) => layers.isLocked(e.layerId)).length : 0);
      this.sources = sources.filter((e) => takes(e) && !layers.isLocked(e.layerId));
      this.adjacent = neighbours.filter((e) => takes(e) && !(moves && layers.isLocked(e.layerId)));
      const skipped = sources.filter((e) => !takes(e) && !layers.isLocked(e.layerId)).length;
      const keyOf = (e: Entity): string | null => {
        if (!state.key) return null;
        if (state.key === LAYER_KEY) return layers.get(e.layerId)?.name ?? null;
        const v = e.attrs?.[state.key];
        return typeof v === 'string' ? v : null;
      };
      const member = (e: Entity): EdgeMember<Entity> => ({ shape: e, zs: elevatedPaths(e).map((p) => p.zs), key: keyOf(e) });
      this.members = { sources: this.sources.map(member), adjacent: this.adjacent.map(member) };
      const border = state.border ? (doc.byUid(state.border) ?? null) : null;
      this.found = edgematchLinks(this.members.sources, this.members.adjacent, {
        distance: readNumber(state.distance) as number,
        angle: readNumber(state.angle) as number,
        border,
        keyed: state.key !== '',
      });
      this.others = this.found.others + skipped;
      const name = (e: Entity) => describe(e, layers.path(e.layerId));
      this.rows = this.found.links.map((l) => {
        const key = this.linkKey(l);
        return {
          use: state.off.has(key) ? '0' : '1',
          src: name(this.sources[l.source]),
          adj: name(this.adjacent[l.adjacent]),
          gap: fixed(l.gap * 1000, 1),
          angle: fixed(l.angle, 1),
          key,
        };
      });
    }
    this.grid.render();
    this.preview();
  }

  /** A link's ends by persistent id: what Kullan remembers while the links are found again. */
  private linkKey(l: EdgeLink): string {
    const uid = (e: Entity) => this.ctx.doc.uidOf(e.id) ?? String(e.id);
    return `${uid(this.sources[l.source])}:${l.sourceEnd}>${uid(this.adjacent[l.adjacent])}:${l.adjacentEnd}`;
  }

  /** Kullan changed: what is left out is remembered, the writing worked out again. */
  private toggled(): void {
    for (const row of this.rows) {
      if (row.use === '0') state.off.add(row.key ?? '');
      else state.off.delete(row.key ?? '');
    }
    this.preview();
  }

  /** What Uygula would write: the used links put together by the core; the summary and the buttons. */
  private preview(): void {
    const { doc } = this.ctx;
    const f = this.found;
    this.changes = [];
    this.written = 0;
    let refused: number[] = [];
    let problem = this.problem;
    const usedRows = this.rows.flatMap((row, i) => (row.use === '0' ? [] : [i]));
    if (f && !problem && usedRows.length) {
      const used = usedRows.map((i) => f.links[i]);
      const border = state.border ? (doc.byUid(state.border) ?? null) : null;
      const answer = edgematchApply(this.members.sources, this.members.adjacent, used, state.meet, state.method, border);
      if ('error' in answer) problem = 'Sınırda buluşma için bir sınır seçin.';
      else {
        refused = answer.refused.map((k) => usedRows[k] + 1);
        this.written = used.length - answer.refused.length;
        const put = (list: (Entity | null)[], zs: (PathElevations | null)[], of: Entity[]) =>
          list.forEach((shape, i) => {
            const uid = doc.uidOf(of[i].id);
            if (shape && uid) this.changes.push({ kind: 'update', uid, geometry: geometryOf(shape as unknown as PutShape, zs[i]) });
          });
        put(answer.sources, answer.sourceZs, this.sources);
        put(answer.adjacent, answer.adjacentZs, this.adjacent);
      }
    }
    this.grid.refresh();
    const gaps = usedRows.map((i) => f!.links[i].gap);
    const worst = usedRows.length ? usedRows.reduce((a, b) => (f!.links[b].gap > f!.links[a].gap ? b : a)) : null;
    summary(
      this.summaryBox,
      summaryLines({
        problem,
        found: f,
        used: usedRows.length,
        worst: worst === null ? null : { gap: f!.links[worst].gap, who: `${worst + 1}. satır` },
        mean: gaps.length ? gaps.reduce((a, b) => a + b, 0) / gaps.length : null,
        others: this.others,
        locked: this.locked,
        refused,
        bordered: !!state.border,
      }),
    );
    this.apply.disabled = !this.changes.length;
    this.copy.disabled = !f;
  }

  /** The used link with the largest gap reads in the warning colour; a link left out, faded. */
  private mark(r: number): string | null {
    const row = this.rows[r];
    if (!row) return null;
    if (row.use === '0') return 'off';
    const f = this.found;
    if (!f) return null;
    let worst = -1;
    this.rows.forEach((x, i) => {
      if (x.use !== '0' && (worst < 0 || f.links[i].gap > f.links[worst].gap)) worst = i;
    });
    return worst === r ? 'worst' : null;
  }

  private write(): void {
    if (this.apply.disabled || !this.changes.length) return;
    const { ctx } = this;
    const result = entitiesEdit.execute({ doc: ctx.doc }, { operation: 'edgematch', changes: this.changes });
    if (result.status !== 'completed') {
      this.status.textContent = 'error' in result ? result.error.message : 'Yazılamadı.';
      this.status.dataset.kind = 'error';
      return;
    }
    ctx.log.success(`${TITLE}: ${this.written} bağ yazıldı, ${result.output.changed.length} çizgi düzeltildi. Ctrl+Z geri alır.`);
    for (const w of result.warnings) ctx.log.warn(w.message);
    ctx.selection.set(result.output.changed.flatMap((uid) => ctx.doc.byUid(uid)?.id ?? []));
    this.dialog.close();
  }

  /** Sahneden seç for Sınır: the window steps aside, one line, polyline or area is picked, the window comes back. */
  private pickBorder(): void {
    const { ctx } = this;
    const before = [...ctx.selection.ids.value];
    this.dialog.close();
    ctx.selection.clear();
    ctx.tools.run(
      new PickObjectsTool(ctx, `${TITLE}: sınır`, ['line', 'polyline', 'polygon'], (keep) => {
        const first = keep ? [...ctx.selection.ids.value][0] : undefined;
        if (first !== undefined) state.border = ctx.doc.uidOf(first) ?? state.border;
        ctx.selection.set(before);
        queueMicrotask(() => openEdgematch(ctx));
      }),
      `${TITLE}: sınır`,
    );
  }

  /** Göster: the window steps aside, the link's two lines are selected and the view goes to it; a click brings the window back. */
  private show(r: number): void {
    const { ctx } = this;
    const l = this.found?.links[r];
    if (!l) return;
    const before = [...ctx.selection.ids.value];
    const half = Math.max(3, l.gap * 60);
    const mid = { x: (l.from.x + l.to.x) / 2, y: (l.from.y + l.to.y) / 2 };
    this.dialog.close();
    ctx.selection.set([this.sources[l.source].id, this.adjacent[l.adjacent].id]);
    ctx.view.zoomToBox({ minX: mid.x - half, minY: mid.y - half, maxX: mid.x + half, maxY: mid.y + half }, 48);
    ctx.tools.run(
      new LookTool(ctx, `${TITLE}: ${r + 1}. bağ, aralık ${fixed(l.gap * 1000, 1)} mm`, () => {
        ctx.selection.set(before);
        queueMicrotask(() => openEdgematch(ctx));
      }),
      `${TITLE}: ${r + 1}. bağ`,
    );
  }

  /** The report: the settings, the links and the counts, tab-separated. */
  private report(): void {
    const f = this.found;
    if (!f) return;
    const { layers } = this.ctx.doc;
    const border = state.border ? this.ctx.doc.byUid(state.border) : undefined;
    const lines: string[][] = [
      [TITLE],
      ['Kaynak', state.scope === 'selection' ? `Seçili (${this.ctx.selection.size})` : layers.path(state.source ?? '')],
      ['Komşu', layers.path(state.adjacent ?? '')],
      ['Sınır', border ? describe(border, layers.path(border.layerId)) : '—'],
      ['Arama uzaklığı (m)', state.distance],
      ['Açı toleransı (°)', state.angle],
      ['Eşleşme ölçütü', state.key === '' ? 'Yok' : state.key === LAYER_KEY ? 'Katman adı' : state.key],
      ['Buluşma', MEETS.find((m) => m.value === state.meet)?.label ?? ''],
      ['Yöntem', METHODS.find((m) => m.value === state.method)?.label ?? ''],
      ['Kullan', 'No', 'Kaynak', 'Komşu', 'Aralık (mm)', 'Açı farkı (°)'],
      ...this.rows.map((row, i) => [row.use === '0' ? 'hayır' : 'evet', String(i + 1), row.src ?? '', row.adj ?? '', row.gap ?? '', row.angle ?? '']),
      ['Eşsiz uç', String(f.unmatched.length), 'Kavşak ucu', String(f.junctions), 'Katılmayan', String(this.others)],
    ];
    copyReport(this.ctx, TITLE, lines);
  }
}

/** A line or a polyline put right as `cad.entities.edit` takes it, its path's elevations written. */
function geometryOf(shape: PutShape, zs: PathElevations | null): EntityGeometry {
  const written = zs?.[0];
  if (shape.kind === 'line') return { kind: 'line', a: shape.a, b: shape.b, ...(written ? { zs: written } : {}) } as EntityGeometry;
  return { kind: 'polyline', pts: shape.pts, ...(shape.bulges ? { bulges: shape.bulges } : {}), ...(written ? { zs: written } : {}) } as EntityGeometry;
}
