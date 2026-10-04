import type { AppContext } from '../../app/context';
import { CRS_REGISTRY, crsBySrid, crsTitle, DATUM_LABEL, DEFAULT_SRID, LOCAL_SRID } from '../../geo/crs';
import { systemOf, type System } from '../../model/geom/crsTransform';
import { crsSettings, datumChoices, DEFINITION_CODE, ownSystem, secondSystem } from '../../model/projectCrs';
import type { ProjectSettings } from '../../model/projectSettings';
import { PickPointTool } from '../../tools/pickPointTool';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { segmented, textField } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { Dropdown } from '../widgets/Dropdown';
import type { MenuItem } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { copyReport, field, Grid, resultTable, summary, summaryLine, type GridModel, type Row } from './common';
import { convertPoint, convertRows, csvText, errorText, fieldNames, type ConvertFormat, type Converted } from './convert';

/**
 * Koordinat dönüştür (`crs.transform`, docs/adr/0167 §4): a point typed or shown on the drawing, or a list pasted, from
 * one coordinate system to another; the source is the project's and the target its second system at first. Each
 * answer says how sure it is (EPSG's operations, ±m). Nothing is written to the drawing; the values are copied, the
 * list also saved as CSV. What is typed stays while the page is open. The project's own definitions are listed first,
 * its datum choices taken where they apply (docs/adr/0168 §9 3c). The desktop's is `apps/desktop/src/calc/convert.rs`.
 */
export function openConvert(ctx: AppContext): void {
  new ConvertDialog(ctx);
}

type Mode = 'point' | 'list';

/** A system the window converts between: one of the registry's by its code, or the project's own definition or its second. */
type Pick = number | 'own' | 'second';

/** A chosen system: what the list calls it, as the transforms read it. */
interface Side {
  readonly pick: Pick;
  readonly name: string;
  readonly title: string;
  readonly system: System;
}

/** WGS 84's latitudes and longitudes, whatever the registry holds. */
const WGS84_SIDE: Side = { pick: 4326, name: 'WGS 84', title: 'WGS 84 (EPSG:4326)', system: { kind: 'geographic', datum: 'WGS84' } };

/** The system a pick names in this project; null for what it does not have, or the transforms do not read. */
function sideOf(pick: Pick, settings: ProjectSettings): Side | null {
  if (typeof pick === 'number') {
    const c = crsBySrid(pick);
    const system = c && c.kind !== 'local' ? systemOf(c) : null;
    return c && system ? { pick, name: c.name, title: crsTitle(c), system } : null;
  }
  const s = crsSettings(settings);
  const named = pick === 'own' ? (s.srid === LOCAL_SRID ? ownSystem(s) : null) : s.secondSrid === null ? secondSystem(s) : null;
  return named?.system ? { pick, name: named.name, title: named.title, system: named.system } : null;
}

/** The project's own system as the window picks it; null without one. */
const ownPick = (s: ProjectSettings): Pick | null => (s.crs.value.srid !== LOCAL_SRID ? s.crs.value.srid : s.customCrs.value ? 'own' : null);

/** The project's second system as the window picks it; null without one. */
const secondPick = (s: ProjectSettings): Pick | null => s.secondSrid.value ?? (s.hasSystem && s.secondCustomCrs.value ? 'second' : null);

const state = {
  from: null as Pick | null,
  to: null as Pick | null,
  mode: 'point' as Mode,
  a: '',
  b: '',
  rows: [{}, {}, {}] as Row[],
};

const TITLE = 'Koordinat dönüştür';

/** The systems one may convert between: the project's own definitions, then a group for each datum, as the registry lists them (no local system). */
function systemItems(settings: ProjectSettings, current: Pick, choose: (pick: Pick) => void): MenuItem[] {
  const out: MenuItem[] = [];
  const defined = (['own', 'second'] as const).map((p) => sideOf(p, settings)).filter((d): d is Side => d !== null);
  if (defined.length) out.push({ kind: 'header', label: 'Projenin tanımları' });
  for (const d of defined) out.push({ label: d.name, hint: DEFINITION_CODE, radio: true, checked: d.pick === current, run: () => choose(d.pick) });
  let datum = '';
  for (const c of CRS_REGISTRY) {
    if (c.kind === 'local') continue;
    if (DATUM_LABEL[c.datum] !== datum) out.push({ kind: 'header', label: (datum = DATUM_LABEL[c.datum]) });
    out.push({ label: c.name, hint: `EPSG:${c.srid}`, radio: true, checked: c.srid === current, run: () => choose(c.srid) });
  }
  return out;
}

