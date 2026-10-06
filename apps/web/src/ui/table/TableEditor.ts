import type { AppContext } from '../../app/context';
import { writeTables } from '../../app/tables';
import { DisposableStore, listen } from '../../core/disposable';
import { fixed } from '../../core/displayNumber';
import type { CellRange, TableAlign, TableEntity, TableGrid } from '../../model/entities';
import { tableEdit, tableSizes, type TableEdit, type TableGeometry } from '../../model/ops/table';
import { sourceWords } from '../../model/tables';
import { h, replaceChildren } from '../dom';
import { icon, type IconName } from '../icons';
import { askUnsaved } from '../widgets/confirm';
import { Dialog } from '../widgets/Dialog';
import '../../styles/table.css';

/** The window's title, which a trace names it by. */
export const EDITOR_TITLE = 'Tabloyu düzenle';

/** A table's own fields, without the object's. */
type Draft = Omit<TableEntity, 'id' | 'uid' | 'layerId' | 'attrs' | 'color' | 'label' | 'symbol' | 'lineWeight'>;

/** A cell: its row and column. */
interface Cell {
  row: number;
  col: number;
}

/** A column's name as a spreadsheet's: A … Z, AA, AB … */
export function columnName(j: number): string {
  let s = '';
  for (let n = j + 1; n > 0; n = Math.floor((n - 1) / 26)) s = String.fromCharCode(65 + ((n - 1) % 26)) + s;
  return s;
}

/** The rows the grid draws past the window either way, so a scroll does not show blanks. */
const OVERSCAN = 6;

/** The grid's row height, CSS px at the interface's scale (the stylesheet's `.table-grid td`). */
const ROW_PX = 26;

/** The table's fields a draft holds. */
function draftOf(t: TableEntity): Draft {
  const { id: _i, uid: _u, layerId: _l, attrs: _a, color: _c, label: _t, symbol: _s, lineWeight: _w, ...d } = t;
  return structuredClone(d);
}

/** The merged range holding a cell, if any. */
const rangeAt = (d: Draft, r: number, c: number): CellRange | undefined =>
  d.merges?.find((m) => r >= m.row && r < m.row + m.rows && c >= m.col && c < m.col + m.cols);

/**
 * Tabloyu düzenle (docs/adr/0184 §5; the desktop's `tables/editor.rs`): the table's cells in a grid as a spreadsheet's
 * (column letters, row numbers, the heading row bold, merged ranges as one cell), the active cell's words in the bar
 * above. Rows and columns are added and deleted, cells merged and unmerged (the core's `tableEdit`, refusals said in the
 * status line); a column's alignment, the heading row, Çizgiler and Kalın çerçeve, a column's width and a row's height,
 * Yazıya sığdır; Kaynaktan ayır. A column widens as its words ask. Ctrl+Z and Ctrl+Y undo and redo inside the window;
 * Kaydet writes the table in one step (`cad.entities.edit` `table`, “Tablo”).
 */
