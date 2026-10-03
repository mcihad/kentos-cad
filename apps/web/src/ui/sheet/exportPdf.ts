import type { Finding } from '../../contracts/generated/sheet/Finding';
import type { SheetBook } from '../../contracts/generated/sheet/SheetBook';
import type { DisposableStore } from '../../core/disposable';
import { h, type Child } from '../dom';
import { icon } from '../icons';
import { note, segmented, toggleSwitch } from '../widgets/controls';
import { field } from './widgets/form';

/**
 * The export window's PDF part (docs/sheet/design.md §9a): which sheets go
 * (this one, those chosen, all; a page each), GeoPDF (on when the project
 * has a transverse Mercator system, else off and why), the drawing's layers
 * as the viewer's layer list, how each map frame goes (vectors, or a
 * picture at the fallback resolution and the layers that make it one),
 * what else the PDF writes differently from the screen (the core's
 * findings: an SVG picture as a picture of so many pixels, or the missing
 * picture's box), and the file's name from the sheet's export defaults.
 */

export type PdfScope = 'this' | 'chosen' | 'all';
export type PdfAction = 'save' | 'print' | 'open';

export interface PdfState {
  scope: PdfScope;
  chosen: Set<string>;
  geo: boolean;
  layers: boolean;
  /** What this window worked out for the sheets and the resolution chosen, kept while they stay. */
  ways?: { key: string; ways: MapWayView[] };
  notes?: { key: string; notes: readonly Finding[] | null };
}

export interface PdfChoiceView {
  readonly sheets: readonly string[];
  readonly dpi: number;
  readonly geo: boolean;
  readonly layers: boolean;
}

export interface MapWayView {
  readonly sheet: string;
  readonly item: string;
  readonly name: string;
  readonly vector: boolean;
  readonly why: string;
}

/** What the window asks of the app for a PDF. */
export interface PdfExporter {
  /** Writes it and saves, prints or opens it (`tab`: one opened at the click); its name, or null (said why). */
  pdf(choice: PdfChoiceView, action: PdfAction, tab: Window | null): Promise<string | null>;
  pdfWays(choice: Pick<PdfChoiceView, 'sheets' | 'dpi'>): MapWayView[];
  /** What the PDF writes differently from the screen besides the maps (the core's findings), worked out before it is written. */
  pdfNotes(choice: Pick<PdfChoiceView, 'sheets' | 'dpi'>): Promise<readonly Finding[]>;
  pdfName(choice: PdfChoiceView): string;
  /** Whether GeoPDF can be written: the project's system is a transverse Mercator one (TM, UTM). */
  pdfGeoReady(): boolean;
}

export interface PdfFieldsInput {
  readonly book: SheetBook;
  readonly sheetId: string;
  readonly state: PdfState;
  readonly crsName: string | null;
  readonly geoReady: boolean;
  readonly dpi: number;
  ways(): MapWayView[];
  notes(): Promise<readonly Finding[]>;
  name(): string;
  rerender(): void;
}

export const PDF_TEXTS = {
  noCrs: 'Projenin koordinat sistemi yok: GeoPDF yazılamaz. Proje ayarlarından sistemi seçin.',
  notTm: (name: string) => `Projenin sistemi (${name}) enine Merkatör değil: GeoPDF bu sürümde TM ve UTM sistemleri için yazılır.`,
  geo: (name: string) => `Konumlu her harita koordinat taşır (${name}): GDAL, QGIS ve Acrobat haritadan koordinat okur.`,
  layers: 'Görüntüleyicinin katman listesinden her çizim katmanı açılıp kapatılır.',
  allVector: (n: number) => `${n === 1 ? 'Harita' : `${n} harita`} vektör olarak gider: çizgiler ve yazılar keskin, yazılar seçilebilir.`,
  picture: (name: string, dpi: number, why: string) => `“${name}” ${dpi} dpi resim olarak gider. Vektörle yazılamayanlar: ${why}.`,
  noMaps: 'Bu paftalarda harita çerçevesi yok.',
} as const;

const SCOPES: readonly { value: PdfScope; label: string }[] = [
  { value: 'this', label: 'Bu pafta' },
  { value: 'chosen', label: 'Seçtiklerim' },
  { value: 'all', label: 'Hepsi' },
];

