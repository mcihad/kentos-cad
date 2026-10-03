import type { HAlign } from '../../../contracts/generated/sheet/HAlign';
import type { ItemKind } from '../../../contracts/generated/sheet/ItemKind';
import type { TableColumn } from '../../../contracts/generated/sheet/TableColumn';
import type { TitleCell } from '../../../contracts/generated/sheet/TitleCell';
import type { TitleRow } from '../../../contracts/generated/sheet/TitleRow';
import { h, type Child } from '../../dom';
import { icon } from '../../icons';
import { tooltip } from '../../widgets/tooltip';
import { area, flag, kinds, labelled, mapOptions, numField, pair, patchKind, patchKindEach, pick, shared, text, type SectionCtx } from './parts';

/**
 * The inspector's sections of the kinds that hold data (docs/sheet/design.md
 * §3.1): a table (fixed rows written in place, or a layer's objects
 * filtered, sorted, only those in a map; its columns: heading, value
 * expression, width, alignment), a coordinate list (the chosen objects' or
 * a layer's points, their names, decimals, the closing and area rows) and a
 * title block (each cell's caption and `[% … %]` value, a signature place).
 * The rows come from the drawing; the engine filters, sorts and lays out.
 */

type Table = Extract<ItemKind, { type: 'table' }>;

const ALIGN = [
  { value: 'left' as HAlign, label: 'Sol' },
  { value: 'center' as HAlign, label: 'Orta' },
  { value: 'right' as HAlign, label: 'Sağ' },
];

/** Fixed rows as text: one row per line, cells parted by “|”. */
const rowsText = (rows: readonly (readonly string[])[]) => rows.map((r) => r.join(' | ')).join('\n');
const textRows = (t: string, columns: number) =>
  t
    .split('\n')
    .filter((l) => l.trim())
    .map((l) => {
      const cells = l.split('|').map((x) => x.trim());
      return Array.from({ length: columns }, (_, i) => cells[i] ?? '');
    });

function columnsEditor(c: SectionCtx, cols: readonly TableColumn[], layer: boolean): Child {
  const put = (next: TableColumn[], label: string) => patchKindEach(c, 'table', label, (k: Table) => {
    // Fixed rows keep one cell per column.
    const source = k.source.type === 'fixed' ? { source: { type: 'fixed', rows: k.source.rows.map((r) => Array.from({ length: next.length }, (_, i) => r[i] ?? '')) } } : {};
    return { columns: next, ...source };
  });
  const rows = cols.map((col, i) => {
    const remove = h('button', { class: 'ibtn', type: 'button', 'aria-label': `${col.heading} sütununu kaldır`, disabled: c.readOnly !== null || cols.length < 2 }, icon('trash', 14));
    remove.addEventListener('click', () => put(cols.filter((_, j) => j !== i), 'Sütunu kaldır'));
    return h(
      'div',
      { class: 'sheet-col' },
      h('div', { class: 'sheet-col__head' }, h('span', { class: 'sheet-col__n num' }, String(i + 1)), remove),
      pair(
        text(c, 'Başlık', `col.${i}.heading`, col.heading, (heading) => put(cols.map((x, j) => (j === i ? { ...x, heading } : x)), 'Sütun başlığı')),
        layer ? text(c, 'Değer (ifade)', `col.${i}.value`, col.value, (value) => put(cols.map((x, j) => (j === i ? { ...x, value } : x)), 'Sütun değeri'), 'Parsel · [Tapu alanı (m²)] · $alan') : null,
      ),
      pair(
        numField(c, 'Genişlik', `col.${i}.width`, col.width / 1000, (w) => put(cols.map((x, j) => (j === i ? { ...x, width: Math.max(0, Math.round(w * 1000)) } : x)), 'Sütun genişliği'), { min: 0, decimals: 1, unit: 'mm' }),
        pick(c, 'Hizalama', `col.${i}.align`, col.align, ALIGN, (align) => put(cols.map((x, j) => (j === i ? { ...x, align } : x)), 'Sütun hizası')),
      ),
    );
  });
  const add = h('button', { class: 'btn btn--small', type: 'button', disabled: c.readOnly !== null }, icon('plus', 14), 'Sütun ekle');
  add.addEventListener('click', () => put([...cols, { heading: 'Yeni', value: '', width: 0, align: 'left' }], 'Sütun ekle'));
  return h('div', { class: 'sheet-cols' }, rows, h('div', { class: 'sheet-insp__actions' }, add), h('p', { class: 'sheet-insp__hint' }, 'Genişlik 0: içeriğe göre. Katman tablosunda değer bir ifadedir: alan adı yalın (Parsel) ya da köşeli parantezle ([Tapu alanı (m²)]), $alan, $uzunluk.'));
}

