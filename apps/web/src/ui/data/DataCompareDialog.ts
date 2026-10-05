import type { AppContext } from '../../app/context';
import {
  countsText,
  fieldsOf,
  nodeChoices,
  reportRows,
  rowWords,
  runCompare,
  thisDrawing,
  writeDifferences,
  type CompareDrawing,
  type CompareResult,
  type CompareSide,
} from '../../app/dataCompare';
import { layerListCsv, layerListTsv } from '../../app/layerList';
import { readDrawing } from '../../app/drawingFile';
import type { LayerInit, LayerNode } from '../../model/layers';
import type { CompareRow } from '../../model/ops/compare';
import { h, replaceChildren } from '../dom';
import { field, select, summaryLine } from '../io/common';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { VirtualRows } from '../widgets/VirtualRows';

/** The window's title, which a trace names it by. */
export const COMPARE_TITLE = 'Veri karşılaştır';

/** A file's layer tree as the comparison reads it: ids, names, kinds and children. */
function treeOf(inits: readonly LayerInit[]): LayerNode[] {
  return inits.map(
    (n) =>
      ({
        id: n.id ?? '',
        name: n.name ?? '',
        type: n.type ?? 'layer',
        visible: n.visible ?? true,
        locked: n.locked ?? false,
        expanded: true,
        style: { color: 'fg', lineType: 'continuous', lineWeight: 0.18 },
        children: treeOf((n as { children?: LayerInit[] }).children ?? []),
      }) as LayerNode,
  );
}

/**
 * Veri karşılaştır (docs/adr/0179; the desktop's `data_compare.rs`): Eski (this drawing or another drawing file, a layer,
 * a group or the whole drawing) and Yeni (this drawing's), the pairing (Konumla, Anahtar alanla), the key field, the
 * search distance and the tolerance, the compared fields; Karşılaştır lists the rows (a row's click selects its object
 * and shows it), Yalnız farklar leaves the same ones out; Panoya kopyala, CSV olarak kaydet…, Farkları çizime yaz.
 */
