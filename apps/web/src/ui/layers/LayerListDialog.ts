import type { AppContext } from '../../app/context';
import { layerListCsv, layerListRows, layerListTsv } from '../../app/layerList';
import { h } from '../dom';
import { summaryLine } from '../io/common';
import { Dialog } from '../widgets/Dialog';

/** The window's title, which a trace names it by. */
export const LIST_TITLE = 'Katman listesi';

/**
 * Katman listesi (docs/adr/0177 §6; the desktop's `layer_list.rs`): every layer and group in the tree's order with its
 * state, look and object count, as the shared rule writes them (`app/layerList.ts`). Panoya kopyala puts the rows
 * between tabs on the clipboard (a spreadsheet takes them); CSV olarak kaydet… downloads them as a CSV file Excel's
 * Turkish settings open.
 */
export function openLayerList(ctx: AppContext): void {
  const { doc, log } = ctx;
  const counts = doc.countByLayer();
  const rows = layerListRows(doc.layers.tree, doc.layers.active.value, (id) => counts.get(id) ?? 0);
  const [head, ...body] = rows;
  const layers = body.filter((r) => r[2] === 'Katman').length;
  const table = h(
    'div',
    { class: 'io-table-wrap layer-list' },
    h(
      'table',
      { class: 'io-table' },
      h('thead', null, h('tr', null, ...head.map((c, i) => h('th', i === head.length - 1 || i === head.length - 2 ? { class: 'num' } : null, c)))),
      h('tbody', null, ...body.map((r) => h('tr', null, ...r.map((c, i) => h('td', i === r.length - 1 || i === r.length - 2 ? { class: 'num' } : null, c))))),
    ),
  );
  const copy = h('button', { class: 'btn', type: 'button' }, 'Panoya kopyala');
  const save = h('button', { class: 'btn', type: 'button' }, 'CSV olarak kaydet…');
  const close = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kapat');
  const dialog = new Dialog({
    title: LIST_TITLE,
    width: 920,
    className: 'dialog--io dialog--layer-list',
    content: [h('div', { class: 'io-summary' }, summaryLine('info', `${layers} katman ve ${body.length - layers} grup, ağacın sırasıyla.`)), table],
    footer: [copy, save, h('div', { class: 'dialog__spacer' }), close],
  });
  copy.addEventListener('click', () => {
    void navigator.clipboard.writeText(layerListTsv(rows)).then(
      () => log.success(`Katman listesi panoya kopyalandı (${body.length} satır; elektronik tabloya yapıştırılabilir).`),
      () => log.warn('Katman listesi panoya kopyalanamadı: tarayıcı izin vermedi.'),
    );
  });
  save.addEventListener('click', () => {
    const url = URL.createObjectURL(new Blob([layerListCsv(rows)], { type: 'text/csv;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = `${doc.name.value || 'cizim'}-katmanlar.csv`;
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    log.success(`Katman listesi CSV olarak kaydedildi: ${a.download} (${body.length} satır).`);
  });
  close.addEventListener('click', () => dialog.close());
}
