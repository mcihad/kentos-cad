import type { AppContext } from '../../app/context';
import type { FileKind } from '../../app/fileIO';
import type { WorkbookFormat } from '../../contracts/generated/WorkbookFormat';
import type { WorkbookSheet } from '../../contracts/generated/WorkbookSheet';
import { formats } from '../../io/client';
import { reportText } from '../io/common';
import { saveExport } from '../io/save';
import type { MenuItem } from '../widgets/PopupMenu';

/**
 * A table written as a file (docs/adr/0223 §2, §3): Öznitelik tablosu's Dışa aktar ▾ and İşlemler's Kaydet ▾, an
 * Excel workbook, an OpenDocument spreadsheet or a CSV file, by the formats core's writer; asked where (the export's
 * place), said in the log with what was written otherwise than its column says.
 */

/** The formats, as the menus list them. */
export const WORKBOOK_FORMATS: readonly { format: WorkbookFormat; label: string; icon: string; detail: string; kind: FileKind }[] = [
  {
    format: 'xlsx',
    label: 'Excel çalışma kitabı (.xlsx)',
    icon: 'exportXlsx',
    detail: 'Sayılar sayı, tarihler tarih olarak; başlık satırı kalın ve donmuş, süzgeçli',
    kind: { description: 'Excel çalışma kitabı', accept: { 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet': ['.xlsx'] } },
  },
  {
    format: 'ods',
    label: 'OpenDocument hesap tablosu (.ods)',
    icon: 'exportOds',
    detail: 'LibreOffice için; sayılar sayı, tarihler tarih olarak',
    kind: { description: 'OpenDocument hesap tablosu', accept: { 'application/vnd.oasis.opendocument.spreadsheet': ['.ods'] } },
  },
  {
    format: 'csv',
    label: 'CSV (.csv)',
    icon: 'exportCsv',
    detail: 'Noktalı virgülle ayrılmış, UTF-8; değerler yazıldığı gibi',
    kind: { description: 'CSV', accept: { 'text/csv': ['.csv'] } },
  },
];

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Writes `sheets` as `format` under the name `name` (without its extension): asked where, said in the log. */
export async function writeTable(ctx: AppContext, sheets: readonly WorkbookSheet[], format: WorkbookFormat, name: string): Promise<void> {
  const choice = WORKBOOK_FORMATS.find((f) => f.format === format)!;
  const rows = sheets.reduce((n, s) => n + s.rows.length, 0);
  let out;
  try {
    out = await formats().writeWorkbook({ format, sheets: [...sheets] });
  } catch (e) {
    ctx.log.error(`Tablo yazılamadı: ${message(e)}`);
    return;
  }
  const saved = await saveExport(ctx, out.bytes, `${name}.${format}`, choice.kind);
  if (!saved) return;
  ctx.log.success(`“${saved}” yazıldı: ${rows} satır.`);
  for (const n of out.report.notes) ctx.log.info(`“${saved}”: ${reportText(n)}`);
}

/** The menu's items: each format writing the table `sheet()` gives (none: nothing to write, said). */
export function exportItems(ctx: AppContext, sheet: () => WorkbookSheet | null, name: () => string): MenuItem[] {
  return WORKBOOK_FORMATS.map((f) => ({
    label: f.label,
    icon: f.icon,
    detail: f.detail,
    run: () => {
      const s = sheet();
      if (!s?.rows.length) return void ctx.log.info('Dışa aktarılacak satır yok.');
      void writeTable(ctx, [s], f.format, name());
    },
  }));
}
