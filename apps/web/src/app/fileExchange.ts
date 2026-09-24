import type { Command } from '../core/commands';
import type { AppContext } from './context';
import type { FileKind, PickedFile } from './fileIO';

/**
 * File exchange commands: coordinate lists (Netcad NCN, TXT, CSV) and DXF,
 * in and out. The dialogs, the formats worker and its Rust module load on first
 * use (CLAUDE.md §20): nothing of them is in the start-up bundle. Reading
 * and writing run in the worker (§6.2 rule 6); the source coordinate system
 * is always asked, never guessed, and nothing is reprojected (§5).
 */

export const COORD_FILES: FileKind = { description: 'Koordinat listesi (NCN, TXT, CSV)', accept: { 'text/plain': ['.ncn', '.txt', '.csv', '.xyz', '.dat', '.asc'] } };
export const DXF_FILES: FileKind = { description: 'AutoCAD DXF (DWG değil)', accept: { 'application/dxf': ['.dxf'] } };

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

function loadFailed(ctx: AppContext, what: string) {
  return (e: unknown) => ctx.log.error(`${what} penceresi yüklenemedi: ${message(e)}. Bağlantınızı denetleyip komutu yeniden çalıştırın.`);
}

/**
 * Picks the file first, straight from the click (the browser's file dialog
 * needs the user's gesture), while the dialog's code loads alongside.
 */
function importWith<M>(ctx: AppContext, kind: FileKind, what: string, load: () => Promise<M>, open: (m: M, file: PickedFile) => void): void {
  const file = ctx.files.pickForImport(kind);
  const ui = load();
  void Promise.all([file, ui]).then(([f, m]) => f && open(m, f), loadFailed(ctx, what));
}

export function registerFileExchangeCommands(ctx: AppContext): void {
  const coordImport = () =>
    importWith(ctx, COORD_FILES, 'Koordinat listesi', () => import('../ui/io/CoordImportDialog'), (m, file) => m.openCoordImport(ctx, file, COORD_FILES));
  const describeImport = 'Nokta adı, Y (sağa), X (yukarı) ve Z sütunlu bir listeyi (Netcad NCN, TXT, CSV) noktalar olarak içe aktarır. Ayırıcı, sütun sırası ve koordinat sistemi önizlemeyle seçilir; koordinatlar dönüştürülmez.';
  const list: Command[] = [
    {
      id: 'file.import.dxf',
      title: 'DXF…',
      category: 'Dosya',
      icon: 'import',
      description:
        "AutoCAD DXF (ASCII) çizimini içe aktarır: katmanlar, bloklar (patlatılarak), yaylı çoklu çizgiler, taramalar ve yazılar; alınamayanlar nedeniyle listelenir. DWG açılamaz: kapalı (tescilli) bir biçimdir; AutoCAD ya da Netcad'de DXF olarak kaydedip onu açın.",
      aliases: ['DXF', 'DXFAL', 'DXFIN'],
      run: () => importWith(ctx, DXF_FILES, 'DXF', () => import('../ui/io/DxfImportDialog'), (m, file) => m.openDxfImport(ctx, file, DXF_FILES)),
    },
    {
      id: 'file.import.ncn',
      title: 'Koordinat listesi (NCN, TXT, CSV)…',
      category: 'Dosya',
      icon: 'import',
      description: describeImport,
      aliases: ['NCN', 'KOORDINATAL', 'NOKTAAL', 'IMPORTXYZ'],
      run: coordImport,
    },
    {
      id: 'crs.points',
      title: 'Nokta listesi içe aktar…',
      short: 'Nokta listesi',
      category: 'Koordinat',
      icon: 'import',
      description: describeImport,
      aliases: ['NOKTALISTESI'],
      run: coordImport,
    },
    {
      id: 'file.export.dxf',
      title: 'DXF…',
      category: 'Dosya',
      icon: 'export',
      description:
        "Seçili, görünen ya da bütün nesneleri katmanlarıyla AutoCAD 2007 DXF'i olarak yazar (UTF-8, metre): koordinatlar tam, yuvarlanmadan; ölçüler çizgi ve yazıya patlatılır. Etiket, öznitelik ve semboller KentOS verisi olarak yazılır ve KentOS'a geri okunur.",
      aliases: ['DXFYAZ', 'DXFVER', 'DXFOUT'],
      run: () => void import('../ui/io/DxfExportDialog').then((m) => m.openDxfExport(ctx)).catch(loadFailed(ctx, 'DXF')),
    },
    {
      id: 'file.export.ncn',
      title: 'Koordinat listesi (NCN, TXT, CSV)…',
      category: 'Dosya',
      icon: 'export',
      description: 'Noktaları (seçili, görünen ya da hepsi) ad, Y, X ve Z sütunlarıyla bir koordinat listesine yazar; değerler tam, yuvarlanmadan yazılır.',
      aliases: ['NCNYAZ', 'KOORDINATVER', 'EXPORTXYZ'],
      run: () => void import('../ui/io/CoordExportDialog').then((m) => m.openCoordExport(ctx)).catch(loadFailed(ctx, 'Koordinat listesi')),
    },
  ];
  ctx.commands.registerAll(list);
}
