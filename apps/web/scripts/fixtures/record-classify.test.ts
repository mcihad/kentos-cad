// Records the layer style window's classes into fixtures/style/v1/classify.json (style/classify.ts): an
// expression's distinct values and the categories made of them, numeric classes by equal interval and equal
// count coloured along a ramp, the objects each takes, the ramps' colours, the plain symbols a class starts
// with, class labels, the class count field and the window's texts. Runs only on purpose:
//   GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-classify.test.ts
// The answers are the web's and were read when recorded; rewriting them is a deliberate change, to be read in
// the diff. src/style/classifyFixture.test.ts keeps checking them; the desktop's layer style checks the same file.
// Outside src/ so the app's type check does not need Node's types.
import { writeFileSync } from 'node:fs';
import { it } from 'vitest';
import type { NewEntity } from '../../src/model/entities';
import { layerDocument } from '../../src/style/cases';
import {
  categoriesOf,
  categoryCounts,
  CLASS_COUNT,
  classCount,
  classesPresent,
  CLASSIFY_TEXTS,
  classLabel,
  countIn,
  DEFAULT_RAMP,
  graduatedOf,
  newCategory,
  numbersOf,
  OTHER_COLOR,
  plainSymbols,
  QUALITATIVE,
  RAMPS,
  rampColors,
  uniqueValues,
  valuesOf,
  type Category,
} from '../../src/style/classify';
import { withObjects } from '../../src/processing/geometry';

const OUT = new URL('../../../../fixtures/style/v1/classify.json', import.meta.url);
const O = { x: 487000, y: 4420000 };
const P = (x: number, y: number) => ({ x: O.x + x, y: O.y + y });
const rect = (x: number, w: number) => [P(x, 0), P(x + w, 0), P(x + w, 10), P(x, 10)];

/** Parcels of 100 … 600 m², a path, a point and a text, with attributes that read as text and as numbers. */
const DRAWING: NewEntity[] = [
  { kind: 'polygon', layerId: 'k', attrs: { Nitelik: 'Tarla', Kat: '3' }, pts: rect(0, 10) },
  { kind: 'polygon', layerId: 'k', attrs: { Nitelik: 'Arsa', Kat: '12' }, pts: rect(20, 20) },
  { kind: 'polygon', layerId: 'k', attrs: { Nitelik: 'Bahçe', Kat: '2' }, pts: rect(50, 30) },
  { kind: 'polygon', layerId: 'k', attrs: { Nitelik: 'Arsa', Kat: '2' }, pts: rect(90, 40) },
  { kind: 'polygon', layerId: 'k', attrs: { Nitelik: '', Kat: 'x' }, pts: rect(140, 50) },
  { kind: 'polygon', layerId: 'k', attrs: { Kat: '4' }, pts: rect(200, 60) },
  { kind: 'polyline', layerId: 'k', attrs: { Nitelik: 'Çayır' }, pts: [P(0, 20), P(25, 20)] },
  { kind: 'point', layerId: 'k', attrs: { Nitelik: 'arsa' }, p: P(30, 20) },
  { kind: 'text', layerId: 'k', attrs: { Nitelik: 'İmar' }, p: P(40, 20), text: 'İmar', height: 2, rotation: 0 },
];

const VALUES = ['Nitelik', 'Kat', '$alan', "Nitelik || '-' || Kat", '$katman', 'Nitelik ='];
const GRADUATED: { expr: string; method: 'interval' | 'count'; n: number; ramp: string }[] = [
  { expr: 'Kat', method: 'interval', n: 5, ramp: 'sariKirmizi' },
  { expr: 'Kat', method: 'count', n: 3, ramp: 'maviler' },
  { expr: '$alan', method: 'interval', n: 4, ramp: 'yesiller' },
  { expr: '$alan', method: 'count', n: 4, ramp: 'spektral' },
  { expr: '$alan', method: 'count', n: 1, ramp: 'griler' },
  { expr: 'Nitelik', method: 'interval', n: 5, ramp: 'sariKirmizi' },
];
const PRESENT = [
  { fill: 1, line: 0, marker: 0 },
  { fill: 0, line: 1, marker: 0 },
  { fill: 0, line: 0, marker: 1 },
  { fill: 2, line: 1, marker: 1 },
  { fill: 0, line: 0, marker: 0 },
];
const LABELS: { min: number; max: number; digits?: number }[] = [
  { min: 1.005, max: 30 },
  { min: 0.125, max: 2 / 3 },
  { min: -1.2345, max: -0.5 },
  { min: -0.125, max: 0.125 },
  { min: 1234567.891, max: 20000000 },
  { min: 2.5, max: 7.25, digits: 0 },
  { min: 0.123456, max: 0.5, digits: 4 },
];
const TYPED = ['7', '0', '-3', '25', 'abc', '2.5', '', '20', '1', ' ', '-0.4', 'Infinity', '1e3'];

