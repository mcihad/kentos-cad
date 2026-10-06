import type { AppContext } from '../../app/context';
import {
  blankCells,
  BLANK_MOST,
  chooseTableFile,
  inOrder,
  insertState,
  lookOf,
  newTable,
  readTableFile,
  SCHEDULE_KINDS,
  scheduleOf,
  sourceOf,
  type InsertSource,
} from '../../app/tables';
import type { TableGrid, TableSource } from '../../model/entities';
import { sheetCells } from '../../model/tables';
import type { ScheduleKind, TableCells } from '../../model/ops/table';
import { STANDARD_STYLE } from '../../model/annotationStyles';
import { chosenStyles, stylesShown } from '../../tools/styleOption';
import { PickObjectsTool } from '../../tools/pickObjectsTool';
import { TablePlaceTool } from '../../tools/tablePlaceTool';
import { h, replaceChildren } from '../dom';
import { icon, type IconName } from '../icons';
import { checkField, field, select, summaryLine } from '../io/common';
import { stepper } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import '../../styles/table.css';

/** The window's title, which a trace names it by. */
export const INSERT_TITLE = 'Tablo ekle';

/** The sources, in the window's order, with their icons. */
const SOURCES: readonly { value: InsertSource; label: string; icon: IconName; hint: string }[] = [
  { value: 'blank', label: 'Boş tablo', icon: 'table', hint: 'Satır ve sütun sayısı verilen boş tablo: hücreler Tabloyu düzenle ile yazılır.' },
  { value: 'file', label: 'Dosyadan', icon: 'tableFile', hint: 'Excel (.xlsx) çalışma kitabının bir sayfası ya da CSV/TXT dosyası; ayırıcı ve kodlama dosyadan anlaşılır.' },
  { value: 'coordinates', label: 'Koordinat', icon: 'tableCoordinates', hint: 'Koordinat çizelgesi: noktaların ve çizgi, çoklu çizgi ve alan köşelerinin adları, koordinatları ve kotları; ortak köşe bir kez.' },
  { value: 'areas', label: 'Alan', icon: 'tableAreas', hint: 'Alan çizelgesi: alanların ve dairelerin adları, alanları ve çevreleri; birden çoksa toplamı.' },
  { value: 'attributes', label: 'Öznitelik', icon: 'tableAttributes', hint: 'Öznitelik tablosu: nesnelerin adları ve öznitelikleri, her öznitelik bir sütun.' },
];

/** Çizgiler's choices. */
const GRIDS: readonly { value: TableGrid | 'all'; label: string; icon: IconName }[] = [
  { value: 'all', label: 'Tümü', icon: 'tableGridAll' },
  { value: 'outer', label: 'Dış', icon: 'tableGridOuter' },
  { value: 'rows', label: 'Satırlar', icon: 'tableGridRows' },
  { value: 'none', label: 'Yok', icon: 'tableGridNone' },
];

/** The rows and columns the preview shows at most. */
const PREVIEW_ROWS = 8;
const PREVIEW_COLUMNS = 6;

const isSchedule = (s: InsertSource): s is ScheduleKind => s === 'coordinates' || s === 'areas' || s === 'attributes';

/**
 * Tablo ekle (docs/adr/0184 §3, §4; the desktop's `tables/insert.rs`): the source (Boş tablo, Dosyadan, a schedule of
 * objects picked with Sahneden seç or selected before), the heading row, Yazı stili (a CAD project's) and the text
 * height on paper, Çizgiler and Kalın çerçeve; a preview of the first rows and the table's size. Yerleştir hands the
 * table to the placement tool (`TablePlaceTool`). The window keeps its choices for as long as the app lives
 * (`insertState`); objects selected when it opens are the schedule's.
 */
