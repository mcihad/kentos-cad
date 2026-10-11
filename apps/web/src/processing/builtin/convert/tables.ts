import { convertBearingRows, convertEllipseRows, convertXyRows, type RowResult } from '../../../model/ops/convert';
import type { Entity } from '../../../model/entities';
import type { FileValue, Feedback, RunResult, TableOutput } from '../../types';
import { defineTool } from '../../types';
import { column, fromRows, outputLayer, readRows, sourceParams, unitParam } from './shared';

/**
 * The table tools (docs/adr/0218 §5–§7): lines from a place, a bearing and a length (ArcGIS Bearing Distance To
 * Line), lines from two places (XY To Line), ellipses from a centre, two semi-axes and a bearing (Table To Ellipse),
 * a row each, from a file or from objects' attributes. The cells are read and the shapes made in the core (one call
 * a run, docs/adr/0180's number rule); a row that gives nothing is in the report with its line (a file's) or object
 * number and why.
 */

/** A table tool's run: its rows, the core's results, the objects, the report. */
function run(
  v: { sourceKind: string; file: FileValue | null; source: { entities: readonly Entity[] } | null; layer: { id: string } },
  columns: readonly string[],
  names: readonly string[],
  make: (cells: string[][]) => RowResult[],
  what: string,
  feedback: Feedback,
): RunResult {
  const rows = readRows(v.sourceKind === 'layer' ? (v.source?.entities ?? []) : null, v.sourceKind === 'layer' ? null : v.file, columns);
  if ('refused' in rows) return { refused: rows.refused };
  feedback.progress(0, 'Satırlar okunuyor');
  const { add, report, summary } = fromRows(rows, make(rows.cells), names, v.layer.id, what);
  if (report.length) feedback.warn(`${report.length} satır atlandı; nedenleri tabloda.`);
  const table: TableOutput = { columns: ['Satır', 'Neden'], rows: report };
  return {
    changes: { add },
    outputs: report.length ? { count: add.length, table } : { count: add.length },
    summary,
  };
}

const OUTPUTS = [
  { name: 'count', label: 'Yazılan nesne sayısı', type: 'number' },
  { name: 'table', label: 'Atlanan satırlar', type: 'table' },
] as const;

export const bearingLine = defineTool({
  id: 'convert.bearingLine',
  label: 'Semt ve uzunluktan çizgi',
  category: 'conversion',
  icon: 'bearingLine',
  description: 'Bir tablonun her satırındaki başlangıç noktası, semt ve uzunluktan çizgi yazar.',
  help: [
    'Örnekler: bir aplikasyon karnesindeki doğrultu ve mesafelerden çizgiler, bir röper noktasından ölçülen semt ve uzunluklar, eski bir tapu krokisindeki sınırların semt ve uzunlukları.',
    'Semt kuzeyden saat yönünde, seçilen açı biriminde (grad ya da derece). Sayılar ondalık nokta ya da virgülle okunur. Boş ya da sayı olmayan hücreli, uzunluğu sıfır ya da eksi satır atlanır ve tabloda nedeniyle söylenir.',
    'Kaynak bir dosya (ilk satırı sütun adları) ya da nesneler olabilir; nesnelerde öznitelikler satırdır. Çizgi satırın dolu hücrelerini öznitelik olarak alır.',
  ].join('\n\n'),
  keywords: ['semt', 'uzunluk', 'açı', 'mesafe', 'doğrultu', 'çizgi', 'bearing distance to line', 'cogo'],
  aliases: ['SEMTUZUNLUK'],
  targets: ['client', 'worker'],
  parameters: [
    ...sourceParams('çizgi'),
    column('east', 'Doğu sütunu', 'Başlangıç noktasının doğusu (CBS projesinde Y).'),
    column('north', 'Kuzey sütunu', 'Başlangıç noktasının kuzeyi (CBS projesinde X).'),
    column('bearing', 'Semt sütunu', 'Kuzeyden saat yönünde doğrultu.'),
    column('length', 'Uzunluk sütunu', 'Çizginin uzunluğu (m).'),
    unitParam,
    outputLayer('Semt çizgileri', '#AB4ABA'),
  ] as const,
  outputs: OUTPUTS,
  run: (v, _ctx, feedback) => {
    const columns = [v.east, v.north, v.bearing, v.length].map((c) => (c ?? '').trim());
    return run(v, columns, columns, (cells) => convertBearingRows(cells, v.unit), 'çizgi', feedback);
  },
});