export function openTableEditor(ctx: AppContext, id: number): void {
  const original = ctx.doc.get(id);
  if (original?.kind !== 'table' || !original.uid) return;
  const uid = original.uid;
  const font = ctx.doc.settings.drawingFont.value;
  const scale = ctx.doc.settings.plotScale.value;
  const f = ctx.format;
  const d = new DisposableStore();
  let draft = draftOf(original);
  const saved = JSON.stringify(draft);
  const past: string[] = [];
  const future: string[] = [];
  let active: Cell = { row: 0, col: 0 };
  let anchor: Cell = { row: 0, col: 0 };
  let problem: string | null = null;

  const n = () => draft.rows.length;
  const m = () => draft.columns.length;
  const changed = () => JSON.stringify(draft) !== saved;

  // The parts.
  const toolbar = h('div', { class: 'table-tools', role: 'toolbar', 'aria-label': 'Tablo araçları' });
  const address = h('span', { class: 'table-bar__address' });
  const words = h('input', { class: 'field table-bar__input', 'aria-label': 'Hücrenin yazısı', spellcheck: 'false', 'data-escape': 'local' }) as HTMLInputElement;
  const head = h('thead');
  const body = h('tbody');
  const grid = h('table', { class: 'table-grid' }, head, body);
  const scroller = h('div', { class: 'table-grid__wrap', tabindex: '0', 'aria-label': 'Tablonun hücreleri' }, grid);
  const sizes = h('div', { class: 'io-row table-sizes' });
  const sourceRow = h('div', { class: 'table-source-row' });
  const status = h('div', { class: 'table-status', role: 'status' });
  const save = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet') as HTMLButtonElement;
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç') as HTMLButtonElement;

  /** The selected range: from the anchor to the active cell, grown to the merged ranges it touches. */
  function selected(): CellRange {
    let r0 = Math.min(anchor.row, active.row);
    let c0 = Math.min(anchor.col, active.col);
    let r1 = Math.max(anchor.row, active.row);
    let c1 = Math.max(anchor.col, active.col);
    for (let grown = true; grown; ) {
      grown = false;
      for (const g of draft.merges ?? []) {
        const meets = g.row <= r1 && g.row + g.rows - 1 >= r0 && g.col <= c1 && g.col + g.cols - 1 >= c0;
        if (!meets) continue;
        const [a, b, c, e] = [Math.min(r0, g.row), Math.min(c0, g.col), Math.max(r1, g.row + g.rows - 1), Math.max(c1, g.col + g.cols - 1)];
        if (a !== r0 || b !== c0 || c !== r1 || e !== c1) [r0, c0, r1, c1, grown] = [a, b, c, e, true];
      }
    }
    return { row: r0, col: c0, rows: r1 - r0 + 1, cols: c1 - c0 + 1 };
  }

  /** The cell holding a cell's words: a merged range's top left. */
  const owner = (c: Cell): Cell => {
    const g = rangeAt(draft, c.row, c.col);
    return g ? { row: g.row, col: g.col } : c;
  };

  /** Keeps a step for Ctrl+Z, then makes `next` the draft. */
  function take(next: Draft): void {
    past.push(JSON.stringify(draft));
    if (past.length > 200) past.shift();
    future.length = 0;
    draft = next;
    problem = null;
    clampActive();
    refresh();
  }

  /** One of the core's edits on the draft; a refusal is said in the status line. */
  function edit(e: TableEdit, grow = false): boolean {
    const out = tableEdit(draft as unknown as TableGeometry, e);
    if (!out.table) {
      problem = out.problem ?? 'Yapılamadı.';
      refresh();
      return false;
    }
    const next = { ...draft, ...(out.table as unknown as Draft) };
    if (!(out.table as unknown as Draft).merges) delete next.merges;
    if (grow) widen(next);
    take(next);
    return true;
  }

  /** Each column at least as wide as its words ask (a cell never spills); rows as they are. */
  function widen(t: Draft): void {
    const fit = tableSizes(t as unknown as TableGeometry, font);
    if (fit) t.columns = t.columns.map((w, j) => Math.max(w, fit.columns[j] ?? w));
  }

  function clampActive(): void {
    const clamp = (c: Cell): Cell => ({ row: Math.min(c.row, n() - 1), col: Math.min(c.col, m() - 1) });
    active = owner(clamp(active));
    anchor = clamp(anchor);
  }

  function undo(): void {
    const prev = past.pop();
    if (prev === undefined) return;
    future.push(JSON.stringify(draft));
    draft = JSON.parse(prev) as Draft;
    problem = null;
    clampActive();
    refresh();
  }

  function redo(): void {
    const next = future.pop();
    if (next === undefined) return;
    past.push(JSON.stringify(draft));
    draft = JSON.parse(next) as Draft;
    problem = null;
    clampActive();
    refresh();
  }

  /** Writes the active cell's words from the bar. */
  function commitWords(): void {
    const at = owner(active);
    const now = draft.cells[at.row]?.[at.col] ?? '';
    if (words.value === now) return;
    edit({ kind: 'setCell', row: at.row, col: at.col, words: words.value }, true);
  }

  /** Clears the selected cells' words, one step. */
  function clearSelected(): void {
    const r = selected();
    let next = draft;
    for (let i = r.row; i < r.row + r.rows; i++)
      for (let j = r.col; j < r.col + r.cols; j++) {
        if (!next.cells[i]?.[j]) continue;
        const out = tableEdit(next as unknown as TableGeometry, { kind: 'setCell', row: i, col: j, words: '' });
        if (out.table) next = { ...next, ...(out.table as unknown as Draft) };
      }
    if (next !== draft) take(next);
  }

  /** The columns' alignment for the selected columns. */
  function align(a: TableAlign): void {
    const r = selected();
    const aligns: TableAlign[] = draft.aligns ? [...draft.aligns] : new Array<TableAlign>(m()).fill('left');
    for (let j = r.col; j < r.col + r.cols; j++) aligns[j] = a;
    const next: Draft = { ...draft, aligns };
    if (aligns.every((x) => x === 'left')) delete next.aligns;
    take(next);
  }

  function setGrid(g: TableGrid | 'all'): void {
    const next = { ...draft };
    if (g === 'all') delete next.grid;
    else next.grid = g;
    take(next);
  }

  function toggleFrame(): void {
    const next = { ...draft };
    if (next.frame !== undefined) delete next.frame;
    else next.frame = (0.7 / 1000) * scale;
    take(next);
  }

  function toggleHeader(): void {
    const next = { ...draft };
    if (next.header) delete next.header;
    else next.header = true;
    take(next);
  }

  function fit(): void {
    const s = tableSizes(draft as unknown as TableGeometry, font);
    if (s) take({ ...draft, rows: s.rows, columns: s.columns });
  }

  function detach(): void {
    const next = { ...draft };
    delete next.source;
    take(next);
  }

  /** The toolbar: each button its icon and what it does, disabled where it cannot. */
  function tools(): void {
    const r = selected();
    const button = (name: IconName, title: string, run: () => void, opts: { on?: boolean; off?: boolean } = {}) => {
      const b = h('button', { class: 'ibtn table-tool', type: 'button', title, 'aria-label': title, 'aria-pressed': opts.on === undefined ? null : String(opts.on), disabled: opts.off }, icon(name, 18));
      b.addEventListener('click', () => {
        run();
        scroller.focus();
      });
      return b;
    };
    const gap = () => h('span', { class: 'table-tools__gap' });
    const colAlign = draft.aligns?.[active.col] ?? 'left';
    const grid = draft.grid ?? 'all';
    const merged = rangeAt(draft, active.row, active.col);
    replaceChildren(
      toolbar,
      button('tableRowAbove', 'Üstüne satır ekle', () => edit({ kind: 'insertRows', at: r.row, count: r.rows })),
      button('tableRowBelow', 'Altına satır ekle', () => edit({ kind: 'insertRows', at: r.row + r.rows, count: r.rows })),
      button('tableColumnLeft', 'Soluna sütun ekle', () => edit({ kind: 'insertColumns', at: r.col, count: r.cols }, true)),
      button('tableColumnRight', 'Sağına sütun ekle', () => edit({ kind: 'insertColumns', at: r.col + r.cols, count: r.cols }, true)),
      button('tableRowDelete', r.rows > 1 ? `${r.rows} satırı sil` : 'Satırı sil', () => edit({ kind: 'deleteRows', from: r.row, count: r.rows }), { off: r.rows >= n() }),
      button('tableColumnDelete', r.cols > 1 ? `${r.cols} sütunu sil` : 'Sütunu sil', () => edit({ kind: 'deleteColumns', from: r.col, count: r.cols }), { off: r.cols >= m() }),
      gap(),
      button('tableMerge', 'Hücreleri birleştir', () => edit({ kind: 'merge', ...r }), { off: r.rows * r.cols < 2 || (!!merged && merged.rows === r.rows && merged.cols === r.cols) }),
      button('tableUnmerge', 'Birleşimi ayır', () => edit({ kind: 'unmerge', row: active.row, col: active.col }), { off: !merged }),
      gap(),
      button('cellLeft', 'Sütunu sola hizala', () => align('left'), { on: colAlign === 'left' }),
      button('cellCenter', 'Sütunu ortala', () => align('center'), { on: colAlign === 'center' }),
      button('cellRight', 'Sütunu sağa hizala', () => align('right'), { on: colAlign === 'right' }),
      gap(),
      button('tableHeader', 'Başlık satırı: ilk satır kalın ve ortalı', toggleHeader, { on: draft.header === true }),
      gap(),
      button('tableGridAll', 'Çizgiler: tümü', () => setGrid('all'), { on: grid === 'all' }),
      button('tableGridOuter', 'Çizgiler: yalnız dış çizgi', () => setGrid('outer'), { on: grid === 'outer' }),
      button('tableGridRows', 'Çizgiler: dış çizgi ve satırlar', () => setGrid('rows'), { on: grid === 'rows' }),
      button('tableGridNone', 'Çizgiler: yok', () => setGrid('none'), { on: grid === 'none' }),
      button('tableFrame', 'Kalın çerçeve', toggleFrame, { on: draft.frame !== undefined, off: grid === 'none' }),
      gap(),
      button('tableFit', 'Yazıya sığdır: satır ve sütunlar yazıları kadar', fit),
      gap(),
      button('undo', 'Geri al (Ctrl+Z)', undo, { off: !past.length }),
      button('redo', 'Yinele (Ctrl+Y)', redo, { off: !future.length }),
    );
  }

  /** The grid's rows in view (and a few past), with spacers for the rest. */
  function rows(): void {
    const view = scroller.clientHeight || 320;
    const first = Math.max(0, Math.floor(scroller.scrollTop / ROW_PX) - OVERSCAN);
    const last = Math.min(n(), Math.ceil((scroller.scrollTop + view) / ROW_PX) + OVERSCAN);
    const r = selected();
    const inRange = (i: number, j: number) => i >= r.row && i < r.row + r.rows && j >= r.col && j < r.col + r.cols;
    const trs: HTMLElement[] = [];
    if (first > 0) trs.push(h('tr', { class: 'table-grid__spacer', 'aria-hidden': 'true' }, h('td', { colspan: String(m() + 1), style: `height:${first * ROW_PX}px` })));
    for (let i = first; i < last; i++) {
      const tds: HTMLElement[] = [h('th', { class: `table-grid__row${i >= r.row && i < r.row + r.rows ? ' table-grid__row--on' : ''}`, scope: 'row' }, String(i + 1))];
      for (let j = 0; j < m(); j++) {
        const g = rangeAt(draft, i, j);
        let rowspan = 1;
        let colspan = 1;
        if (g) {
          // A merged range is drawn from its top left, or from the first row in view when that is under it.
          const top = Math.max(g.row, first);
          if (i !== top || j !== g.col) continue;
          rowspan = Math.min(g.row + g.rows, last) - top;
          colspan = g.cols;
        }
        const at = g ? { row: g.row, col: g.col } : { row: i, col: j };
        const w = draft.cells[at.row]?.[at.col] ?? '';
        const heading = draft.header === true && at.row === 0;
        const alignment = heading ? 'center' : (draft.aligns?.[at.col] ?? 'left');
        const isActive = at.row === active.row && at.col === active.col;
        const td = h(
          'td',
          {
            class: ['table-grid__cell', `table-grid__cell--${alignment}`, heading ? 'table-grid__cell--head' : '', inRange(i, j) ? 'table-grid__cell--on' : '', isActive ? 'table-grid__cell--active' : ''].filter(Boolean).join(' '),
            rowspan: rowspan > 1 ? String(rowspan) : null,
            colspan: colspan > 1 ? String(colspan) : null,
            title: w.length > 24 ? w : null,
            dataset: { row: String(at.row), col: String(at.col) },
          },
          w,
        );
        tds.push(td);
      }
      trs.push(h('tr', { style: `height:${ROW_PX}px` }, tds));
    }
    if (last < n()) trs.push(h('tr', { class: 'table-grid__spacer', 'aria-hidden': 'true' }, h('td', { colspan: String(m() + 1), style: `height:${(n() - last) * ROW_PX}px` })));
    replaceChildren(body, ...trs);
  }

  function heads(): void {
    const r = selected();
    replaceChildren(
      head,
      h(
        'tr',
        null,
        h('th', { class: 'table-grid__corner' }),
        draft.columns.map((_, j) => h('th', { class: `table-grid__col${j >= r.col && j < r.col + r.cols ? ' table-grid__col--on' : ''}`, scope: 'col' }, columnName(j))),
      ),
    );
  }

  /** A number field of the sizes row; a typed value applies on Enter or when the field is left. */
  function number(label: string, value: number, unit: string, take: (v: number) => void, key: string): HTMLElement {
    const input = h('input', { class: 'field table-num', value: fixed(value, 2), inputmode: 'decimal', spellcheck: 'false', 'aria-label': label, dataset: { key } }) as HTMLInputElement;
    const commit = () => {
      const v = Number(input.value.trim().replace(',', '.'));
      if (Number.isFinite(v) && v > 0 && fixed(v, 2) !== fixed(value, 2)) take(v);
      else input.value = fixed(value, 2);
    };
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') commit();
      if (e.key === 'Escape') {
        e.preventDefault();
        input.value = fixed(value, 2);
        input.blur();
      }
    });
    input.addEventListener('blur', commit);
    return h('label', { class: 'table-size' }, h('span', { class: 'io-field__label' }, label), h('span', { class: 'table-size__value' }, input, h('span', { class: 'table-size__unit' }, unit)));
  }

  function sizesRow(): void {
    const r = selected();
    const unit = f.lengthUnitLabel;
    const toM = (v: number) => f.toMetres(v);
    const fromM = (v: number) => f.fromMetres(v);
    const colWord = r.cols > 1 ? `${columnName(r.col)}–${columnName(r.col + r.cols - 1)} genişliği` : `${columnName(r.col)} sütununun genişliği`;
    const rowWord = r.rows > 1 ? `${r.row + 1}–${r.row + r.rows} yüksekliği` : `${r.row + 1}. satırın yüksekliği`;
    replaceChildren(
      sizes,
      number(colWord, fromM(draft.columns[r.col]), unit, (v) => {
        const columns = [...draft.columns];
        for (let j = r.col; j < r.col + r.cols; j++) columns[j] = toM(v);
        take({ ...draft, columns });
      }, 'width'),
      number(rowWord, fromM(draft.rows[r.row]), unit, (v) => {
        const rows = [...draft.rows];
        for (let i = r.row; i < r.row + r.rows; i++) rows[i] = toM(v);
        take({ ...draft, rows });
      }, 'height'),
      draft.frame !== undefined ? number('Çerçeve', (draft.frame / scale) * 1000, 'mm', (v) => take({ ...draft, frame: (v / 1000) * scale }), 'frame') : null,
    );
  }

  function sourceLine(): void {
    if (!draft.source) return void replaceChildren(sourceRow, h('span', { class: 'table-source-row__text' }, icon('table', 16), 'Elle yazılmış tablo'));
    const off = h('button', { class: 'btn btn--small', type: 'button', title: 'Tablo kaynağını bırakır: Tabloyu güncelle ona dokunmaz.' }, 'Kaynaktan ayır');
    off.addEventListener('click', detach);
    replaceChildren(
      sourceRow,
      h('span', { class: 'table-source-row__text' }, icon(draft.source.kind === 'file' ? 'tableFile' : draft.source.kind === 'coordinates' ? 'tableCoordinates' : draft.source.kind === 'areas' ? 'tableAreas' : 'tableAttributes', 16), `Kaynak: ${sourceWords(draft.source)}`),
      h('span', { class: 'table-source-row__hint' }, 'Tabloyu güncelle elle yazılanların üstüne yazar.'),
      off,
    );
  }

  function bar(): void {
    const at = owner(active);
    const g = rangeAt(draft, at.row, at.col);
    address.textContent = g ? `${columnName(g.col)}${g.row + 1}:${columnName(g.col + g.cols - 1)}${g.row + g.rows}` : `${columnName(at.col)}${at.row + 1}`;
    if (document.activeElement !== words) words.value = draft.cells[at.row]?.[at.col] ?? '';
  }

  function statusLine(): void {
    if (problem) {
      status.dataset.kind = 'error';
      replaceChildren(status, icon('error', 14), problem);
      return;
    }
    const width = draft.columns.reduce((a, b) => a + b, 0);
    const depth = draft.rows.reduce((a, b) => a + b, 0);
    status.dataset.kind = 'info';
    replaceChildren(
      status,
      `${n().toLocaleString('tr-TR')} satır × ${m().toLocaleString('tr-TR')} sütun; ${f.length(width, false)} × ${f.length(depth, false)} ${f.lengthUnitLabel}. Çift tık ya da F2 hücreyi yazar, Shift ile alan seçilir.`,
    );
  }

  function refresh(): void {
    tools();
    heads();
    rows();
    sizesRow();
    sourceLine();
    bar();
    statusLine();
    save.disabled = !changed();
  }

  /** Moves the active cell by rows and columns, over merged ranges; with `extend` the anchor stays. */
  function move(dr: number, dc: number, extend = false): void {
    const g = rangeAt(draft, active.row, active.col);
    let row = active.row + dr;
    let col = active.col + dc;
    if (g && dr > 0) row = g.row + g.rows;
    if (g && dc > 0) col = g.col + g.cols;
    row = Math.max(0, Math.min(n() - 1, row));
    col = Math.max(0, Math.min(m() - 1, col));
    active = extend ? { row, col } : owner({ row, col });
    if (!extend) anchor = { ...active };
    problem = null;
    refresh();
    reveal();
  }

  /** Scrolls the active cell into view. */
  function reveal(): void {
    const top = active.row * ROW_PX;
    const head = (grid.tHead?.offsetHeight ?? ROW_PX) + 2;
    if (top < scroller.scrollTop) scroller.scrollTop = top;
    else if (top + ROW_PX + head > scroller.scrollTop + scroller.clientHeight) scroller.scrollTop = top + ROW_PX + head - scroller.clientHeight;
    const cell = body.querySelector<HTMLElement>(`[data-row="${active.row}"][data-col="${active.col}"]`);
    cell?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }

  /** Starts writing the active cell: the bar takes the focus, with `typed` in place of its words when given. */
  function write(typed?: string): void {
    const at = owner(active);
    words.value = typed ?? draft.cells[at.row]?.[at.col] ?? '';
    words.focus();
    const end = words.value.length;
    words.setSelectionRange(end, end);
  }

  // The grid: a click picks a cell, Shift+click a range, a drag a range; a double click writes it.
  let dragging = false;
  const cellOf = (t: EventTarget | null): Cell | null => {
    const el = (t as HTMLElement | null)?.closest?.<HTMLElement>('[data-row]');
    return el ? { row: Number(el.dataset.row), col: Number(el.dataset.col) } : null;
  };
  d.add(
    listen<PointerEvent>(body, 'pointerdown', (e) => {
      const c = cellOf(e.target);
      if (!c || e.button !== 0) return;
      if (document.activeElement === words) commitWords();
      active = c;
      if (!e.shiftKey) anchor = { ...c };
      dragging = true;
      problem = null;
      refresh();
      scroller.focus();
      e.preventDefault();
    }),
  );
  d.add(
    listen<PointerEvent>(body, 'pointerover', (e) => {
      if (!dragging) return;
      const c = cellOf(e.target);
      if (!c || (c.row === active.row && c.col === active.col)) return;
      active = c;
      refresh();
    }),
  );
  d.add(listen(window, 'pointerup', () => (dragging = false)));
  d.add(
    listen<MouseEvent>(body, 'dblclick', (e) => {
      if (cellOf(e.target)) write();
    }),
  );
  d.add(listen(scroller, 'scroll', () => rows()));
  d.add(
    listen<KeyboardEvent>(scroller, 'keydown', (e) => {
      const mod = e.ctrlKey || e.metaKey;
      if (mod && (e.key === 'z' || e.key === 'Z') && !e.shiftKey) return void (e.preventDefault(), undo());
      if (mod && (e.key === 'y' || e.key === 'Y' || ((e.key === 'z' || e.key === 'Z') && e.shiftKey))) return void (e.preventDefault(), redo());
      const arrows: Record<string, [number, number]> = { ArrowUp: [-1, 0], ArrowDown: [1, 0], ArrowLeft: [0, -1], ArrowRight: [0, 1] };
      if (arrows[e.key]) {
        e.preventDefault();
        move(...arrows[e.key], e.shiftKey);
      } else if (e.key === 'Tab') {
        e.preventDefault();
        move(0, e.shiftKey ? -1 : 1);
      } else if (e.key === 'Enter' || e.key === 'F2') {
        e.preventDefault();
        write();
      } else if (e.key === 'Delete' || e.key === 'Backspace') {
        e.preventDefault();
        clearSelected();
      } else if (e.key.length === 1 && !mod && !e.altKey) {
        e.preventDefault();
        write(e.key);
      }
    }),
  );
  // The bar: Enter writes and goes down, Tab writes and goes right, Esc takes the words back.
  d.add(
    listen<KeyboardEvent>(words, 'keydown', (e) => {
      if (e.key === 'Enter' || e.key === 'Tab') {
        e.preventDefault();
        commitWords();
        scroller.focus();
        if (e.key === 'Enter') move(e.shiftKey ? -1 : 1, 0);
        else move(0, e.shiftKey ? -1 : 1);
      } else if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        const at = owner(active);
        words.value = draft.cells[at.row]?.[at.col] ?? '';
        scroller.focus();
      }
    }),
  );
  d.add(listen(words, 'blur', () => commitWords()));

  function commit(): void {
    if (!changed()) return void dialog.close();
    if (writeTables(ctx, 'table', [{ uid, geometry: { ...draft, kind: 'table' } as unknown as TableGeometry }])) {
      ctx.log.success('Tablo kaydedildi. Ctrl+Z geri alır.');
      draft = JSON.parse(saved) as Draft;
      dialog.close();
    }
  }

  save.addEventListener('click', commit);
  cancel.addEventListener('click', () => dialog.request());
  const dialog = new Dialog({
    title: EDITOR_TITLE,
    width: 900,
    className: 'dialog--io dialog--table-editor',
    content: [toolbar, h('div', { class: 'table-bar' }, address, words), scroller, sizes, sourceRow, status],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, save],
    beforeClose: () => {
      if (!changed()) return true;
      void askUnsaved({ name: EDITOR_TITLE, after: 'Pencere kapanırsa bu değişiklikler kaybolur.', verb: 'kapat' }).then((a) => {
        if (a === 'save') commit();
        else if (a === 'discard') {
          draft = JSON.parse(saved) as Draft;
          dialog.close();
        }
      });
      return false;
    },
    onClose: () => d.dispose(),
  });
  refresh();
  queueMicrotask(() => {
    rows();
    scroller.focus();
  });
}
