import type { TableFileRead } from '../contracts/generated/TableFileRead';
import { Signal } from '../core/signal';
import { applyTextStyle, FACE_FIELDS, type TextFace } from '../model/annotationStyles';
import type { Entity, TableEntity, TableGrid } from '../model/entities';
import { sheetCells } from '../model/tables';
import { tableRefresh, type ScheduleKind, type TableCells, type TableGeometry } from '../model/ops/table';
import { entitiesEdit, geometryOf } from '../product/entitiesEdit';
import { formats } from '../io/client';
import type { AppContext } from './context';
import { readFile } from './fileAccess';
import { BLANK_COLUMN, inOrder, listedOf, newTable, scheduleOf, sourceOf, tableUnits, type TableLook } from '../tools/newTable';

export { BLANK_COLUMN, inOrder, listedOf, newTable, scheduleOf, sourceOf, tableUnits, type TableLook };

/**
 * Tablo (docs/adr/0184 §3–§6) in the app: what Tablo ekle, its placement, Tabloyu düzenle and Tabloyu güncelle read
 * of the drawing and write to it; the desktop's `kentos_interaction::table` and `apps/desktop/src/tables/`. The rules
 * are the geometry core's (`ops::table`, `ops::table_edit`): a schedule's rows from objects in the order given, a
 * file's rows as cells, a new table's sizes, a source's new cells over a table. The windows are ui/table/.
 */

/** Where a new table's rows come from. */
export type InsertSource = 'blank' | 'file' | ScheduleKind;

/** The most rows and columns Boş tablo asks for. */
export const BLANK_MOST = 100;

/** A file Tablo ekle read: its name and sheets. */
export interface TableFile {
  readonly name: string;
  readonly read: TableFileRead;
}

/** What Tablo ekle remembers for as long as the app lives, the desktop's `TableInsert`. */
export const insertState = {
  source: 'blank' as InsertSource,
  rows: 4,
  columns: 3,
  header: true,
  /** The cells' height on paper, mm. */
  heightMm: 2.5,
  grid: 'all' as TableGrid | 'all',
  frame: false,
  /** Kalın çerçeve's width on paper, mm. */
  frameMm: 0.7,
  file: null as TableFile | null,
  sheet: 0,
  /** The schedule's objects, by persistent id, in the drawing's order. */
  objects: [] as string[],
};

/** Which objects a schedule of `kind` reads (Sahneden seç takes only these). */
export const SCHEDULE_KINDS: Readonly<Record<ScheduleKind, readonly Entity['kind'][] | undefined>> = {
  coordinates: ['point', 'line', 'polyline', 'polygon'],
  areas: ['polygon', 'circle', 'ellipse'],
  attributes: undefined,
};

/** Tablo ekle's look: the chosen style's face and fixed height, else the paper height; lines and frame at the plot scale. */
export function lookOf(ctx: AppContext, style: string | null, header: boolean): TableLook {
  const scale = ctx.doc.settings.plotScale.value;
  const picked = ctx.doc.settings.textStyle(style ?? undefined);
  const look = applyTextStyle(picked, { height: (insertState.heightMm / 1000) * scale }, scale);
  const face: TextFace = {};
  for (const k of FACE_FIELDS) if (look[k] !== undefined && look[k] !== null && look[k] !== false) (face as Record<string, unknown>)[k] = look[k];
  return {
    header,
    height: look.height,
    face,
    ...(insertState.grid !== 'all' && { grid: insertState.grid }),
    ...(insertState.frame && insertState.grid !== 'none' && { frame: (insertState.frameMm / 1000) * scale }),
  };
}


/** A blank table's cells: `rows` × `columns` empty ones. */
export const blankCells = (rows: number, columns: number): TableCells => ({
  cells: Array.from({ length: rows }, () => new Array<string>(columns).fill('')),
  aligns: new Array<string>(columns).fill('left'),
  fitted: 0,
});


/** The tables among these ids. */
export const tablesIn = (ctx: AppContext, ids: Iterable<number>): TableEntity[] => [...ids].flatMap((id) => {
  const e = ctx.doc.get(id);
  return e?.kind === 'table' ? [e] : [];
});

/** A source's new cells, or why there are none: its objects that are still in the drawing, in the source's order. */
export function sourceCells(ctx: AppContext, t: TableEntity): { cells: TableCells; missing: number } | { why: string } {
  const s = t.source;
  if (!s) return { why: 'Tablonun kaynağı yok: elle yazıldı ya da kaynağından ayrıldı.' };
  if (s.kind === 'file') return { why: `Tablo “${s.name}” dosyasından: güncellemek için dosyayı seçin.` };
  const objects = s.objects.flatMap((uid) => ctx.doc.byUid(uid) ?? []);
  return { cells: scheduleOf(ctx, s.kind, objects), missing: s.objects.length - objects.length };
}

