import type { TableSheet } from '../contracts/generated/TableSheet';
import type { CellRange, TableAlign, TableSource } from './entities';
import { tableFromRows, type TableCells } from './ops/table';

/**
 * Tablo's rules on the page (docs/adr/0184 §1, §6), the contract's `TableShape::problem` and `cell_problem`
 * (crates/shared/contracts/src/table.rs), word for word: the commands refuse a table they break with
 * `invalid_table`, the snapshot reader with its place. The shared command cases (fixtures/commands/v1) hold the two
 * platforms to them.
 */

/** The most rows and columns a table may have, and cells in all; the most letters a cell may hold. */
export const MAX_TABLE_ROWS = 10_000;
export const MAX_TABLE_COLUMNS = 100;
export const MAX_TABLE_CELLS = 100_000;
export const MAX_CELL_LETTERS = 1000;
/** The longest a row or a column may be, and the largest text height, metres. */
export const MAX_TABLE_SIZE = 1e6;
/** The most objects a table's source may name. */
export const MAX_SOURCE_OBJECTS = 100_000;

const sizeHolds = (x: number): boolean => Number.isFinite(x) && x > 0 && x <= MAX_TABLE_SIZE;

/** Rust's `char::is_control`: the C0 and C1 controls (general category Cc). */
const isControl = (c: number): boolean => c <= 0x1f || (c >= 0x7f && c <= 0x9f);

/** A text's letters as Rust counts them (Unicode scalar values). */
const letters = (s: string): number => [...s].length;

/** What is wrong with a cell's words, if anything: a line break or control character, too many letters. */
export function cellProblem(words: string): string | null {
  for (const ch of words) if (isControl(ch.codePointAt(0)!)) return 'satır sonu ya da denetim karakteri var; hücre tek satırdır';
  const n = letters(words);
  return n > MAX_CELL_LETTERS ? `${n} harf; en çok ${MAX_CELL_LETTERS}` : null;
}

/** The fields of a table the rules read. */
export interface TableShape {
  height: number;
  rows: readonly number[];
  columns: readonly number[];
  cells: readonly (readonly string[])[];
  merges?: readonly CellRange[];
  aligns?: readonly TableAlign[];
  frame?: number;
  source?: TableSource;
}

/** Rust's `{}` of a float as far as a refusal shows one: the shortest text that reads back, NaN, inf, -inf. */
const shown = (x: number): string => (Number.isFinite(x) ? String(x) : Number.isNaN(x) ? 'NaN' : x > 0 ? 'inf' : '-inf');

/**
 * What is wrong with a table: the field and the refusal's words; null when it may be written. Its rows and columns in
 * their bounds, each cell's words one line, merged ranges inside it, of more than one cell, apart, their other cells
 * empty; a frame narrower than half its width and depth; as many alignments as columns; a source of objects within
 * its bounds, a file's name not empty.
 */
