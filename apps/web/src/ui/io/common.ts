import '../../styles/io.css';
import type { AppContext } from '../../app/context';
import type { Bounds } from '../../contracts/generated/Bounds';
import type { ReportItem } from '../../contracts/generated/ReportItem';
import { CRS_REGISTRY, crsBySrid, DATUM_LABEL, type CrsDef, type Datum } from '../../geo/crs';
import { ENTITY_KIND_LABEL, type EntityKind } from '../../model/entities';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { note } from '../widgets/controls';

/**
 * Parts the import and export windows share: the picked file, the source
 * coordinate system question, labelled fields and the summary lines
 * (DESIGN.md §7.9; styles/io.css loads with these windows, not at start-up).
 */

/** The picked file: its name and a line of facts. */
export function fileLine(name: string, meta: string): HTMLElement {
  return h('div', { class: 'io-file' }, icon('fileOpen', 18), h('div', { class: 'io-file__text' }, h('span', { class: 'io-file__name', title: name }, name), h('span', { class: 'io-file__meta' }, meta)));
}

/**
 * Label above, control below; an optional hint line under it. A div, not a
 * <label>: a click on the text of a label would press the first button of a
 * segmented control inside it (controls carry their own aria-label).
 */
export function field(label: string, control: Child, hint?: Child, size: 'auto' | 'grow' | 'wide' = 'auto'): HTMLElement {
  return h('div', { class: `io-field${size === 'auto' ? '' : ` io-field--${size}`}` }, h('span', { class: 'io-field__label' }, label), control, hint ? h('p', { class: 'io-field__hint' }, hint) : null);
}

/** A checkbox with its text, laid out as a field (label above) so a row lines up. */
export function checkField(label: string, text: string, checked: boolean, onChange: (v: boolean) => void, key: string): HTMLElement {
  const box = h('input', { type: 'checkbox', checked, dataset: { key } });
  box.addEventListener('change', () => onChange(box.checked));
  return field(label, h('label', { class: 'io-check' }, box, text));
}

export interface Choice<T extends string> {
  value: T;
  label: string;
  disabled?: boolean;
}

/** A native select (keyboard and mouse as the system does them). */
export function select<T extends string>(label: string, choices: readonly Choice<T>[], value: T, onChange: (v: T) => void, key?: string): HTMLSelectElement {
  const s = h(
    'select',
    { class: 'field', 'aria-label': label, dataset: key ? { key } : undefined },
    choices.map((c) => h('option', { value: c.value, selected: c.value === value, disabled: c.disabled }, c.label)),
  );
  s.addEventListener('change', () => onChange(s.value as T));
  return s;
}

/** A line of the summary box: a green check, an amber warning, an info mark or a red error. */
export function summaryLine(kind: 'ok' | 'warn' | 'info' | 'error', ...content: Child[]): HTMLElement {
  const glyph = kind === 'ok' ? 'success' : kind === 'warn' ? 'warning' : kind === 'error' ? 'error' : 'info';
  return h('div', { class: 'io-summary__line', dataset: { kind } }, icon(glyph, 14), h('div', null, content));
}

/** A report item as a sentence ("IMAGE: 2, raster görüntüler alınmaz (satır 120, 488)."). */
export const reportText = (i: ReportItem): string =>
  `${i.what}: ${i.count}, ${i.reason}${i.lines.length ? ` (satır ${i.lines.join(', ')}${i.count > i.lines.length ? ' …' : ''})` : ''}.`;

/** Object counts by kind, largest first, without plurals (Turkish counts take none: "12 çizgi, 3 yay"). */
export function kindCounts(counts: ReadonlyMap<string, number>): string {
  return [...counts]
    .sort((a, b) => b[1] - a[1])
    .map(([k, n]) => `${n} ${(ENTITY_KIND_LABEL[k as EntityKind] ?? k).toLocaleLowerCase('tr-TR')}`)
    .join(', ');
}

/** Report items as summary lines. */
export function reportLines(items: readonly ReportItem[], kind: 'warn' | 'info', max = 12): HTMLElement[] {
  const lines = items.slice(0, max).map((i) => summaryLine(kind, reportText(i)));
  if (items.length > max) lines.push(summaryLine(kind, `… ve ${items.length - max} başka kalem.`));
  return lines;
}

/** Where the data lies, in the project's units ("Y 452 345.123 – 452 360.000"). */
export function extentLine(ctx: AppContext, b: Bounds): HTMLElement {
  const f = ctx.format;
  return summaryLine('info', `Kapsam: Y (sağa) ${f.coord(b.minX)} – ${f.coord(b.maxX)}, X (yukarı) ${f.coord(b.minY)} – ${f.coord(b.maxY)}.`);
}