/** A table with a source's new cells (Tabloyu güncelle); null when the core gives none. */
export const refreshed = (ctx: AppContext, t: TableEntity, cells: TableCells): TableGeometry | null =>
  tableRefresh(t as unknown as TableGeometry, cells.cells, cells.aligns, ctx.doc.settings.drawingFont.value);

/** Writes tables over tables in one step (`table`: Tabloyu düzenle, `tableUpdate`: Tabloyu güncelle); whether it did. */
export function writeTables(ctx: AppContext, operation: 'table' | 'tableUpdate', tables: readonly { uid: string; geometry: TableGeometry }[]): boolean {
  if (!tables.length) return false;
  const result = entitiesEdit.execute(
    { doc: ctx.doc },
    { operation, changes: tables.map((t) => ({ kind: 'update' as const, uid: t.uid, geometry: geometryOf(t.geometry as never) as never })) },
  );
  if (result.status !== 'completed') {
    if ('error' in result) ctx.log.warn(result.error.message);
    return false;
  }
  for (const w of result.warnings) ctx.log.warn(w.message);
  return true;
}

/** The one table selected, if the selection is one table. */
export function oneTable(ctx: AppContext): TableEntity | null {
  const ids = [...ctx.selection.ids.value];
  if (ids.length !== 1) return null;
  const e = ctx.doc.get(ids[0]);
  return e?.kind === 'table' ? e : null;
}

/** The tables Tabloyu güncelle rewrites: the selected ones with a source; with none selected, every one with a source. */
export function sourcedTables(ctx: AppContext): TableEntity[] {
  const chosen = tablesIn(ctx, ctx.selection.ids.value).filter((t) => t.source);
  if (ctx.selection.size > 0) return chosen;
  return [...ctx.doc.all()].filter((e): e is TableEntity => e.kind === 'table' && !!e.source);
}

/** Asks for a file the way Tablo ekle does; null when none is chosen. */
export function chooseTableFile(): Promise<File | null> {
  return new Promise((done) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.xlsx,.csv,.txt,.tsv,.XLSX,.CSV,.TXT,.TSV';
    input.addEventListener('change', () => done(input.files?.[0] ?? null), { once: true });
    input.addEventListener('cancel', () => done(null), { once: true });
    input.click();
  });
}

/** The most a table file may be (the core's `table_file::MAX_BYTES`): a larger one is not read at all. */
const MAX_FILE_BYTES = 64 * 1024 * 1024;

/** A table file's sheets through the formats worker; a failure is said and gives null. */
export async function readTableFile(ctx: AppContext, file: File): Promise<TableFileRead | null> {
  if (file.size > MAX_FILE_BYTES) return { sheets: [], problem: `Dosya ${MAX_FILE_BYTES / (1024 * 1024)} MB'tan büyük; bu kadar büyük tablo okunmuyor.` };
  try {
    const bytes = await readFile(file);
    return bytes ? await formats().readTableFile(bytes) : null;
  } catch (e) {
    ctx.log.error(`“${file.name}” okunamadı: ${e instanceof Error ? e.message : String(e)}`);
    return null;
  }
}

/** Same cells, alignments and sizes: nothing to write. */
const sameTable = (a: TableEntity, b: TableGeometry): boolean =>
  JSON.stringify([a.cells, a.rows, a.columns, a.aligns ?? null, a.merges ?? null]) === JSON.stringify([b.cells, b.rows, b.columns, b.aligns ?? null, b.merges ?? null]);

/**
 * Tabloyu güncelle (docs/adr/0184 §6): each table rewritten from its source in one step: a schedule from its objects
 * still in the drawing, a file's from the file chosen again (one question for each file); its look and the sizes given
 * by hand kept (`tableRefresh`). A table on a locked layer is left and said, one already up to date is not written.
 */