export function tableProblem(t: TableShape): [string, string] | null {
  if (!sizeHolds(t.height)) return ['height', `Tablonun yazı yüksekliği ${shown(t.height)}; sıfırdan büyük ve sonlu olmalı.`];
  const n = t.rows.length;
  const m = t.columns.length;
  if (n === 0 || n > MAX_TABLE_ROWS) return ['rows', `Tablonun ${n} satırı var; en az 1, en çok ${MAX_TABLE_ROWS} olmalı.`];
  if (m === 0 || m > MAX_TABLE_COLUMNS) return ['columns', `Tablonun ${m} sütunu var; en az 1, en çok ${MAX_TABLE_COLUMNS} olmalı.`];
  if (n * m > MAX_TABLE_CELLS) return ['cells', `Tablonun ${n * m} hücresi var; en çok ${MAX_TABLE_CELLS} olmalı. Tabloyu bölün.`];
  for (const [i, h] of t.rows.entries()) if (!sizeHolds(h)) return ['rows', `Tablonun ${i + 1}. satırının yüksekliği ${shown(h)}; sıfırdan büyük ve sonlu olmalı.`];
  for (const [j, w] of t.columns.entries()) if (!sizeHolds(w)) return ['columns', `Tablonun ${j + 1}. sütununun genişliği ${shown(w)}; sıfırdan büyük ve sonlu olmalı.`];
  if (t.cells.length !== n) return ['cells', `Tablonun ${n} satırı var ama ${t.cells.length} satırlık hücre verildi; her satırın hücreleri verilmeli.`];
  for (const [i, row] of t.cells.entries()) {
    if (row.length !== m) return ['cells', `Tablonun ${i + 1}. satırında ${row.length} hücre var; sütun sayısı kadar (${m}) olmalı.`];
    for (const [j, words] of row.entries()) {
      const why = cellProblem(words);
      if (why) return ['cells', `${i + 1}. satırın ${j + 1}. hücresi: ${why}.`];
    }
  }
  const merges = t.merges ?? [];
  for (const [k, r] of merges.entries()) {
    if (r.rows === 0 || r.cols === 0 || r.rows * r.cols < 2 || r.row + r.rows > n || r.col + r.cols > m)
      return ['merges', `${k + 1}. birleşik alan (${r.row + 1}. satır, ${r.col + 1}. sütundan ${r.rows} × ${r.cols}) tablonun içinde ve birden çok hücre olmalı.`];
    const other = merges.slice(0, k).find((o) => o.row < r.row + r.rows && r.row < o.row + o.rows && o.col < r.col + r.cols && r.col < o.col + o.cols);
    if (other) return ['merges', `${k + 1}. birleşik alan, ${other.row + 1}. satır ${other.col + 1}. sütundaki birleşik alanla örtüşüyor; birleşik alanlar ayrı olmalı.`];
    for (let i = r.row; i < r.row + r.rows; i++)
      for (let j = r.col; j < r.col + r.cols; j++)
        if ((i !== r.row || j !== r.col) && t.cells[i][j] !== '')
          return ['merges', `${i + 1}. satırın ${j + 1}. hücresi birleşik bir alanın içinde ama boş değil; birleşik alanın yazısı sol üst hücresindedir.`];
  }
  if (t.frame !== undefined) {
    const f = t.frame;
    const w = t.columns.reduce((a, b) => a + b, 0);
    const d = t.rows.reduce((a, b) => a + b, 0);
    if (!(Number.isFinite(f) && f > 0 && 2 * f < Math.min(w, d)))
      return ['frame', `Tablonun çerçeve kalınlığı ${shown(f)}; sıfırdan büyük, tablonun eninin ve boyunun yarısından küçük olmalı.`];
  }
  if (t.aligns !== undefined && t.aligns.length !== m) return ['aligns', `Tablonun ${m} sütunu var ama ${t.aligns.length} hiza verildi; her sütunun hizası verilmeli.`];
  const s = t.source;
  if (s?.kind === 'file') {
    if (s.name.trim() === '' || [...s.name].some((c) => isControl(c.codePointAt(0)!))) return ['source', 'Tablonun kaynağı olan dosyanın adı boş olamaz.'];
    if (s.sheet !== undefined && s.sheet.trim() === '') return ['source', 'Tablonun kaynağı olan sayfanın adı boş olamaz.'];
  } else if (s && (s.objects.length === 0 || s.objects.length > MAX_SOURCE_OBJECTS))
    return ['source', `Tablonun kaynağı ${s.objects.length} nesne gösteriyor; en az 1, en çok ${MAX_SOURCE_OBJECTS} olmalı.`];
  return null;
}

/** Where a table's rows came from, as Öznitelikler says it (the desktop's `tables::source_words`). */
export function sourceWords(source: TableSource | undefined): string {
  if (!source) return 'Elle yazıldı';
  if (source.kind === 'file') return source.sheet === undefined ? source.name : `${source.name} › ${source.sheet}`;
  const what = source.kind === 'coordinates' ? 'Koordinat çizelgesi' : source.kind === 'areas' ? 'Alan çizelgesi' : 'Öznitelik tablosu';
  return `${what} (${source.objects.length} nesne)`;
}

/**
 * What is said of a sheet with more rows than a table holds: the reader stops one past them, so the rule's count would
 * be the reader's, not the sheet's (docs/adr/0184 §4; the desktop's `kentos_interaction::table::SHEET_CUT`).
 */
export const SHEET_CUT = "Sayfada 10.000'den çok satır var; bir tablo en çok 10.000 satırdır. Sayfayı bölün ya da fazla satırları silin.";

/**
 * A file sheet's rows as cells (the empty rows and columns after the last with words dropped); refused as `SHEET_CUT`
 * when the reader stopped past a table's rows (Tablo ekle, Tabloyu güncelle; the desktop's `sheet_cells`).
 */
export function sheetCells(sheet: TableSheet, header: boolean): TableCells {
  const cells = tableFromRows(sheet.rows, header);
  return cells.problem && sheet.cut ? { ...cells, problem: SHEET_CUT } : cells;
}