export const xyLine = defineTool({
  id: 'convert.xyLine',
  label: "XY'den çizgi",
  category: 'conversion',
  icon: 'xyLine',
  description: 'Bir tablonun her satırındaki başlangıç ve bitiş koordinatlarından çizgi yazar.',
  help: [
    'Örnekler: bir hesap tablosundaki kenarları, bir aplikasyon listesindeki bağlantıları, iki nokta listesinin eşleşmelerini çizgi olarak çizmek.',
    'Sayılar ondalık nokta ya da virgülle okunur. Boş ya da sayı olmayan hücreli satır ve başlangıcı ile bitişi aynı yerde olan satır atlanır ve tabloda nedeniyle söylenir. Çizgi satırın dolu hücrelerini öznitelik olarak alır.',
  ].join('\n\n'),
  keywords: ['xy', 'koordinat', 'çizgi', 'başlangıç', 'bitiş', 'xy to line'],
  aliases: ['XYCIZGI'],
  targets: ['client', 'worker'],
  parameters: [
    ...sourceParams('çizgi'),
    column('startEast', 'Başlangıç doğu', 'Başlangıcın doğusu (CBS projesinde Y).'),
    column('startNorth', 'Başlangıç kuzey', 'Başlangıcın kuzeyi (CBS projesinde X).'),
    column('endEast', 'Bitiş doğu', 'Bitişin doğusu.'),
    column('endNorth', 'Bitiş kuzey', 'Bitişin kuzeyi.'),
    outputLayer('Çizgiler', '#12A594'),
  ] as const,
  outputs: OUTPUTS,
  run: (v, _ctx, feedback) => {
    const columns = [v.startEast, v.startNorth, v.endEast, v.endNorth].map((c) => (c ?? '').trim());
    return run(v, columns, columns, (cells) => convertXyRows(cells), 'çizgi', feedback);
  },
});

export const tableEllipse = defineTool({
  id: 'convert.tableEllipse',
  label: 'Tablodan elips',
  category: 'conversion',
  icon: 'tableEllipse',
  description: 'Bir tablonun her satırındaki merkez, iki yarı eksen ve semtten elips yazar.',
  help: [
    'Örnekler: ağ dengelemesinin hata elipslerini (a, b ve büyük eksenin semti) çizmek, ölçü belirsizliklerini ya da etki alanlarını göstermek.',
    'Büyük eksen semt doğrultusundadır (kuzeyden saat yönünde, seçilen açı biriminde; hücre boşsa kuzey). Küçük yarı eksen büyükten büyükse ikisi yer değiştirir, doğrultu dik açı döner. Yarı ekseni sıfır ya da eksi satır atlanır.',
  ].join('\n\n'),
  keywords: ['elips', 'hata elipsi', 'yarı eksen', 'tablo', 'table to ellipse', 'error ellipse'],
  aliases: ['TABLODANELIPS'],
  targets: ['client', 'worker'],
  parameters: [
    ...sourceParams('elips'),
    column('east', 'Doğu sütunu', 'Merkezin doğusu (CBS projesinde Y).'),
    column('north', 'Kuzey sütunu', 'Merkezin kuzeyi (CBS projesinde X).'),
    column('major', 'Büyük yarı eksen', 'Büyük yarı eksen (m).'),
    column('minor', 'Küçük yarı eksen', 'Küçük yarı eksen (m).'),
    column('bearing', 'Semt sütunu', 'Büyük eksenin doğrultusu; boşsa kuzey.', true),
    unitParam,
    outputLayer('Elipsler', '#F76B15'),
  ] as const,
  outputs: OUTPUTS,
  run: (v, _ctx, feedback) => {
    const columns = [v.east, v.north, v.major, v.minor, v.bearing].map((c) => (c ?? '').trim());
    return run(v, columns, columns, (cells) => convertEllipseRows(cells, v.unit), 'elips', feedback);
  },
});