export async function updateTables(ctx: AppContext, read: (file: File) => Promise<TableFileRead | null>): Promise<void> {
  const all = sourcedTables(ctx);
  if (!all.length) return void ctx.log.info('Kaynağı olan tablo yok: Tabloyu güncelle çizelgeleri ve dosyadan gelen tabloları yeniden yazar.');
  const locked = all.filter((t) => ctx.doc.layers.isLocked(t.layerId));
  const tables = all.filter((t) => !ctx.doc.layers.isLocked(t.layerId));
  if (locked.length) ctx.log.warn(`${locked.length} tablo kilitli katmanda: güncellenmedi.`);
  const out: { uid: string; geometry: TableGeometry }[] = [];
  const files = new Map<string, TableFileRead | null>();
  for (const t of tables) {
    const s = t.source!;
    let cells: TableCells;
    if (s.kind === 'file') {
      if (!files.has(s.name)) {
        ctx.log.info(`“${s.name}” dosyasını seçin: tablo ondan yeniden yazılır.`);
        const file = await chooseTableFile();
        files.set(s.name, file ? await read(file) : null);
      }
      const book = files.get(s.name);
      if (!book) continue;
      if (book.problem) {
        ctx.log.warn(`“${s.name}”: ${book.problem}`);
        continue;
      }
      const sheet = s.sheet === undefined ? book.sheets[0] : book.sheets.find((x) => x.name === s.sheet);
      if (!sheet) {
        ctx.log.warn(`Dosyada “${s.sheet}” sayfası yok: tablo güncellenmedi.`);
        continue;
      }
      cells = sheetCells(sheet, t.header === true);
    } else {
      const got = sourceCells(ctx, t);
      if ('why' in got) continue;
      if (got.missing) ctx.log.warn(`Çizelgenin ${got.missing} nesnesi çizimde yok: tablo kalanlardan yazıldı.`);
      cells = got.cells;
    }
    if (cells.problem) {
      ctx.log.warn(cells.problem);
      continue;
    }
    const next = refreshed(ctx, t, cells);
    if (next && t.uid && !sameTable(t, next)) out.push({ uid: t.uid, geometry: next });
  }
  if (!out.length) {
    if (tables.length) ctx.log.info('Tablolar güncel.');
    return;
  }
  if (writeTables(ctx, 'tableUpdate', out)) ctx.log.success(`${out.length} tablo güncellendi.`);
}

/** The table commands: Tablo ekle, Tabloyu düzenle and Tabloyu güncelle (a CAD project's ribbon, the command line). */
export function registerTableCommands(ctx: AppContext, open: { insert: () => void; edit: (id: number) => void; read: (file: File) => Promise<TableFileRead | null> }): void {
  const cat = 'Tablo';
  // The drawing's objects changed: whether a table has a source may have.
  const drawn = new Signal(0);
  ctx.doc.events.on('touched', () => drawn.set(drawn.value + 1));
  ctx.doc.events.on('reset', () => drawn.set(drawn.value + 1));
  ctx.commands.register({
    id: 'table.insert',
    title: 'Tablo ekle',
    category: cat,
    icon: 'tableInsert',
    aliases: ['TABLO', 'TABLE', 'TB', 'TABLOEKLE'],
    description:
      'Çizime tablo ekler: boş, Excel (.xlsx) ya da CSV/TXT dosyasından, ya da seçili nesnelerin koordinat, alan veya öznitelik çizelgesi; yazı stili, yüksekliği, çizgileri ve kalın çerçevesiyle. Sol üst köşesine tıklanarak yerleştirilir.',
    run: () => open.insert(),
  });
  ctx.commands.register({
    id: 'table.edit',
    title: 'Tabloyu düzenle',
    category: cat,
    icon: 'tableEdit',
    aliases: ['TABLEDIT', 'TABLODUZENLE'],
    description: 'Seçili tablonun hücrelerini, satır ve sütunlarını, birleşik hücrelerini, hizasını, başlık satırını, çizgilerini ve çerçevesini düzenleyen pencereyi açar. Tabloya çift tıklamak da açar.',
    isEnabled: () => oneTable(ctx) !== null,
    whyDisabled: () => (oneTable(ctx) ? null : 'Bir tablo seçin (ya da tabloya çift tıklayın).'),
    watch: [ctx.selection.ids],
    run: () => {
      const t = oneTable(ctx);
      if (!t) return;
      if (ctx.doc.layers.isLocked(t.layerId)) return void ctx.log.warn('Kilitli katmandaki tablo düzenlenemez.');
      open.edit(t.id);
    },
  });
  ctx.commands.register({
    id: 'table.update',
    title: 'Tabloyu güncelle',
    category: cat,
    icon: 'tableUpdate',
    aliases: ['TABLOGUNCELLE', 'DATALINKUPDATE'],
    description:
      'Seçili tabloları (seçim yoksa kaynağı olan bütün tabloları) kaynaklarından yeniden yazar: çizelgeler nesnelerinin bugünkü koordinat, alan ve özniteliklerinden, dosyadan gelen tablo yeniden seçilen dosyadan; biçimi ve elle verilen boyları kalır. Tek adımda.',
    isEnabled: () => sourcedTables(ctx).length > 0,
    whyDisabled: () => (sourcedTables(ctx).length ? null : 'Kaynağı olan tablo yok: çizelgeler ve dosyadan gelen tablolar güncellenir.'),
    watch: [ctx.selection.ids, drawn],
    run: () => void updateTables(ctx, open.read),
  });
  // A double click on a table (the select tool's) opens Tabloyu düzenle.
  ctx.view.events.on('editText', ({ id }) => {
    if (ctx.doc.get(id)?.kind === 'table') open.edit(id);
  });
}