export function openDataCompare(ctx: AppContext): void {
  const { doc, log } = ctx;
  const here = thisDrawing(ctx);
  let other: CompareDrawing | null = null;
  let oldHere = true;
  const active = doc.layers.active.value;
  const firstOther = doc.layers.all().find((n) => n.type === 'layer' && n.id !== active);
  let oldNode: string = firstOther?.id ?? '';
  let newNode: string = active;
  let matching: 'location' | 'key' = 'location';
  let key = '';
  const ignored = new Set<string>();
  let onlyDiffs = true;
  let result: CompareResult | null = null;
  let shown: CompareRow[] = [];

  const oldDrawing = () => (oldHere || !other ? here : other);
  const sides = (): [CompareSide, CompareSide] => [
    { drawing: oldDrawing(), node: oldNode || null },
    { drawing: here, node: newNode || null },
  ];

  const sourceBox = h('div', { class: 'compare-source' });
  const oldBox = h('div', { class: 'compare-node' });
  const newBox = h('div', { class: 'compare-node' });
  const matchBox = h('div');
  const keyBox = h('div');
  const search = h('input', { class: 'field compare-num', 'aria-label': 'Arama uzaklığı (m)', value: '1', inputmode: 'decimal', spellcheck: 'false' }) as HTMLInputElement;
  const tolerance = h('input', { class: 'field compare-num', 'aria-label': 'Tolerans (m)', value: '0.001', inputmode: 'decimal', spellcheck: 'false' }) as HTMLInputElement;
  const fieldsBox = h('div', { class: 'io-row compare-fields' });
  const only = h('input', { type: 'checkbox', checked: onlyDiffs }) as HTMLInputElement;
  const summary = h('div', { class: 'io-summary' });
  const tbody = h('tbody');
  const head = ['Durum', 'Tür', 'Anahtar', 'Eski katman', 'Yeni katman', 'Konum farkı', 'Değişen alanlar'];
  const scroller = h('div', { class: 'io-table-wrap compare-rows' }, h('table', { class: 'io-table' }, h('thead', null, h('tr', null, ...head.map((c) => h('th', null, c)))), tbody));
  const rows = new VirtualRows({ parent: tbody, scroller, spacer: () => h('tr', null, h('td', { colspan: String(head.length) })), row: (i) => rowOf(shown[i]) });
  const button = (words: string, primary = false) => h('button', { class: primary ? 'btn btn--primary' : 'btn', type: 'button' }, words) as HTMLButtonElement;
  const compare = button('Karşılaştır', true);
  const fromFile = button('Dosyadan…');
  const copy = button('Panoya kopyala');
  const csv = button('CSV olarak kaydet…');
  const write = button('Farkları çizime yaz');
  const close = button('Kapat');
  const file = h('input', { type: 'file', accept: '.kcad,.KCAD', hidden: true }) as HTMLInputElement;

  function rowOf(row: CompareRow): HTMLElement {
    const r = result!;
    const words = rowWords(r, row, (m) => ctx.format.length(m));
    const tr = h('tr', { class: `compare-row compare-row--${row.status}`, title: 'Nesneye gitmek için tıklayın' }, ...words.map((w) => h('td', null, w)));
    tr.addEventListener('click', () => show(row));
    return tr;
  }

  /** A row's object on the drawing: the new one, else the old one of this drawing, selected; another drawing's only shown. */
  function show(row: CompareRow): void {
    const r = result;
    if (!r) return;
    const b = row.new === undefined ? undefined : r.next[row.new];
    const a = row.old === undefined ? undefined : r.old[row.old];
    const mine = b ?? (r.oldSide.drawing.here ? a : undefined);
    if (mine) {
      ctx.selection.set([mine.id]);
      ctx.view.zoomToSelection();
      return;
    }
    if (a) {
      const xs: number[] = [];
      const ys: number[] = [];
      const walk = (v: unknown): void => {
        if (Array.isArray(v)) v.forEach(walk);
        else if (v && typeof v === 'object') {
          const o = v as Record<string, unknown>;
          if (typeof o.x === 'number' && typeof o.y === 'number') (xs.push(o.x), ys.push(o.y));
          else Object.values(o).forEach(walk);
        }
      };
      walk(a);
      if (xs.length) ctx.view.zoomToBox({ minX: Math.min(...xs), minY: Math.min(...ys), maxX: Math.max(...xs), maxY: Math.max(...ys) }, 80);
    }
  }

  function build(): void {
    const [o, n] = sides();
    const choices = [{ value: 'here', label: 'Bu çizim' }, ...(other ? [{ value: 'file', label: other.name }] : [])];
    replaceChildren(sourceBox, field('Eski veri', select('Eski veri', choices, oldHere ? 'here' : 'file', (v) => ((oldHere = v === 'here'), (oldNode = ''), build()))), fromFile);
    replaceChildren(oldBox, field('Eski katman', select('Eski katman', nodeChoices(o.drawing.tree), oldNode, (v) => ((oldNode = v), build())), undefined, 'grow'));
    replaceChildren(newBox, field('Yeni katman', select('Yeni katman', nodeChoices(here.tree), newNode, (v) => ((newNode = v), build())), undefined, 'grow'));
    replaceChildren(
      matchBox,
      field(
        'Eşleme',
        segmented({
          label: 'Eşleme',
          options: [
            { value: 'location', label: 'Konumla' },
            { value: 'key', label: 'Anahtar alanla' },
          ],
          value: matching,
          onChange: (v) => ((matching = v), build()),
        }),
      ),
    );
    const fields = fieldsOf(o, n);
    if (key && !fields.includes(key)) key = '';
    // Paired by location there is no key: the list is off, saying so.
    const keySelect =
      matching === 'key'
        ? select('Anahtar alan', [{ value: '', label: 'Seçin' }, ...fields.map((f) => ({ value: f, label: f }))], key, (v) => ((key = v), build()))
        : select('Anahtar alan', [{ value: '', label: 'Konumla eşlemede yok' }], '', () => {});
    keySelect.disabled = matching !== 'key';
    replaceChildren(keyBox, field('Anahtar alan', keySelect));
    replaceChildren(
      fieldsBox,
      ...(fields.length
        ? fields.map((f) => {
            const box = h('input', { type: 'checkbox', checked: !ignored.has(f) }) as HTMLInputElement;
            box.addEventListener('change', () => (box.checked ? ignored.delete(f) : ignored.add(f)));
            return h('label', { class: 'io-check' }, box, f);
          })
        : [h('span', { class: 'compare-none' }, 'İki tarafın nesnelerinde öznitelik yok.')]),
    );
  }

  function list(): void {
    const r = result;
    shown = r ? r.rows.filter((row) => !onlyDiffs || row.status !== 'same') : [];
    rows.set(shown.length);
    for (const b of [copy, csv, write]) b.disabled = !r;
    replaceChildren(
      summary,
      r
        ? summaryLine(r.rows.some((row) => row.status !== 'same') ? 'info' : 'ok', `${countsText(r.rows)}.`)
        : summaryLine('info', 'Eski ve Yeni veriyi seçip Karşılaştır’a basın. Konumla eşlemede arama uzaklığı içindeki aynı türden nesneler eşlenir; tolerans içinde kalan geometri aynı sayılır.'),
    );
  }

  const dialog = new Dialog({
    title: COMPARE_TITLE,
    width: 920,
    className: 'dialog--io dialog--compare',
    content: [
      h('div', { class: 'io-row' }, sourceBox, oldBox, newBox),
      h('div', { class: 'io-row compare-run' }, matchBox, keyBox, field('Arama uzaklığı (m)', search), field('Tolerans (m)', tolerance), compare),
      h('div', { class: 'io-row' }, field('Karşılaştırılan öznitelikler', fieldsBox, undefined, 'grow'), field('Liste', h('label', { class: 'io-check' }, only, 'Yalnız farklar'))),
      summary,
      scroller,
      file,
    ],
    footer: [copy, csv, write, h('div', { class: 'dialog__spacer' }), close],
    onClose: () => rows.dispose(),
  });

  compare.addEventListener('click', () => {
    const [o, n] = sides();
    const number = (input: HTMLInputElement) => Number(input.value.trim().replace(',', '.'));
    const got = runCompare(o, n, { match: matching, key: key || null, search: number(search), tolerance: number(tolerance), ignore: [...ignored] });
    if (typeof got === 'string') return void log.warn(got);
    result = got;
    log.success(`Karşılaştırıldı: ${countsText(got.rows)}.`);
    list();
  });
  only.addEventListener('change', () => ((onlyDiffs = only.checked), list()));
  fromFile.addEventListener('click', () => file.click());
  file.addEventListener('change', () => {
    const f = file.files?.[0];
    file.value = '';
    if (!f) return;
    void f.arrayBuffer().then(async (buf) => {
      const read = await readDrawing(new Uint8Array(buf), { codec: ctx.files.kcad, identities: ctx.files.identities });
      if (!read.ok) return void log.warn(`“${f.name}” okunamadı: ${read.error}`);
      const s = read.content.settings;
      other = { name: f.name, here: false, tree: treeOf(read.content.layers), entities: read.content.entities, system: s.customCrs ? JSON.stringify(s.customCrs) : String(s.srid) };
      oldHere = false;
      oldNode = '';
      log.info(`“${f.name}” okundu: ${read.content.entities.length} nesne; Eski veri olarak seçildi.`);
      build();
    });
  });
  copy.addEventListener('click', () => {
    if (!result) return;
    const lines = reportRows(result, onlyDiffs);
    void navigator.clipboard.writeText(layerListTsv(lines)).then(
      () => log.success(`Karşılaştırma raporu panoya kopyalandı (${lines.length - 1} satır; elektronik tabloya yapıştırılabilir).`),
      () => log.warn('Rapor panoya kopyalanamadı: tarayıcı izin vermedi.'),
    );
  });
  csv.addEventListener('click', () => {
    if (!result) return;
    const lines = reportRows(result, onlyDiffs);
    const url = URL.createObjectURL(new Blob([layerListCsv(lines)], { type: 'text/csv;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = `${doc.name.value || 'cizim'}-karsilastirma.csv`;
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    log.success(`Karşılaştırma raporu CSV olarak kaydedildi: ${a.download} (${lines.length - 1} satır).`);
  });
  write.addEventListener('click', () => result && writeDifferences(ctx, result));
  close.addEventListener('click', () => dialog.close());
  build();
  list();
}