function tableSection(c: SectionCtx, layers: () => { id: string; name: string }[]): Child[] {
  const v = <V>(of: (k: Table) => V) => shared(c, 'table', of);
  const source = v((k) => k.source);
  const one = c.items.length === 1 ? kinds(c, 'table')[0] : null;
  const out: Child[] = [
    text(c, 'Başlık', 'table.title', v((k) => k.title), (title) => patchKind(c, 'Tablo başlığı', { title })),
    flag(c, 'Başlık satırı', v((k) => k.header), (header) => patchKind(c, 'Tablo', { header })),
    pick(c, 'Kaynak', 'table.source', source ? source.type : null, [
      { value: 'fixed', label: 'Sabit satırlar' },
      { value: 'layer', label: 'Katmanın nesneleri' },
      { value: 'continued', label: 'Önceki tablonun devamı' },
    ], (t) => patchKind(c, 'Tablo kaynağı', { source: t === 'fixed' ? { type: 'fixed', rows: [] } : t === 'layer' ? { type: 'layer', layer: layers()[0]?.id ?? '', filter: '', sort: [], atlasFilter: false } : { type: 'continued' } })),
  ];
  if (source?.type === 'fixed' && one?.source.type === 'fixed') {
    const cols = one.columns.length;
    out.push(area(c, 'Satırlar', 'table.rows', rowsText(one.source.rows), (t) => patchKind(c, 'Tablo satırları', { source: { type: 'fixed', rows: textRows(t, cols) } }), 5));
    out.push(h('p', { class: 'sheet-insp__hint' }, 'Her satır bir tablo satırı; hücreler “|” ile ayrılır. [% @ifade %] yazılabilir.'));
  }
  if (source?.type === 'layer' && one?.source.type === 'layer') {
    const src = one.source;
    out.push(
      pick(c, 'Katman', 'table.layer', src.layer, [...layers().map((l) => ({ value: l.id, label: l.name })), ...(layers().some((l) => l.id === src.layer) ? [] : [{ value: src.layer, label: `${src.layer} (projede yok)` }])], (layer) => patchKind(c, 'Tablonun katmanı', { source: { layer } })),
      text(c, 'Süzgeç (ifade)', 'table.filter', src.filter, (filter) => patchKind(c, 'Tablo süzgeci', { source: { filter } }), 'boş: hepsi'),
      pick(c, 'Yalnız şu haritadakiler', 'table.inMap', src.onlyInMap ?? '', mapOptions(c, 'Hepsi (haritaya bakmadan)'), (map) => patchKind(c, 'Tablo süzgeci', { source: { onlyInMap: map || null } })),
    );
  }
  if (one) out.push(labelled('Sütunlar', columnsEditor(c, one.columns, one.source.type === 'layer')));
  return out;
}

function coordinateSection(c: SectionCtx, layers: () => { id: string; name: string }[]): Child[] {
  const v = <V>(of: (k: Extract<ItemKind, { type: 'coordinateList' }>) => V) => shared(c, 'coordinateList', of);
  const source = v((k) => k.source);
  const naming = v((k) => k.naming);
  return [
    text(c, 'Başlık', 'coords.title', v((k) => k.title), (title) => patchKind(c, 'Koordinat listesi', { title })),
    pick(c, 'Noktalar', 'coords.source', source ? (source.type === 'layer' ? `layer:${source.layer}` : 'selection') : null, [
      { value: 'selection', label: 'Çizimde seçili nesneler', detail: 'Modelde seçtiğiniz parselin köşeleri, noktalar' },
      ...layers().map((l) => ({ value: `layer:${l.id}`, label: `Katman: ${l.name}` })),
    ], (val) => patchKind(c, 'Koordinat kaynağı', { source: val === 'selection' ? { type: 'selection' } : { type: 'layer', layer: val.slice(6) } })),
    pick(c, 'Nokta adları', 'coords.naming', naming ? naming.type : null, [
      { value: 'given', label: 'Kendi adları' },
      { value: 'sequence', label: 'Sıra numarası' },
    ], (t) => patchKind(c, 'Nokta adları', { naming: t === 'given' ? { type: 'given' } : { type: 'sequence', prefix: '', start: 1 } })),
    pair(
      numField(c, 'Ondalık', 'coords.decimals', v((k) => k.decimals), (d) => patchKind(c, 'Ondalık', { decimals: Math.max(0, Math.min(4, Math.round(d))) }), { min: 0, max: 4 }),
      pick(c, 'Taşma', 'coords.overflow', v((k) => k.overflow.type), [{ value: 'clip', label: 'Kes' }, { value: 'continueIn', label: 'Devamı başka tabloda' }], (t) => patchKind(c, 'Taşma', { overflow: t === 'clip' ? { type: 'clip' } : { type: 'continueIn', items: [] } })),
    ),
    flag(c, 'Z sütunu', v((k) => k.z), (z) => patchKind(c, 'Z sütunu', { z })),
    flag(c, 'Kapanış satırı', v((k) => k.closingRow), (closingRow) => patchKind(c, 'Kapanış satırı', { closingRow }), 'İlk nokta son satırda yeniden: kapalı şekil.'),
    flag(c, 'Alan satırı', v((k) => k.areaRow), (areaRow) => patchKind(c, 'Alan satırı', { areaRow }), 'Kapalı şeklin alanı, çizimin ölçtüğü gibi (yaylar dahil).'),
  ];
}