const byDatum = (): Map<Datum, CrsDef[]> => {
  const groups = new Map<Datum, CrsDef[]>();
  for (const c of CRS_REGISTRY) groups.set(c.datum, [...(groups.get(c.datum) ?? []), c]);
  return groups;
};

/** What a file says of its coordinate system (GeoJSON, Shapefile; docs/adr/0046). */
export interface CrsStatement {
  /** The EPSG code the statement names, when it could be read. */
  srid: number | null;
  /** The statement as the file writes it ("urn:ogc:def:crs:EPSG::5256", "TUREF_TM36"); empty when the file says nothing. */
  text: string;
  /** RFC 7946 (no `crs` member), a GeoJSON `crs` member, a .prj; `none`: the file says nothing. */
  source: 'rfc7946' | 'geoJsonCrs' | 'prj' | 'none';
}

/**
 * "Bu koordinatlar hangi sistemde?" A file that says nothing of its system
 * (DXF, a coordinate list, a Shapefile without .prj) starts at the project's
 * system; one that says (GeoJSON, a .prj; `declare`) starts at what it says,
 * or at no choice when that could not be read: a statement is never guessed
 * into a system. Another system than the project's blocks the import:
 * coordinate transformations do not exist yet and coordinates are never
 * reprojected silently (CLAUDE.md §5). The user may say the file is in the
 * project's system after all (a file that claims WGS 84 but holds TM
 * coordinates); the window then says what that means. Where the extent is
 * known, coordinates that cannot be in the chosen system are pointed out.
 */
export class CrsQuestion {
  readonly el: HTMLElement;
  /** The chosen system; null: none chosen yet. */
  private srid: number | null;
  private statement: CrsStatement | null = null;
  private bounds: Bounds | null = null;
  private readonly ctx: AppContext;
  private readonly host = h('div');
  private readonly notes = h('div');
  private readonly onChange: () => void;

  constructor(ctx: AppContext, onChange: () => void) {
    this.ctx = ctx;
    this.onChange = onChange;
    this.srid = ctx.doc.crs.value.srid;
    this.el = h('div', { class: 'io-field' }, h('span', { class: 'io-field__label' }, 'Bu koordinatlar hangi sistemde?'), this.host, this.notes);
    replaceChildren(this.host, this.select());
    this.render();
  }

  /** Whether the file's system is the project's (the only case that can be imported). */
  get matches(): boolean {
    return this.srid !== null && this.srid === this.ctx.doc.crs.value.srid;
  }

  /**
   * What the file says of its system, and the extent of its coordinates:
   * the choice starts at the statement's system (the project's when the
   * file says nothing, none when the statement cannot be read).
   */
  declare(statement: CrsStatement | null, bounds?: Bounds | null): void {
    this.statement = statement;
    this.bounds = bounds ?? null;
    const project = this.ctx.doc.crs.value.srid;
    if (!statement || statement.source === 'none') this.srid = project;
    else this.srid = statement.srid !== null && crsBySrid(statement.srid) ? statement.srid : null;
    replaceChildren(this.host, this.select());
    this.render();
    this.onChange();
  }

  private select(): HTMLSelectElement {
    const project = this.ctx.doc.crs.value.srid;
    const s = h(
      'select',
      { class: 'field', 'aria-label': 'Bu koordinatlar hangi sistemde?' },
      this.srid === null ? h('option', { value: '', selected: true, disabled: true }, 'Sistemi seçin…') : null,
      [...byDatum()].map(([datum, list]) =>
        h(
          'optgroup',
          { label: DATUM_LABEL[datum] },
          list.map((c) =>
            h(
              'option',
              { value: String(c.srid), selected: c.srid === this.srid },
              `${c.name} (EPSG:${c.srid})${c.srid === project ? ', projenin sistemi' : ''}${this.statement?.srid === c.srid && this.statement.source !== 'none' ? ', dosyanın dediği' : ''}`,
            ),
          ),
        ),
      ),
    );
    s.addEventListener('change', () => {
      this.srid = Number(s.value);
      this.render();
      this.onChange();
    });
    return s;
  }

  /** What the file said, as a line. */
  private said(): string | null {
    const st = this.statement;
    if (!st) return null;
    const code = st.srid !== null ? ` (EPSG:${st.srid})` : '';
    switch (st.source) {
      case 'rfc7946':
        return 'Dosya: RFC 7946 GeoJSON, crs üyesi yok; koordinatları WGS 84 boylam, enlem (EPSG:4326) olmalı.';
      case 'geoJsonCrs':
        return st.srid !== null ? `Dosyanın crs üyesi: “${st.text}”${code}.` : `Dosyanın crs üyesi okunamadı (${st.text}); koordinatların sistemini siz seçin.`;
      case 'prj':
        return st.srid !== null ? `Dosyanın .prj'si: “${st.text}”${code}.` : `Dosyanın .prj'si tanınmadı (“${st.text}”); koordinatların sistemini siz seçin.`;
      case 'none':
        return "Dosya koordinat sistemini belirtmiyor (.prj yok): projenin sistemi seçili; koordinatların bu sistemde olduğundan emin olun.";
    }
  }