class ConvertDialog {
  private readonly ctx: AppContext;
  private readonly systems = h('div', { class: 'calc-form' });
  private readonly body = h('div', { class: 'calc-section' });
  private readonly summaryBox = h('div', { class: 'io-summary' });
  private readonly results = h('div', { class: 'calc-section' });
  private readonly copy = h('button', { class: 'btn btn--primary', type: 'button' }, 'Panoya kopyala');
  private readonly csv = h('button', { class: 'btn', type: 'button' }, 'CSV olarak kaydet');
  private readonly dialog: Dialog;
  private converted: Converted | null = null;
  private listed: { heading: string[]; rows: string[][] } | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    state.from ??= ownPick(ctx.doc.settings) ?? DEFAULT_SRID;
    state.to ??= secondPick(ctx.doc.settings) ?? (this.from.system.kind === 'geographic' ? DEFAULT_SRID : 4326);
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    this.dialog = new Dialog({
      title: TITLE,
      width: 820,
      className: 'dialog--io dialog--calc',
      // The values first, how sure they are under them (docs/adr/0167 §4).
      content: [this.systems, this.body, this.results, this.summaryBox],
      footer: [h('span', { class: 'io-status' }), this.csv, this.copy, close],
    });
    close.addEventListener('click', () => this.dialog.close());
    this.copy.addEventListener('click', () => this.copyValues());
    this.csv.addEventListener('click', () => this.saveCsv());
    this.render();
  }

  private get from(): Side {
    const settings = this.ctx.doc.settings;
    return sideOf(state.from ?? DEFAULT_SRID, settings) ?? sideOf(DEFAULT_SRID, settings) ?? WGS84_SIDE;
  }

  private get to(): Side {
    return sideOf(state.to ?? 4326, this.ctx.doc.settings) ?? WGS84_SIDE;
  }

  /** The project's axes' names and length digits, the user's notation for latitudes and longitudes. */
  private get format(): ConvertFormat {
    const f = this.ctx.format;
    return { east: f.eastLabel, north: f.northLabel, decimals: this.ctx.doc.settings.lengthDecimals.value, notation: this.ctx.prefs.geographic.value };
  }

  private render(): void {
    const pick = (which: 'from' | 'to') => {
      const dd = new Dropdown({
        ariaLabel: which === 'from' ? 'Kaynak sistem' : 'Hedef sistem',
        width: 300,
        items: () => systemItems(this.ctx.doc.settings, which === 'from' ? this.from.pick : this.to.pick, (pick) => ((state[which] = pick), this.render())),
      });
      dd.set((which === 'from' ? this.from : this.to).title);
      return dd.el;
    };
    const swap = h('button', { class: 'btn btn--icon calc-swap', type: 'button', 'aria-label': 'Kaynakla hedefi değiştir' }, icon('reverse', 16));
    swap.addEventListener('click', () => {
      [state.from, state.to] = [state.to, state.from];
      this.render();
    });
    tooltip(swap, () => ({ title: 'Kaynakla hedefi değiştir' }));
    const mode = segmented<Mode>({
      label: 'Dönüştürülecek',
      options: [
        { value: 'point', label: 'Tek nokta' },
        { value: 'list', label: 'Liste', hint: 'Satır satır ad ve iki değer; elektronik tablodan yapıştırılabilir.' },
      ],
      value: state.mode,
      onChange: (m) => ((state.mode = m), this.render()),
    });
    replaceChildren(
      this.systems,
      h('div', { class: 'calc-convert-systems' }, field('Kaynak sistem', pick('from')), swap, field('Hedef sistem', pick('to'))),
      h('div', { class: 'io-row' }, field('Dönüştürülecek', mode)),
    );
    if (state.mode === 'point') this.renderPoint();
    else this.renderList();
    this.recompute();
  }

  /** Tek nokta: the two values, and the point shown on the drawing when the source is the project's system. */
  private renderPoint(): void {
    const [a, b] = fieldNames(this.from.system, this.format);
    const recompute = () => this.recompute();
    const geographic = this.from.system.kind === 'geographic';
    const first = textField({ label: a, value: state.a, placeholder: geographic ? '40 45 12.3456' : '', onChange: (v) => ((state.a = v), recompute()) });
    const second = textField({ label: b, value: state.b, placeholder: geographic ? '29 55 01.2345' : '', onChange: (v) => ((state.b = v), recompute()) });
    first.classList.add('calc-num');
    second.classList.add('calc-num');
    const own = this.from.pick === ownPick(this.ctx.doc.settings);
    const show = h('button', { class: 'btn calc-known__pick', type: 'button', disabled: !own }, icon('snap', 14), 'Çizimden');
    tooltip(show, () => ({
      title: 'Çizimden seç',
      description: own ? 'Noktayı çizimde gösterin; kenetlenir.' : 'Çizimin koordinatları projenin sistemindedir: çizimden seçmek için kaynak sistem projeninki olmalı.',
    }));
    show.addEventListener('click', () => {
      this.dialog.close();
      this.ctx.tools.run(
        new PickPointTool(this.ctx, `${TITLE}: nokta`, (p) => {
          if (p) [state.a, state.b] = [String(p.x), String(p.y)];
          queueMicrotask(() => openConvert(this.ctx));
        }),
        `${TITLE}: nokta`,
      );
    });
    replaceChildren(this.body, h('div', { class: 'calc-convert-point' }, field(a, first), field(b, second), show));
  }

  /** Liste: name and two values a row; a paste fills down and right. */
  private renderList(): void {
    const [a, b] = fieldNames(this.from.system, this.format);
    const numeric = this.from.system.kind !== 'geographic';
    const model: GridModel = {
      columns: [
        { key: 'name', label: 'Ad' },
        { key: 'a', label: a, numeric },
        { key: 'b', label: b, numeric },
      ],
      rows: () => state.rows,
      readonly: () => false,
      canInsertAfter: () => true,
      insertAfter: (r) => state.rows.splice(r + 1, 0, {}),
      canRemove: () => state.rows.length > 1,
      remove: (r) => state.rows.splice(r, 1),
    };
    const grid = new Grid(model, () => this.recompute());
    replaceChildren(
      this.body,
      h('div', { class: 'calc-table-head' }, h('p', { class: 'io-field__hint' }, 'Satır satır ad ve iki değer yazın ya da elektronik tablodan yapıştırın (sekme, noktalı virgül ya da boşlukla ayrılmış).')),
      grid.el,
    );
  }

  private recompute(): void {
    const f = this.format;
    const [from, to] = [this.from, this.to];
    const [ta, tb] = fieldNames(to.system, f);
    const choices = datumChoices(crsSettings(this.ctx.doc.settings));
    this.converted = null;
    this.listed = null;
    if (from.pick === to.pick) {
      summary(this.summaryBox, [summaryLine('warn', 'Kaynak ve hedef aynı sistem: başka bir hedef seçin.')]);
      replaceChildren(this.results);
    } else if (state.mode === 'point') {
      const got = state.a.trim() || state.b.trim() ? convertPoint(from.system, to.system, choices, state.a, state.b, f) : null;
      if (got === null) {
        summary(this.summaryBox, [summaryLine('info', 'Noktanın iki değerini yazın ya da çizimden seçin.')]);
        replaceChildren(this.results);
      } else if (typeof got === 'string') {
        summary(this.summaryBox, [summaryLine('warn', errorText(got, from.system, f))]);
        replaceChildren(this.results);
      } else {
        this.converted = got;
        summary(this.summaryBox, [summaryLine(got.exact ? 'ok' : 'info', `${to.name}: ${got.accuracy}.`)]);
        replaceChildren(
          this.results,
          h(
            'div',
            { class: 'calc-convert-result' },
            ...got.values.map(([name, v]) => h('div', { class: 'calc-convert-value' }, h('span', { class: 'calc-convert-name' }, name), h('span', { class: 'num' }, v))),
          ),
        );
      }
    } else {
      const rows = convertRows(from.system, to.system, choices, state.rows, f);
      const good = rows.filter((r) => typeof r.result !== 'string');
      const bad = rows.filter((r) => typeof r.result === 'string');
      const heading = ['Ad', ta, tb];
      const table = good.map((r) => {
        const c = r.result as Converted;
        return [r.name, c.values[0][1], c.values[1][1]];
      });
      this.listed = good.length ? { heading, rows: table } : null;
      const first = good[0]?.result as Converted | undefined;
      summary(this.summaryBox, [
        rows.length ? null : summaryLine('info', 'Dönüştürülecek satır yok: tabloya yazın ya da yapıştırın.'),
        good.length ? summaryLine('ok', `${good.length} nokta dönüştürüldü. ${to.name}: ${first!.accuracy}.`) : null,
        ...bad.slice(0, 6).map((r) => summaryLine('warn', `Satır ${r.row}: ${errorText(r.result as Exclude<typeof r.result, Converted>, from.system, f)}`)),
        bad.length > 6 ? summaryLine('warn', `${bad.length - 6} satır daha okunamadı.`) : null,
      ]);
      replaceChildren(this.results, good.length ? resultTable(heading, table, [false, true, true]) : null);
    }
    this.copy.disabled = !(this.converted || this.listed);
    this.csv.hidden = state.mode !== 'list';
    this.csv.disabled = !this.listed;
  }

  /** The values to the clipboard: the point's two, or the list's rows with a heading, tab-separated. */
  private copyValues(): void {
    if (this.converted) copyReport(this.ctx, TITLE, [this.converted.values.map(([, v]) => v)]);
    else if (this.listed) copyReport(this.ctx, TITLE, [this.listed.heading, ...this.listed.rows]);
  }

  /** The list as a CSV file, downloaded. */
  private saveCsv(): void {
    if (!this.listed) return;
    const blob = new Blob([csvText(this.listed.heading, this.listed.rows)], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `koordinatlar-${typeof this.to.pick === 'number' ? this.to.pick : 'ozel'}.csv`;
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    this.ctx.log.success(`${this.listed.rows.length} nokta CSV olarak kaydedildi: ${a.download}.`);
  }
}
