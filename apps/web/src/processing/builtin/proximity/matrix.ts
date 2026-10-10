import { defineTool } from '../../types';
import { MEASURE_OPTIONS, MOST_ROWS, PROXIMITY_KINDS, PROXIMITY_SCOPES, bound, lengthText, nameOf } from './shared';

/** The most targets a matrix takes, as columns. */
export const MOST_COLUMNS = 200;

/**
 * Uzaklık matrisi (docs/adr/0215 §3.2; ArcGIS Generate Near Table, QGIS "Distance matrix"): the distances from each
 * object to its nearest `k` targets (0: all), within a bound: a list (one pair a row, with its rank), a matrix (the
 * targets as columns) or a summary (each object's count, least, mean and most). The search is the run's store's
 * (`Store::nearest`). Edits nothing.
 */
export const proximityMatrix = defineTool({
  id: 'proximity.matrix',
  label: 'Uzaklık matrisi',
  category: 'proximity',
  icon: 'distanceMatrix',
  description: 'Her nesneden hedeflere uzaklıkları tablo olarak verir: en yakın k hedef ya da hepsi; liste, matris ya da özet.',
  help: [
    'Örnekler: her mahalleden en yakın üç sağlık ocağına uzaklıklar; okullar arası uzaklık matrisi; her parselin yollara en kısa uzaklıklarının özeti.',
    'Liste her çifti bir satıra yazar (Kaynak, Hedef, Sıra, Uzaklık); Matris kaynakları satır, hedefleri sütun yapar (en çok 200 hedef); Özet her kaynağın hedef sayısını, en az, ortalama ve en çok uzaklığını verir. En yakın k 0 ise bütün hedefler alınır.',
    'Adlar Ad alanından; boşsa nesnenin etiketi, o da yoksa listedeki sırası. Tablo pencerede Panoya kopyala ve CSV olarak kaydet ile alınır. Çizim değişmez.',
  ].join('\n\n'),
  keywords: ['uzaklık matrisi', 'mesafe tablosu', 'distance matrix', 'near table', 'en yakın k', 'yakınlık'],
  aliases: ['UZAKLIKMATRISI', 'MESAFETABLOSU'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Kaynaklar', type: 'features', kinds: PROXIMITY_KINDS, scopes: PROXIMITY_SCOPES, description: 'Uzaklıkları ölçülen nesneler.' },
    { name: 'targets', label: 'Hedefler', type: 'features', kinds: PROXIMITY_KINDS, scopes: PROXIMITY_SCOPES, description: 'Uzaklığı ölçülen hedefler.' },
    { name: 'measure', label: 'Ölçü', type: 'enum', options: MEASURE_OPTIONS, default: 'edges' },
    { name: 'k', label: 'En yakın k', type: 'number', integer: true, min: 0, max: 10000, default: 5, description: '0: bütün hedefler.' },
    { name: 'max', label: 'En çok uzaklık', type: 'number', unit: 'm', min: 0, default: 0, description: '0: sınırsız.' },
    {
      name: 'form',
      label: 'Biçim',
      type: 'enum',
      options: [
        { value: 'list', label: 'Liste', hint: 'Her çift bir satır: Kaynak, Hedef, Sıra, Uzaklık' },
        { value: 'matrix', label: 'Matris', hint: 'Kaynaklar satır, hedefler sütun' },
        { value: 'summary', label: 'Özet', hint: 'Her kaynağın hedef sayısı, en az, ortalama ve en çok uzaklığı' },
      ],
      default: 'list',
    },
    { name: 'name', label: 'Ad alanı', type: 'field', of: 'input', optional: true, description: 'Kaynakların tablodaki adı; boşsa etiket, o da yoksa sıra.' },
    { name: 'targetName', label: 'Hedef ad alanı', type: 'field', of: 'targets', optional: true, description: 'Hedeflerin tablodaki adı.' },
  ] as const,
  outputs: [
    { name: 'table', label: 'Uzaklık tablosu', type: 'table' },
    { name: 'count', label: 'Çift sayısı', type: 'number' },
  ],
  run: async (v, ctx, feedback) => {
    const inputs = v.input.entities;
    const targets = v.targets.entities;
    if (v.form === 'matrix' && targets.length > MOST_COLUMNS) return { refused: `Matris en çok ${MOST_COLUMNS} hedef alır; ${targets.length} hedef var. Liste biçimini seçin.` };
    const perInput = v.k > 0 ? Math.min(v.k, targets.length) : targets.length;
    if (v.form === 'list' && v.max <= 0 && inputs.length * perInput > MOST_ROWS)
      return { refused: `Tablo ${inputs.length * perInput} satır olurdu (en çok ${MOST_ROWS}); En yakın k ya da En çok uzaklık verin.` };
    feedback.progress(0, 'Uzaklıklar ölçülüyor');
    const found = ctx.geometry.nearest(
      inputs.map((e) => e.id),
      targets.map((e) => e.id),
      v.k,
      bound(v.max),
      v.measure,
    );
    if (found.length > MOST_ROWS) return { refused: `Tablo ${found.length} satır olurdu (en çok ${MOST_ROWS}); En yakın k ya da En çok uzaklık verin.` };
    const source = (i: number) => nameOf(inputs[i], v.name ?? '', i);
    const target = (j: number) => nameOf(targets[j], v.targetName ?? '', j);
    const length = (d: number) => lengthText(ctx.units, d);
    let columns: string[];
    const rows: string[][] = [];
    if (v.form === 'list') {
      columns = ['Kaynak', 'Hedef', 'Sıra', 'Uzaklık'];
      let last = -1;
      let rank = 0;
      for (const f of found) {
        rank = f.input === last ? rank + 1 : 1;
        last = f.input;
        rows.push([source(f.input), target(f.target), String(rank), length(f.d)]);
      }
    } else if (v.form === 'matrix') {
      columns = ['Kaynak', ...targets.map((_, j) => target(j))];
      const cells = inputs.map(() => targets.map(() => ''));
      for (const f of found) cells[f.input][f.target] = length(f.d);
      inputs.forEach((_, i) => rows.push([source(i), ...cells[i]]));
    } else {
      columns = ['Kaynak', 'Hedef sayısı', 'En az', 'Ortalama', 'En çok'];
      const lists = inputs.map((): number[] => []);
      for (const f of found) lists[f.input].push(f.d);
      lists.forEach((ds, i) => {
        if (!ds.length) return rows.push([source(i), '0', '', '', '']);
        let sum = 0;
        for (const d of ds) sum += d;
        rows.push([source(i), String(ds.length), length(ds[0]), length(sum / ds.length), length(ds[ds.length - 1])]);
      });
    }
    return {
      outputs: { table: { columns, rows }, count: found.length },
      summary: `${inputs.length} kaynaktan ${found.length} uzaklık ölçüldü.`,
    };
  },
});