it.runIf(!!process.env.GOLDEN_WRITE)('records the layer style window’s classes', () => {
  const doc = layerDocument('Kadastro', { color: 'fg', lineType: 'continuous', lineWeight: 0.25 }, DRAWING);
  const entities = [...doc.all()];
  const scope = { layerName: (id: string) => doc.layers.get(id)?.name ?? id, measures: (list: readonly (typeof entities)[number][]) => withObjects(list, (s) => s.measures(list.map((e) => e.id))) };
  const present = classesPresent(entities);
  const values = VALUES.map((expr) => {
    const r = valuesOf(entities, expr, scope);
    return { expr, values: r.values, unique: uniqueValues(r.values), error: r.error ?? null };
  });
  const found = uniqueValues(valuesOf(entities, 'Nitelik', scope).values);
  const made = categoriesOf(found, present);
  const kept: Category[] = [{ value: 'Arsa', label: 'ARSA', symbols: plainSymbols('#000000', present), enabled: false }];
  const categories = [
    { id: 'from-values', expr: 'Nitelik', old: [], expect: { categories: made, counts: categoryCounts(valuesOf(entities, 'Nitelik', scope).values, made) } },
    { id: 'keeps-existing', expr: 'Nitelik', old: kept, expect: { categories: categoriesOf(found, present, kept), counts: categoryCounts(valuesOf(entities, 'Nitelik', scope).values, categoriesOf(found, present, kept)) } },
  ];
  const graduated = GRADUATED.map((g) => {
    const r = numbersOf(entities, g.expr, scope);
    const classes = graduatedOf(r.values, g.method, g.n, g.ramp, present);
    return { ...g, expect: { numbers: r.values, classes, counts: classes.map((c, i) => countIn(r.values, c.min, c.max, i === classes.length - 1)) } };
  });
  const ramps = Object.keys(RAMPS).flatMap((ramp) => [1, 2, 3, 5, 7].map((n) => ({ ramp, n, colors: rampColors(RAMPS[ramp].stops, n) })));
  const file = {
    format: 'kentos.style-classify',
    version: 1,
    note: 'Katman stili penceresinin sınıfları (style/classify.ts): bir ifadenin farklı değerleri (sayılar değerine, yazılar Türkçe sırasına göre) ve onlardan yapılan kategoriler (QUALITATIVE renkleri sırayla; var olan kategori etiketini ve sembollerini korur), eşit aralık ya da eşit sayıyla sayısal sınıflar (eşit sınırlar birleşir) ve renk rampası boyunca renkleri, her sınıfın aldığı nesne (alt sınır dahil, üst sınır hariç; son sınıf üstü de alır), rampaların renkleri (iki renk arası doğrusal, kanal başına Math.round), bir sınıfın başladığı düz semboller (katmanda olan geometri sınıfları; hiçbiri yoksa üçü de), sınıf etiketleri (Math.round: yarım +∞ yönüne; sayı en kısa yazılışıyla), sınıf sayısı alanı (1–20, geçersizse 5) ve pencerenin sözleri. $alan geometri deposundan gelir. Yanıtlar web’indir ve kaydedilirken okunmuştur.',
    layerName: 'Kadastro',
    origin: O,
    entities,
    present,
    qualitative: QUALITATIVE,
    ramps: RAMPS,
    defaults: { classCount: CLASS_COUNT, ramp: DEFAULT_RAMP, method: 'interval', otherColor: OTHER_COLOR },
    texts: {
      ...CLASSIFY_TEXTS,
      found: { n: 7, text: CLASSIFY_TEXTS.found(7) },
      classified: { classes: 5, values: 6, text: CLASSIFY_TEXTS.classified(5, 6) },
    },
    values,
    categories,
    newCategory: [0, 1, 12, 13].map((count) => ({ count, category: newCategory(count, present) })),
    graduated,
    rampColors: ramps,
    plain: PRESENT.map((p) => ({ present: p, color: '#E15759', symbols: plainSymbols('#E15759', p) })),
    labels: LABELS.map((l) => ({ ...l, label: classLabel(l, l.digits) })),
    classCount: TYPED.map((typed) => ({ typed, count: classCount(typed) })),
  };
  writeFileSync(OUT, `${JSON.stringify(file, null, 1)}\n`);
});