/** The core's findings for the PDF in the window: warnings (with their fix) first, then the notes. */
function notesOf(notes: readonly Finding[]): Child[] {
  const row = (f: Finding) => h('span', { class: 'sheet-export__way', dataset: { code: f.code } }, `${f.message} ${f.fix}`);
  const warnings = notes.filter((f) => f.severity !== 'info');
  const infos = notes.filter((f) => f.severity === 'info');
  return [warnings.length ? note('warn', ...warnings.map(row)) : null, infos.length ? note('info', ...infos.map(row)) : null];
}

export function pdfFields(o: PdfFieldsInput, d: DisposableStore): Child[] {
  const s = o.state;
  // Worked out again only when the sheets or the resolution change (each window its own).
  const key = `${s.scope}|${[...s.chosen].join(',')}|${o.dpi}|${o.book.sheets.map((x) => x.id).join(',')}`;
  if (s.ways?.key !== key) s.ways = { key, ways: o.ways() };
  const ways = s.ways.ways;
  if (s.notes?.key !== key) {
    // The SVG pictures are drawn first (a moment): the notes come when they are.
    const mine: NonNullable<PdfState['notes']> = { key, notes: null };
    s.notes = mine;
    void o.notes().then(
      (n) => {
        mine.notes = n;
        if (s.notes === mine && n.length) o.rerender();
      },
      () => (mine.notes = []),
    );
  }
  const notes = s.notes.notes ?? [];
  const pictures = ways.filter((w) => !w.vector);
  const out: Child[] = [
    field(
      'Paftalar',
      segmented<PdfScope>({ label: 'Paftalar', value: s.scope, options: SCOPES.map((x) => ({ ...x, label: x.value === 'all' ? `${x.label} (${o.book.sheets.length})` : x.label })), onChange: (v) => ((s.scope = v), o.rerender()) }),
      'Her pafta bir sayfa; kâğıt boyu ve ölçek aynen.',
    ),
  ];
  if (s.scope === 'chosen')
    out.push(
      h(
        'div',
        { class: 'sheet-toggles sheet-export__sheets' },
        o.book.sheets.map((x) =>
          h(
            'label',
            { class: 'sheet-toggle' },
            toggleSwitch({ label: x.name, checked: s.chosen.has(x.id), onChange: (on) => (on ? s.chosen.add(x.id) : s.chosen.delete(x.id), o.rerender()) }),
            h('span', null, x.name),
          ),
        ),
      ),
    );
  const geoWhy = !o.crsName ? PDF_TEXTS.noCrs : !o.geoReady ? PDF_TEXTS.notTm(o.crsName) : null;
  out.push(
    h(
      'div',
      { class: 'sheet-insp__row' },
      h('span', { class: 'sheet-insp__label' }, 'GeoPDF'),
      toggleSwitch({ label: 'GeoPDF', checked: s.geo && !geoWhy, disabled: !!geoWhy, onChange: (v) => ((s.geo = v), o.rerender()) }),
    ),
    h('p', { class: 'sheet-insp__hint' }, geoWhy ?? PDF_TEXTS.geo(o.crsName!)),
    h(
      'div',
      { class: 'sheet-insp__row' },
      h('span', { class: 'sheet-insp__label' }, 'Çizim katmanları PDF katmanı olsun'),
      toggleSwitch({ label: 'Çizim katmanları PDF katmanı olsun', checked: s.layers, onChange: (v) => ((s.layers = v), o.rerender()) }),
    ),
    h('p', { class: 'sheet-insp__hint' }, PDF_TEXTS.layers),
    !ways.length ? note('info', PDF_TEXTS.noMaps) : pictures.length ? note('warn', ...pictures.map((w) => h('span', { class: 'sheet-export__way' }, PDF_TEXTS.picture(w.name, o.dpi, w.why)))) : note('info', PDF_TEXTS.allVector(ways.length)),
    ...notesOf(notes),
    h('p', { class: 'sheet-insp__hint sheet-export__file' }, icon('export', 13), `Dosya: ${o.name()}.pdf`),
  );
  void d;
  return out;
}