export function openTableInsert(ctx: AppContext, picked = false): void {
  const s = insertState;
  if (!picked) {
    const chosen = inOrder(ctx, ctx.selection.ids.value).filter((e) => e.kind !== 'table');
    s.objects = chosen.flatMap((e) => e.uid ?? []);
  }
  let style: string | null = chosenStyles.text;

  const sourceBox = h('div', { class: 'table-sources', role: 'radiogroup', 'aria-label': 'Kaynak' });
  const detail = h('div', { class: 'table-detail' });
  const looks = h('div', { class: 'io-row' });
  const lines = h('div', { class: 'io-row' });
  const preview = h('div', { class: 'table-preview' });
  const summary = h('div', { class: 'io-summary' });
  const place = h('button', { class: 'btn btn--primary', type: 'button' }, 'Yerleştir') as HTMLButtonElement;
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç') as HTMLButtonElement;

  const objects = () => s.objects.flatMap((uid) => ctx.doc.byUid(uid) ?? []);

  /** The cells the source gives now, with their source; or why none. */
  function cellsNow(): { cells: TableCells; source?: TableSource } | { why: string; kind: 'info' | 'error' } {
    if (s.source === 'blank') return { cells: blankCells(s.rows, s.columns) };
    if (s.source === 'file') {
      if (!s.file) return { why: 'Bir Excel (.xlsx), CSV ya da TXT dosyası seçin.', kind: 'info' };
      if (s.file.read.problem) return { why: s.file.read.problem, kind: 'error' };
      const sheet = s.file.read.sheets[s.sheet] ?? s.file.read.sheets[0];
      if (!sheet) return { why: 'Dosyada okunacak sayfa yok.', kind: 'error' };
      const cells = sheetCells(sheet, s.header);
      return cells.problem ? { why: cells.problem, kind: 'error' } : { cells, source: { kind: 'file', name: s.file.name, ...(sheet.name !== undefined && { sheet: sheet.name }) } };
    }
    const list = objects();
    if (!list.length) return { why: 'Nesne seçilmedi: Sahneden seç ile çizimden seçin.', kind: 'info' };
    const cells = scheduleOf(ctx, s.source, list);
    return cells.problem ? { why: cells.problem, kind: 'error' } : { cells, source: sourceOf(s.source, list) };
  }

  function sources(): void {
    replaceChildren(
      sourceBox,
      ...SOURCES.map((o) => {
        const b = h(
          'button',
          { class: 'table-source', type: 'button', role: 'radio', 'aria-checked': String(o.value === s.source), title: o.hint, dataset: { source: o.value } },
          icon(o.icon, 22),
          h('span', null, o.label),
        );
        b.addEventListener('click', () => {
          s.source = o.value;
          build();
        });
        return b;
      }),
    );
  }

  function details(): void {
    const hint = SOURCES.find((o) => o.value === s.source)?.hint ?? '';
    if (s.source === 'blank') {
      replaceChildren(
        detail,
        h(
          'div',
          { class: 'io-row' },
          field('Satır', stepper({ label: 'Satır', value: s.rows, min: 1, max: BLANK_MOST, onChange: (v) => ((s.rows = v), build()) })),
          field('Sütun', stepper({ label: 'Sütun', value: s.columns, min: 1, max: BLANK_MOST, onChange: (v) => ((s.columns = v), build()) })),
          checkField('Başlık', 'İlk satır başlık', s.header, (v) => ((s.header = v), build()), 'header'),
        ),
        h('p', { class: 'io-field__hint' }, hint),
      );
      return;
    }
    if (s.source === 'file') {
      const choose = h('button', { class: 'btn', type: 'button' }, icon('tableFile', 16), s.file ? 'Başka dosya…' : 'Dosya seç…');
      choose.addEventListener('click', () => void pickFile());
      const book = s.file?.read;
      const sheets = book?.sheets ?? [];
      const meta = book && !book.problem ? [sheets.length > 1 ? `${sheets.length} sayfa` : null, book.encoding ?? null].filter(Boolean).join(' · ') : '';
      replaceChildren(
        detail,
        h(
          'div',
          { class: 'table-file' },
          h('div', { class: 'io-file__text' }, h('span', { class: 'io-file__name' }, s.file?.name ?? 'Dosya seçilmedi'), meta ? h('span', { class: 'io-file__meta' }, meta) : null),
          choose,
        ),
        h(
          'div',
          { class: 'io-row' },
          sheets.length > 1
            ? field(
                'Sayfa',
                select(
                  'Sayfa',
                  sheets.map((sh, i) => ({ value: String(i), label: sh.name ?? `Sayfa ${i + 1}` })),
                  String(s.sheet),
                  (v) => ((s.sheet = Number(v)), build()),
                  'sheet',
                ),
              )
            : null,
          checkField('Başlık', 'İlk satır başlık', s.header, (v) => ((s.header = v), build()), 'header'),
        ),
        h('p', { class: 'io-field__hint' }, hint),
      );
      return;
    }
    const kind = s.source;
    const pick = h('button', { class: 'btn', type: 'button', title: 'Pencere çizimin yanına çekilir; nesneleri tıklayın ya da pencereyle seçin, Enter bitirir.' }, icon('target', 16), 'Sahneden seç');
    pick.addEventListener('click', () => pickObjects(kind));
    const n = objects().length;
    replaceChildren(
      detail,
      h('div', { class: 'table-objects' }, h('span', { class: 'table-objects__count' }, n ? `${n.toLocaleString('tr-TR')} nesne` : 'Nesne seçilmedi'), pick),
      h('p', { class: 'io-field__hint' }, hint),
    );
  }

  function appearance(): void {
    const scale = ctx.doc.settings.plotScale.value;
    const styles = ctx.doc.settings.textStyles.value;
    const fixed = ctx.doc.settings.textStyle(style ?? undefined)?.height;
    replaceChildren(
      looks,
      stylesShown(ctx)
        ? field(
            'Yazı stili',
            select('Yazı stili', [{ value: '', label: STANDARD_STYLE }, ...styles.map((st) => ({ value: st.id, label: st.name }))], style ?? '', (v) => ((style = v || null), build()), 'style'),
          )
        : null,
      field(
        'Yazı yüksekliği',
        fixed !== undefined
          ? h('span', { class: 'table-fixed' }, `${fixed} mm (stilin)`)
          : stepper({ label: 'Yazı yüksekliği', value: s.heightMm, min: 0.5, max: 50, step: 0.5, decimals: 2, unit: 'mm', onChange: (v) => ((s.heightMm = v), build()) }),
        `Kâğıtta; çizimde ${ctx.format.length(((fixed ?? s.heightMm) / 1000) * scale, false)} ${ctx.format.lengthUnitLabel} (1:${scale.toLocaleString('tr-TR')}).`,
      ),
    );
    const gridButtons = h(
      'div',
      { class: 'seg table-lines', role: 'radiogroup', 'aria-label': 'Çizgiler' },
      ...GRIDS.map((g) => {
        const b = h('button', { class: 'seg__opt', type: 'button', role: 'radio', 'aria-checked': String(g.value === s.grid), dataset: { grid: g.value } }, icon(g.icon, 16), g.label);
        b.addEventListener('click', () => ((s.grid = g.value), build()));
        return b;
      }),
    );
    const frameBox = h('input', { type: 'checkbox', checked: s.frame && s.grid !== 'none', disabled: s.grid === 'none', dataset: { key: 'frame' } }) as HTMLInputElement;
    frameBox.addEventListener('change', () => ((s.frame = frameBox.checked), build()));
    replaceChildren(
      lines,
      field('Çizgiler', gridButtons),
      field(
        'Kalın çerçeve',
        h(
          'div',
          { class: 'table-frame', title: s.grid === 'none' ? 'Çizgisiz tabloda çerçeve yok.' : 'Dış çizgi, kâğıtta bu kalınlıkta, tablonun içine doğru dolu bir bant olur.' },
          h('label', { class: 'io-check' }, frameBox, icon('tableFrame', 16), 'Çerçeve'),
          s.frame && s.grid !== 'none' ? stepper({ label: 'Çerçeve kalınlığı', value: s.frameMm, min: 0.1, max: 10, step: 0.1, decimals: 2, unit: 'mm', onChange: (v) => ((s.frameMm = v), build()) }) : null,
        ),
      ),
    );
  }

  /** The first rows of the cells, the heading row bold, numbers right as their columns. */
  function previewOf(cells: TableCells | null): void {
    if (!cells) return void replaceChildren(preview);
    const rows = cells.cells.slice(0, PREVIEW_ROWS);
    const m = Math.min(cells.cells[0]?.length ?? 0, PREVIEW_COLUMNS);
    const header = isSchedule(s.source) || s.header;
    const more = [cells.cells.length > PREVIEW_ROWS ? `${(cells.cells.length - PREVIEW_ROWS).toLocaleString('tr-TR')} satır daha` : null, (cells.cells[0]?.length ?? 0) > PREVIEW_COLUMNS ? `${(cells.cells[0].length - PREVIEW_COLUMNS).toLocaleString('tr-TR')} sütun daha` : null].filter(Boolean);
    replaceChildren(
      preview,
      h('span', { class: 'io-field__label' }, 'Önizleme'),
      h(
        'div',
        { class: 'table-preview__wrap' },
        h(
          'table',
          { class: 'table-preview__grid' },
          h(
            'tbody',
            null,
            rows.map((r, i) =>
              h(
                'tr',
                { class: i === 0 && header ? 'table-preview__head' : null },
                r.slice(0, m).map((w, j) => h('td', { class: i === 0 && header ? null : cells.aligns[j] === 'right' ? 'num' : null }, w || ' ')),
              ),
            ),
          ),
        ),
      ),
      more.length ? h('p', { class: 'io-field__hint' }, `… ${more.join(', ')}.`) : null,
    );
  }

  function build(): void {
    sources();
    details();
    appearance();
    const now = cellsNow();
    if ('why' in now) {
      previewOf(null);
      replaceChildren(summary, summaryLine(now.kind, now.why));
      place.disabled = true;
      return;
    }
    previewOf(now.cells);
    const header = isSchedule(s.source) || s.header;
    const table = newTable(ctx, now.cells, lookOf(ctx, style, header), { x: 0, y: 0 }, now.source);
    const width = table.columns.reduce((a, b) => a + b, 0);
    const depth = table.rows.reduce((a, b) => a + b, 0);
    const f = ctx.format;
    const n = now.cells.cells.length;
    const m = now.cells.cells[0]?.length ?? 0;
    replaceChildren(
      summary,
      summaryLine('ok', `${n.toLocaleString('tr-TR')} satır × ${m.toLocaleString('tr-TR')} sütun; tablo ${f.length(width, false)} × ${f.length(depth, false)} ${f.lengthUnitLabel}.`),
      now.cells.fitted ? summaryLine('info', `${now.cells.fitted.toLocaleString('tr-TR')} hücrenin satır sonları boşluk oldu: hücre tek satırdır.`) : null,
    );
    place.disabled = false;
  }

  async function pickFile(): Promise<void> {
    const file = await chooseTableFile();
    if (!file) return;
    const read = await readTableFile(ctx, file);
    if (!read) return;
    s.file = { name: file.name, read };
    s.sheet = 0;
    build();
  }

  /** Sahneden seç: the window steps aside, the objects are picked, the window comes back with them. */
  function pickObjects(kind: ScheduleKind): void {
    const before = [...ctx.selection.ids.value];
    dialog.close();
    ctx.selection.set(objects().map((e) => e.id));
    ctx.tools.run(
      new PickObjectsTool(ctx, `${INSERT_TITLE}: nesneler`, SCHEDULE_KINDS[kind], (keep) => {
        if (keep) s.objects = inOrder(ctx, ctx.selection.ids.value).flatMap((e) => e.uid ?? []);
        ctx.selection.set(before);
        queueMicrotask(() => openTableInsert(ctx, true));
      }),
      `${INSERT_TITLE}: nesneler`,
    );
  }

  function go(): void {
    const now = cellsNow();
    if ('why' in now) return;
    const header = isSchedule(s.source) || s.header;
    const look = lookOf(ctx, style, header);
    chosenStyles.text = style;
    dialog.close();
    ctx.tools.run(new TablePlaceTool(ctx, (p) => newTable(ctx, now.cells, look, p, now.source)), INSERT_TITLE);
  }

  place.addEventListener('click', go);
  cancel.addEventListener('click', () => dialog.close());
  const dialog = new Dialog({
    title: INSERT_TITLE,
    width: 680,
    className: 'dialog--io dialog--table',
    content: [field('Kaynak', sourceBox), detail, h('div', { class: 'table-section' }, h('span', { class: 'table-section__title' }, 'Görünüş'), looks, lines), preview, summary],
    footer: [cancel, place],
  });
  build();
}
