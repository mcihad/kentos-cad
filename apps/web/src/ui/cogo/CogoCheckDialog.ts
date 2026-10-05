import { COGO_REPORT_HEADER, cogoItemWords, cogoRows, cogoSummary, cogoUpdate, type CogoRow } from '../../app/cogo';
import type { AppContext } from '../../app/context';
import { layerListCsv, layerListTsv } from '../../app/layerList';
import { h, replaceChildren } from '../dom';
import { field, summaryLine } from '../io/common';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { VirtualRows } from '../widgets/VirtualRows';

/** The window's title, which a trace names it by. */
export const COGO_TITLE = 'Kayıtlı ölçüleri denetle';

/**
 * Kayıtlı ölçüleri denetle (docs/adr/0180 §3; the desktop's `cogo.rs`): the lines and arcs with recorded values (every
 * one, or the selected ones) checked against the drawing with the tolerances typed, a row per recorded value; Yalnız
 * farklar leaves the matching ones out; a row's click selects its object and shows it. Seçilenlere çizimden yaz writes
 * the selected objects' measured values (Kayıtlı ölçüleri çizimden yaz) and checks again; Panoya kopyala, CSV olarak
 * kaydet….
 */
export function openCogoCheck(ctx: AppContext): void {
  const { doc, log } = ctx;
  let selectionOnly = ctx.selection.ids.value.size > 0;
  let onlyDiffs = true;
  let rows: CogoRow[] = [];
  let shown: [CogoRow, number][] = [];

  const scopeBox = h('div');
  const lengthTol = h('input', { class: 'field compare-num', 'aria-label': 'Uzunluk toleransı (m)', value: '0.01', inputmode: 'decimal', spellcheck: 'false' }) as HTMLInputElement;
  const semtTol = h('input', { class: 'field compare-num', 'aria-label': 'Semt toleransı (cc)', value: '50', inputmode: 'decimal', spellcheck: 'false' }) as HTMLInputElement;
  const only = h('input', { type: 'checkbox', checked: onlyDiffs }) as HTMLInputElement;
  const summary = h('div', { class: 'io-summary' });
  const tbody = h('tbody');
  const scroller = h('div', { class: 'io-table-wrap compare-rows' }, h('table', { class: 'io-table' }, h('thead', null, h('tr', null, ...COGO_REPORT_HEADER.map((c) => h('th', null, c)))), tbody));
  const list = new VirtualRows({ parent: tbody, scroller, spacer: () => h('tr', null, h('td', { colspan: '7' })), row: (i) => rowOf(shown[i]) });
  const button = (words: string, primary = false) => h('button', { class: primary ? 'btn btn--primary' : 'btn', type: 'button' }, words) as HTMLButtonElement;
  const check = button('Denetle', true);
  const update = button('Seçilenlere çizimden yaz');
  const copy = button('Panoya kopyala');
  const csv = button('CSV olarak kaydet…');
  const close = button('Kapat');

  function rowOf([row, i]: [CogoRow, number]): HTMLElement {
    const words = cogoItemWords(ctx, row, i);
    const item = row.finding.items[i];
    // The comparison table's colours: unreadable yellow, differing red, matching green.
    const kind = item.difference === undefined ? 'geometry' : item.over ? 'removed' : 'added';
    const tr = h('tr', { class: `compare-row compare-row--${kind}`, title: 'Nesneye gitmek için tıklayın' }, ...words.map((w) => h('td', null, w)));
    tr.addEventListener('click', () => {
      ctx.selection.set([row.entity.id]);
      ctx.view.zoomToSelection();
    });
    return tr;
  }

  function buildScope(): void {
    replaceChildren(
      scopeBox,
      field(
        'Kapsam',
        segmented({
          label: 'Kapsam',
          options: [
            { value: 'all', label: 'Bütün çizim' },
            { value: 'selection', label: 'Seçim' },
          ],
          value: selectionOnly ? 'selection' : 'all',
          onChange: (v) => ((selectionOnly = v === 'selection'), buildScope()),
        }),
      ),
    );
  }

  function list_(): void {
    shown = rows.flatMap((r) => r.finding.items.map((_, i) => [r, i] as [CogoRow, number]).filter(([, i]) => !onlyDiffs || r.finding.items[i].over || r.finding.items[i].difference === undefined));
    list.set(shown.length);
    const differs = rows.some((r) => r.finding.status !== 'ok');
    replaceChildren(summary, summaryLine(rows.length && differs ? 'warn' : rows.length ? 'ok' : 'info', cogoSummary(rows)));
    for (const b of [copy, csv]) b.disabled = !rows.length;
  }

  function run(): void {
    const number = (input: HTMLInputElement) => Number(input.value.trim().replace(',', '.'));
    const length = number(lengthTol);
    const cc = number(semtTol);
    if (!(length >= 0) || !(cc >= 0)) return void log.warn('Toleranslar sıfır ya da artı birer sayı olmalı: uzunluk metre, semt cc.');
    rows = cogoRows(ctx, selectionOnly, { length, cc });
    log.info(cogoSummary(rows));
    list_();
  }

  const report = () => [[...COGO_REPORT_HEADER], ...rows.flatMap((r) => r.finding.items.map((_, i) => cogoItemWords(ctx, r, i, true)))];

  const dialog = new Dialog({
    title: COGO_TITLE,
    width: 860,
    className: 'dialog--io dialog--cogo',
    content: [
      h('div', { class: 'io-row compare-run' }, scopeBox, field('Uzunluk toleransı (m)', lengthTol), field('Semt toleransı (cc)', semtTol), check, field('Liste', h('label', { class: 'io-check' }, only, 'Yalnız farklar'))),
      summary,
      scroller,
    ],
    footer: [copy, csv, update, h('div', { class: 'dialog__spacer' }), close],
    onClose: () => list.dispose(),
  });

  check.addEventListener('click', run);
  only.addEventListener('change', () => ((onlyDiffs = only.checked), list_()));
  update.addEventListener('click', () => {
    if (cogoUpdate(ctx, ctx.selection.ids.value)) run();
  });
  copy.addEventListener('click', () => {
    const lines = report();
    void navigator.clipboard.writeText(layerListTsv(lines)).then(
      () => log.success(`Kayıtlı ölçüler raporu panoya kopyalandı (${lines.length - 1} satır; elektronik tabloya yapıştırılabilir).`),
      () => log.warn('Rapor panoya kopyalanamadı: tarayıcı izin vermedi.'),
    );
  });
  csv.addEventListener('click', () => {
    const lines = report();
    const url = URL.createObjectURL(new Blob([layerListCsv(lines)], { type: 'text/csv;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = `${doc.name.value || 'cizim'}-kayitli-olculer.csv`;
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    log.success(`Kayıtlı ölçüler raporu CSV olarak kaydedildi: ${a.download} (${lines.length - 1} satır).`);
  });
  close.addEventListener('click', () => dialog.close());
  buildScope();
  run();
}