  /** Coordinates that cannot be in the chosen system (degrees in a metre system, or the reverse). */
  private implausible(chosen: CrsDef): string | null {
    const b = this.bounds;
    if (!b) return null;
    const degrees = Math.abs(b.minX) <= 180 && Math.abs(b.maxX) <= 180 && Math.abs(b.minY) <= 90 && Math.abs(b.maxY) <= 90;
    if (chosen.kind === 'geographic' && !degrees)
      return `Koordinatlar boylam, enlem aralığının dışında (${this.ctx.format.coord(b.minX)} … ${this.ctx.format.coord(b.maxX)}): ${chosen.name} olamaz. Dosyayı yazan program metre koordinatı yazmış olabilir; sistemini biliyorsanız onu seçin.`;
    if (chosen.kind === 'projected' && degrees)
      return `Koordinatların hepsi ±180, ±90 içinde: derece (boylam, enlem) gibi görünüyor; ${chosen.name} sisteminde anlamsız bir yere düşer. Dosyanın sistemini denetleyin.`;
    return null;
  }

  /** The notes under the list (the list itself is made again only when the file changes, keeping its focus). */
  private render(): void {
    const project = this.ctx.doc.crs.value;
    const lines: HTMLElement[] = [];
    const said = this.said();
    if (said) lines.push(h('p', { class: 'io-field__hint' }, said));
    const st = this.statement;
    if (st && st.srid !== null && !crsBySrid(st.srid))
      lines.push(note('warn', `Dosyanın dediği EPSG:${st.srid} KentOS'ta tanımlı değil. Koordinatlar gerçekte projenin sisteminde ise onu seçin; değilse bu dosya dönüştürülmeden alınamaz.`));
    const source = this.srid === null ? undefined : crsBySrid(this.srid);
    if (!source) {
      if (this.srid === null) lines.push(note('info', 'Koordinatların hangi sistemde olduğunu seçin; içe aktarma ancak seçilen sistem projeninki olunca açılır.'));
      replaceChildren(this.notes, lines);
      return;
    }
    const overridden = st && st.source !== 'none' && st.srid !== null && st.srid !== source.srid;
    if (overridden)
      lines.push(
        note(
          'warn',
          h('strong', null, 'Dosyanın dediğinden başka bir sistem seçtiniz. '),
          `Dosya EPSG:${st.srid} diyor; koordinatlar ${source.name} (EPSG:${source.srid}) sayılacak, dönüştürülmeden. Yalnız dosyanın gerçekte bu sistemde olduğunu biliyorsanız seçin.`,
        ),
      );
    if (this.matches) {
      lines.push(h('p', { class: 'io-field__hint' }, `Projenin sistemi (${project.name}). Koordinatlar olduğu gibi alınır; dönüştürülmez, yuvarlanmaz.`));
    } else {
      const what =
        source.datum !== project.datum
          ? `${DATUM_LABEL[source.datum]} → ${DATUM_LABEL[project.datum]} datum dönüşümü`
          : source.kind !== project.kind
            ? 'coğrafi ile projeksiyonlu sistem arasında dönüşüm'
            : 'dilim dönüşümü';
      lines.push(
        note(
          'warn',
          h('strong', null, 'Koordinatlar dönüştürülemez. '),
          `Proje ${project.name} (EPSG:${project.srid}) sisteminde. ${source.name} koordinatlarını almak için ${what} gerekir; koordinat dönüşümü henüz yok (Koordinat → Datum dönüşümü, geliştirme aşamasında) ve koordinatlar sessizce dönüştürülmez, bu yüzden içe aktarma kapalı. Dosya aslında projenin sistemindeyse onu seçin; proje de bu sistemdeyse projenin sistemini Proje ayarları → Koordinat sistemi'nden atayın.`,
        ),
      );
    }
    const odd = this.implausible(source);
    if (odd) lines.push(note('warn', odd));
    replaceChildren(this.notes, lines);
  }
}

/** A name without its extension ("noktalar.ncn" → "noktalar"). */
export const baseName = (name: string) => name.replace(/\.[^.]+$/, '') || name;

/** The file name an export suggests: the drawing's name (without .kcad) and the format's extension. */
export const exportName = (ctx: AppContext, extension: string) => `${ctx.doc.name.value.replace(/\.kcad$/i, '') || 'cizim'}${extension}`;
