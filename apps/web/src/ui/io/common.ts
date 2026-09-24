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

/**
 * "Bu koordinatlar hangi sistemde?" A source file carries no reliable CRS,
 * so the user says it; the project's system is the default. Another system
 * blocks the import: datum and zone transformations do not exist yet and
 * coordinates are never reprojected silently (CLAUDE.md §5).
 */
export class CrsQuestion {
  readonly el: HTMLElement;
  private srid: number;
  private readonly ctx: AppContext;
  private readonly notes = h('div');
  private readonly onChange: () => void;

  constructor(ctx: AppContext, onChange: () => void) {
    this.ctx = ctx;
    this.onChange = onChange;
    this.srid = ctx.doc.crs.value.srid;
    const project = this.srid;
    const s = h(
      'select',
      { class: 'field', 'aria-label': 'Bu koordinatlar hangi sistemde?' },
      [...byDatum()].map(([datum, list]) =>
        h(
          'optgroup',
          { label: DATUM_LABEL[datum] },
          list.map((c) => h('option', { value: String(c.srid), selected: c.srid === project }, `${c.name} (EPSG:${c.srid})${c.srid === project ? ', projenin sistemi' : ''}`)),
        ),
      ),
    );
    s.addEventListener('change', () => {
      this.srid = Number(s.value);
      this.render();
      this.onChange();
    });
    this.el = h('div', { class: 'io-field' }, h('span', { class: 'io-field__label' }, 'Bu koordinatlar hangi sistemde?'), s, this.notes);
    this.render();
  }

  /** Whether the file's system is the project's (the only case that can be imported). */
  get matches(): boolean {
    return this.srid === this.ctx.doc.crs.value.srid;
  }

  private render(): void {
    const project = this.ctx.doc.crs.value;
    const source = crsBySrid(this.srid);
    if (!source || this.matches) {
      replaceChildren(this.notes, h('p', { class: 'io-field__hint' }, `Projenin sistemi (${project.name}). Koordinatlar olduğu gibi alınır; dönüştürülmez, yuvarlanmaz.`));
      return;
    }
    const what =
      source.datum !== project.datum
        ? `${DATUM_LABEL[source.datum]} → ${DATUM_LABEL[project.datum]} datum dönüşümü`
        : source.kind !== project.kind
          ? 'coğrafi ile projeksiyonlu sistem arasında dönüşüm'
          : 'dilim dönüşümü';
    replaceChildren(
      this.notes,
      note(
        'warn',
        h('strong', null, 'Koordinatlar dönüştürülemez. '),
        `Proje ${project.name} (EPSG:${project.srid}) sisteminde. ${source.name} koordinatlarını almak için ${what} gerekir; bu dönüşüm henüz yok (geliştirme aşamasında) ve koordinatlar sessizce dönüştürülmez, bu yüzden içe aktarma kapalı. Dosya aslında projenin sistemindeyse onu seçin; proje de bu sistemdeyse projenin sistemini Proje ayarları → Koordinat sistemi'nden atayın.`,
      ),
    );
  }
}

/** A name without its extension ("noktalar.ncn" → "noktalar"). */
export const baseName = (name: string) => name.replace(/\.[^.]+$/, '') || name;

/** The file name an export suggests: the drawing's name (without .kcad) and the format's extension. */
export const exportName = (ctx: AppContext, extension: string) => `${ctx.doc.name.value.replace(/\.kcad$/i, '') || 'cizim'}${extension}`;