/** Every cell of a title block with its place (row, cell, and inside a cell's own rows). */
function cellsOf(rows: readonly TitleRow[], path: number[] = []): { path: number[]; cell: TitleCell }[] {
  return rows.flatMap((r, ri) => r.cells.flatMap((cell, ci) => [{ path: [...path, ri, ci], cell }, ...cellsOf(cell.rows, [...path, ri, ci])]));
}

/** The rows with one cell changed. */
function withCell(rows: readonly TitleRow[], path: readonly number[], change: (c: TitleCell) => TitleCell): TitleRow[] {
  const [ri, ci, ...rest] = path;
  return rows.map((r, i) =>
    i !== ri ? r : { ...r, cells: r.cells.map((cell, j) => (j !== ci ? cell : rest.length ? { ...cell, rows: withCell(cell.rows, rest, change) } : change(cell))) },
  );
}

function titleBlockSection(c: SectionCtx): Child[] {
  if (c.items.length !== 1) return [h('p', { class: 'sheet-insp__hint' }, 'Antetin hücreleri tek antet seçiliyken düzenlenir.')];
  const k = kinds(c, 'titleBlock')[0];
  const put = (path: number[], change: (x: TitleCell) => TitleCell, label: string) => patchKindEach(c, 'titleBlock', label, (t) => ({ rows: withCell(t.rows, path, change) }));
  return [
    h(
      'div',
      { class: 'sheet-cells' },
      cellsOf(k.rows).map(({ path, cell }) => {
        const key = `cell.${path.join('.')}`;
        const sign = h('button', { class: 'sheet-fx', type: 'button', 'aria-pressed': String(cell.signature), 'aria-label': 'İmza hücresi', disabled: c.readOnly !== null }, '✎');
        sign.addEventListener('click', () => put(path, (x) => ({ ...x, signature: !x.signature }), 'İmza hücresi'));
        c.d.add(tooltip(sign, () => ({ title: 'İmza hücresi', description: 'Değer (bir ad) imza çizgisinin altına yazılır.' })));
        return h(
          'div',
          { class: 'sheet-cell' },
          pair(
            text(c, 'Etiket', `${key}.label`, cell.label, (label) => put(path, (x) => ({ ...x, label }), 'Antet etiketi')),
            h('div', { class: 'sheet-cell__value' }, text(c, 'Değer', `${key}.value`, cell.value, (value) => put(path, (x) => ({ ...x, value }), 'Antet değeri'), '[% @ad %]'), sign),
          ),
        );
      }),
    ),
    h('p', { class: 'sheet-insp__hint' }, 'Değerlere [% @proje_adi %], [% @olcek %], [% @tarih %] ya da paftanın değişkenleri (Pafta → Değişkenler) yazılır.'),
  ];
}

export function dataSection(c: SectionCtx, kind: ItemKind['type'], layers: () => { id: string; name: string }[]): Child[] | null {
  switch (kind) {
    case 'table':
      return tableSection(c, layers);
    case 'coordinateList':
      return coordinateSection(c, layers);
    case 'titleBlock':
      return titleBlockSection(c);
    default:
      return null;
  }
}
