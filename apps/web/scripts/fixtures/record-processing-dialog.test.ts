// Records the processing dialog's behaviour into fixtures/processing/v1/dialog.json (ui/processing/dialogPlan.ts):
// every built-in tool's and model's form, sessions on parcels.kcad (what the dialog shows when it opens and after
// each thing the user does, runs included), and tables of the pure rules (status line, Nerede çalışır, restoring
// stored values, kind chips, field tokens, insertion, number text, the expression line's icon). Runs only on purpose:
//   GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-processing-dialog.test.ts
// The answers are the web's and were read when recorded; rewriting them is a deliberate change, to be read in the
// diff. src/ui/processing/dialogFixture.test.ts keeps checking them; the desktop's tool window plays the same file.
// Outside src/ so the app's type check does not need Node's types.
import { readFileSync, writeFileSync } from 'node:fs';
import { it } from 'vitest';
import type { EntityKind } from '../../src/model/entities';
import { BUILTIN_TOOLS } from '../../src/processing/builtin';
import { BUILTIN_MODELS } from '../../src/processing/builtin/models';
import { modelAsTool } from '../../src/processing/modelRunner';
import type { ValidationIssue } from '../../src/processing/parameters';
import { ProcessingRunner, type TargetChoice } from '../../src/processing/runner';
import type { ExecutionTarget, FeaturesValue } from '../../src/processing/types';
import { builtinForms, fixtureTexts, loadDrawing, playSession, viewDelta, type SessionSpec } from '../../src/ui/processing/dialogFixture';
import { footerOf, startState, statusLine, targetsView, type DialogStatus } from '../../src/ui/processing/dialogPlan';
import { SCOPE_SHORT } from '../../src/ui/processing/dialogTexts';
import { fieldToken, insertText, kindsView, numberOfText, previewIcon, toggleKind } from '../../src/ui/processing/fieldPlan';
import { TARGET_LABEL, TARGET_SHORT } from '../../src/ui/processing/targets';

const DIR = new URL('../../../../fixtures/processing/v1/', import.meta.url);
const DOCUMENT = 'parcels.kcad';

const SESSIONS: SessionSpec[] = [
  {
    id: 'numbering',
    title: 'Köşe noktalarını numarala: başlangıç noktası istenince görünür, çalıştırınca eksik olduğu söylenir; nokta gösterilince pencere olduğu gibi döner; yazı yüksekliği ve gelişmiş ayarlar, hatalı ve düzeltilen uzunluk, çalıştırma, seçili olanı yeniden seçmek, geri alma, vazgeçilen nokta',
    open: { tool: 'points.numberVertices' },
    selection: [1, 2],
    available: ['client'],
    steps: [
      { do: { choose: { name: 'start', value: 'point' } } },
      { do: { run: true } },
      { do: { pick: { name: 'startPoint', point: { x: 487045, y: 4420030 } } } },
      { do: { choose: { name: 'output', value: 'text' } } },
      { do: { choose: { name: 'shared', value: false } } },
      { do: { toggleAdvanced: true } },
      { do: { type: { name: 'length', text: '99' } } },
      { do: { run: true } },
      { do: { type: { name: 'length', text: '8' } } },
      { do: { run: true } },
      { do: { choose: { name: 'output', value: 'text' } } },
      { do: { undo: true } },
      { do: { pick: { name: 'startPoint', point: null } } },
    ],
  },
  {
    id: 'numbering-inputs',
    title: 'Girdi: seçim boşken çalıştırma sorunu alanda gösterir; kapsamlar, tür çipleri, katman listesi, seçili kapsama yeniden basmak',
    open: { tool: 'points.numberVertices' },
    selection: [],
    available: ['client'],
    steps: [
      { do: { run: true } },
      { do: { toggleAdvanced: true } },
      { do: { scope: { name: 'input', scope: 'all' } } },
      { do: { kind: { name: 'input', kind: 'polyline' } } },
      { do: { scope: { name: 'input', scope: 'layer' } } },
      { do: { scopeLayer: { name: 'input', layerId: 'parsel' } } },
      { do: { kind: { name: 'input', kind: 'polygon' } } },
      { do: { kind: { name: 'input', kind: 'polygon' } } },
      { do: { scope: { name: 'input', scope: 'layer' } } },
      { do: { run: true } },
    ],
  },
  {
    id: 'layer-field',
    title: 'Hedef katman: mevcut katman seçmek, yeniye dönmek, yazılan adın mevcut bir katmanın adı olup olmadığı (Türkçe katlamalı), boş ad; mevcut adla yazılan katmana çalıştırma',
    open: { tool: 'points.numberVertices' },
    selection: [1],
    available: ['client'],
    steps: [
      { do: { choose: { name: 'layer', value: { layerId: 'mevcut' } } } },
      { do: { choose: { name: 'layer', value: { newName: 'Köşe noktaları' } } } },
      { do: { type: { name: 'layer', text: 'parsel ' } } },
      { do: { type: { name: 'layer', text: 'cizim' } } },
      { do: { type: { name: 'layer', text: 'Kose noktalari' } } },
      { do: { type: { name: 'layer', text: '  ' } } },
      { do: { type: { name: 'layer', text: 'MEVCUT NOKTALAR' } } },
      { do: { run: true } },
    ],
  },
  {
    id: 'layer-locked',
    title: 'Geçmişten açılış: kilitli hedef katman çalıştırınca söylenir; başka katman seçilince düzelir; verilen değerlerde gelişmiş bir değer varsayılandan farklı olduğu için Gelişmiş açık başlar; saklanan yer seçimi',
    open: { tool: 'points.numberVertices' },
    selection: [1],
    available: ['client', 'worker'],
    given: { input: { scope: 'selection' }, prefix: 'K', step: 2, start: 'first', layer: { layerId: 'kilitli' } },
    choice: 'worker',
    steps: [{ do: { run: true } }, { do: { choose: { name: 'layer', value: { layerId: 'mevcut' } } } }, { do: { target: 'auto' } }, { do: { toggleAdvanced: true } }],
  },
  {
    id: 'missing-layers',
    title: 'Artık olmayan katmanlar: kapsamın ve hedefin katmanı “—”, girdi boş; çalıştırınca ikisi de söylenir',
    open: { tool: 'points.numberVertices' },
    selection: [],
    available: ['client'],
    given: { input: { scope: 'layer', layerId: 'yok' }, layer: { layerId: 'yok' } },
    steps: [{ do: { run: true } }, { do: { scopeLayer: { name: 'input', layerId: 'kilitli' } } }],
  },
  {
    id: 'last-values',
    title: 'Son çalıştırmanın değerleri: uymayanlar varsayılana döner, uyanlar kalır; gelişmişteki değişmiş değer bölümü açar; burada olmayan yer seçimi; Varsayılanlar',
    open: { tool: 'points.numberVertices' },
    selection: [1],
    available: ['client'],
    last: { direction: 'left', prefix: 5, length: 40, tolerance: 0.01, input: { scope: 'all', kinds: ['circle'] }, layer: { newName: 3 }, extra: true },
    choice: 'worker',
    steps: [{ do: { run: true } }, { do: { reset: true } }],
  },
  {
    id: 'calculate',
    title: 'Öznitelik hesapla: alan adının notları ve listesi, ifadenin alan çipleri ve satırı, koşul, yer seçimi, arka planda çalıştırma ve geri alma',
    open: { tool: 'attributes.calculate' },
    selection: [1, 2, 3],
    available: ['client', 'worker'],
    steps: [
      { do: { type: { name: 'field', text: 'Ada ' } } },
      { do: { type: { name: 'field', text: 'Mahalle' } } },
      { do: { type: { name: 'field', text: '' } } },
      { do: { type: { name: 'field', text: 'Ada ' } } },
      { do: { type: { name: 'value', text: "Ada || '/' || Parsel" } } },
      { do: { toggleAdvanced: true } },
      { do: { type: { name: 'where', text: "Mahalle = 'x'" } } },
      { do: { type: { name: 'where', text: "Nitelik = 'Arsa'" } } },
      { do: { target: 'worker' } },
      { do: { run: true } },
      { do: { undo: true } },
    ],
  },
  {
    id: 'calculate-locked',
    title: 'Öznitelik hesapla: seçilenlerin hepsi kilitli katmandayken çalıştırma durur, sorun girdinin altında',
    open: { tool: 'attributes.calculate' },
    selection: [4],
    available: ['client'],
    steps: [{ do: { run: true } }, { do: { choose: { name: 'label', value: false } } }],
  },
  {
    id: 'select',
    title: 'İfadeyle seç: kapsamın ilk seçeneği Tümü, tür çipleri, görünen alan, yazarken hatalı ifade, seçime ekleyen çalıştırma (geri alınacak adım yok)',
    open: { tool: 'selection.byExpression' },
    selection: [3],
    view: [487000, 4420000, 487030, 4420035],
    available: ['client'],
    steps: [
      { do: { kind: { name: 'input', kind: 'point' } } },
      { do: { scope: { name: 'input', scope: 'visible' } } },
      { do: { type: { name: 'condition', text: 'Nitelik = ' } } },
      { do: { type: { name: 'condition', text: "Nitelik = 'Arsa'" } } },
      { do: { choose: { name: 'mode', value: 'add' } } },
      { do: { run: true } },
    ],
  },
  {
    id: 'edge-lengths',
    title: 'Kenar uzunluklarını yaz: önizleme, sınırı aşan ondalık, çalıştırma denemesi, Varsayılanlar, virgüllü sayı, çizgide çalıştırma',
    open: { tool: 'annotation.edgeLengths' },
    selection: [7],
    available: ['client', 'worker'],
    steps: [
      { do: { type: { name: 'decimals', text: '9' } } },
      { do: { run: true } },
      { do: { reset: true } },
      { do: { type: { name: 'decimals', text: '1,' } } },
      { do: { type: { name: 'decimals', text: '' } } },
      { do: { type: { name: 'decimals', text: '1' } } },
      { do: { choose: { name: 'side', value: 'inside' } } },
      { do: { run: true } },
    ],
  },
  {
    id: 'model',
    title: 'Parsel ölçü yazıları modeli: girdileri, adım adım çalıştırma, özet ve geri alma',
    open: { model: 'builtin.parcelSheet' },
    selection: [1, 2, 3],
    available: ['client'],
    steps: [{ do: { type: { name: 'prefix', text: 'K' } } }, { do: { run: true } }, { do: { undo: true } }],
  },
  {
    id: 'pick-scene',
    title:
      'Sahneden seç (ADR 0088): başlangıç köşesinin yanındaki düğmeyle nokta (seçenek ve nokta birlikte) ve vazgeçilen nokta; girdi nesneleri: alınmayan türe tıklama, noktanın üstüne tıklayınca oradaki alan, alan içi, sağdan sola kutuda yalnız alınan tür, yeniden tıklayınca çıkan, Enter ile Seçili; alınmayan türe tıklayınca kenarı en yakın alan ve Esc ile önceki seçim; tür çipiyle daraltılan türler ve korunan süzgeç; hiçbir şey seçmeden Enter Esc gibi; erimde kenar yokken noktaya tıklayınca içinde olduğu alan',
    open: { tool: 'points.numberVertices' },
    selection: [1],
    available: ['client'],
    steps: [
      { do: { pickChoice: { name: 'start', point: { x: 487052, y: 4420031 } } } },
      { do: { pickChoice: { name: 'start', point: null } } },
      { do: { scope: { name: 'input', scope: 'all' } } },
      {
        do: {
          pickObjects: {
            name: 'input',
            tolerance: 0.5,
            actions: [
              // The line: not a kind the numbering takes, and no area's edge within reach.
              { click: { x: 487050, y: 4420040 } },
              // A point on the corner of parcels 1 and 2: the parcel there (the first in the drawing's order), not the point.
              { click: { x: 487020, y: 4420000 } },
              // Inside parcel 3.
              { click: { x: 487052, y: 4420015 } },
              // Right to left, a crossing: it touches the polyline and the line; only the polyline is taken.
              { box: { from: { x: 487062, y: 4420058 }, to: { x: 486995, y: 4420035 } } },
              // Parcel 3 again: out.
              { click: { x: 487052, y: 4420015 } },
            ],
            end: 'done',
          },
        },
      },
      // Zoomed out (a 12 m aperture): a click on the line, whose kind is not taken, takes the parcel whose edge is nearest; then Esc.
      { do: { pickObjects: { name: 'input', tolerance: 12, actions: [{ click: { x: 487050, y: 4420040 } }], end: 'cancel' } } },
      { do: { scope: { name: 'input', scope: 'all' } } },
      { do: { kind: { name: 'input', kind: 'polyline' } } },
      {
        do: {
          pickObjects: {
            name: 'input',
            tolerance: 0.5,
            actions: [
              { box: { from: { x: 487062, y: 4420058 }, to: { x: 486995, y: 4420035 } } },
              { click: { x: 487030, y: 4420015 } },
            ],
            end: 'done',
          },
        },
      },
      { do: { scope: { name: 'input', scope: 'all' } } },
      { do: { pickObjects: { name: 'input', tolerance: 0.5, actions: [], end: 'done' } } },
      // Inside parcel 1, 1.04 m from both its edges: point 8 on its corner is under the pointer (a point reaches 1.5 m),
      // no edge is within the 1 m aperture, so the parcel the click is in is taken.
      { do: { pickObjects: { name: 'input', tolerance: 1, actions: [{ click: { x: 487001.04, y: 4420028.96 } }], end: 'done' } } },
      // Only polylines taken: a click inside parcel 2, away from its edges, takes nothing (the area it is in is not of a taken kind).
      { do: { scope: { name: 'input', scope: 'all' } } },
      { do: { kind: { name: 'input', kind: 'polyline' } } },
      { do: { kind: { name: 'input', kind: 'polygon' } } },
      { do: { pickObjects: { name: 'input', tolerance: 0.5, actions: [{ click: { x: 487032, y: 4420015 } }], end: 'done' } } },
    ],
  },
  {
    id: 'by-location',
    title: 'Konuma göre seç (ADR 0200): Uzaklıkta seçilince Uzaklık görünür, virgüllü uzaklık; başvuru seçimken seçim boşsa sorun başvurunun altında; başvuru katman olunca çalışır ve seçer',
    open: { tool: 'selection.byLocation' },
    selection: [],
    available: ['client'],
    steps: [
      { do: { choose: { name: 'relation', value: 'near' } } },
      { do: { type: { name: 'distance', text: '2,5' } } },
      { do: { run: true } },
      { do: { scope: { name: 'reference', scope: 'layer' } } },
      { do: { scopeLayer: { name: 'reference', layerId: 'mevcut' } } },
      { do: { run: true } },
    ],
  },
  {
    id: 'summary',
    title: 'Özet istatistik (ADR 0200): alan ve gruplama listeleri, çalıştırınca sonuç tablosu (Grup sütunu, Toplam satırı); bir değer değişince tablo kalkar; gruplamasız tek satır',
    open: { tool: 'statistics.summary' },
    selection: [],
    available: ['client'],
    steps: [
      { do: { scopeLayer: { name: 'input', layerId: 'parsel' } } },
      { do: { choose: { name: 'field', value: 'Parsel' } } },
      { do: { choose: { name: 'group', value: 'Nitelik' } } },
      { do: { run: true } },
      { do: { choose: { name: 'group', value: '' } } },
      { do: { run: true } },
    ],
  },
  {
    id: 'join-file',
    title: 'Anahtarla birleştir (ADR 0200): son değerlerde yalnız adı kalan dosya yeniden istenir; seçilen dosyanın sütunları anahtar ve aktarılacak alanlar listesinde; iki alan işaretlenir; çalıştırma ve geri alma',
    open: { tool: 'attributes.joinByField' },
    selection: [1, 2, 3],
    available: ['client'],
    last: { sourceKind: 'file', file: { name: 'eski.csv' } },
    steps: [
      { do: { scope: { name: 'target', scope: 'selection' } } },
      { do: { run: true } },
      {
        do: {
          choose: {
            name: 'file',
            value: {
              name: 'malikler.csv',
              rows: [
                ['Parsel', 'Malik', 'Hisse'],
                ['1', 'Ayşe Yılmaz', '1/2'],
                ['3', 'Mehmet Kaya', '1'],
                ['03', 'Tekrar Kayıt', '1'],
                ['5', 'Kimse', '1'],
              ],
            },
          },
        },
      },
      { do: { choose: { name: 'targetKey', value: 'Parsel' } } },
      { do: { choose: { name: 'fields', value: 'Malik' } } },
      { do: { choose: { name: 'fields', value: 'Malik, Hisse' } } },
      { do: { run: true } },
      { do: { undo: true } },
    ],
  },
];

/** Status lines: the run's state, whether Çalıştır was pressed, the problems. */
const STATUS: { title: string; status: DialogStatus; attempted: boolean; issues: ValidationIssue[] }[] = [
  { title: 'boşta', status: { kind: 'idle' }, attempted: false, issues: [] },
  { title: 'sorun var ama çalıştırılmadı', status: { kind: 'idle' }, attempted: false, issues: [{ param: 'length', message: '“Toplam uzunluk” en çok 30 olmalı.' }] },
  {
    title: 'çalıştırma denemesinden sonra iki alan',
    status: { kind: 'idle' },
    attempted: true,
    issues: [
      { param: 'length', message: '“Toplam uzunluk” en çok 30 olmalı.' },
      { param: 'pad', message: '“Doldurma karakteri” en çok 1 karakter olabilir.' },
    ],
  },
  { title: 'aracın kendi kuralı', status: { kind: 'idle' }, attempted: true, issues: [{ message: 'Toplam uzunluk önekten uzun olmalı; yoksa doldurma yapılamaz.' }] },
  { title: 'bu uygulamada çalışıyor', status: { kind: 'running', fraction: 0, label: '', where: 'client' }, attempted: true, issues: [] },
  { title: 'arka planda çalışıyor', status: { kind: 'running', fraction: 0.425, label: '', where: 'worker' }, attempted: true, issues: [] },
  { title: 'aracın adım yazısıyla', status: { kind: 'running', fraction: 0.5, label: 'Değerler hesaplanıyor', where: 'worker' }, attempted: true, issues: [] },
  { title: 'modelin adımı', status: { kind: 'running', fraction: 1 / 3, label: '2. adım (Kenar uzunluklarını yaz) çalışıyor' }, attempted: true, issues: [] },
  { title: 'ekleyen çalıştırma', status: { kind: 'ok', text: '2 nesnede 6 köşe numaralandı: P00001 – P00006; 2 ortak köşe komşularıyla tek numara aldı.', pick: [11, 12, 13, 14, 15, 16], selected: false, undo: true }, attempted: false, issues: [] },
  { title: 'değiştiren çalıştırma', status: { kind: 'ok', text: '3 nesnede “Ada” yazıldı.', pick: [1, 2, 3], selected: false, undo: true }, attempted: false, issues: [] },
  { title: 'seçen çalıştırma', status: { kind: 'ok', text: '2 / 9 nesne koşulu sağladı; seçimde 2 nesne var.', pick: [1, 2], selected: true, undo: false }, attempted: false, issues: [] },
  { title: 'hiçbir şey değiştirmeyen çalıştırma', status: { kind: 'ok', text: 'Yeni numara gerekmedi: bütün köşelerin numarası zaten var.', pick: [], selected: false, undo: false }, attempted: false, issues: [] },
  { title: 'çalışamadı', status: { kind: 'error', text: '“Köşe noktalarını numarala” bu ortamda çalıştırılamıyor (server gerekli).' }, attempted: true, issues: [] },
  { title: 'durduruldu', status: { kind: 'error', text: 'İşlem iptal edildi; çizim değişmedi.' }, attempted: true, issues: [] },
  {
    title: 'başlamadan durdu, sorun alanında',
    status: { kind: 'invalid', text: '“Alanlar”: seçili nesneler arasında uygun nesne yok. Önce nesneleri seçin ya da kapsamı değiştirin.' },
    attempted: true,
    issues: [{ param: 'input', message: '“Alanlar”: seçili nesneler arasında uygun nesne yok. Önce nesneleri seçin ya da kapsamı değiştirin.' }],
  },
  {
    title: 'başlamadan durdu, sorun artık alanda değil',
    status: { kind: 'invalid', text: '“Alanlar”: seçili nesneler arasında uygun nesne yok. Önce nesneleri seçin ya da kapsamı değiştirin.' },
    attempted: true,
    issues: [],
  },
];

/** Nerede çalışır for hosts: the desktop today (only in the app), the web (the page and a worker). */
const TARGETS: { title: string; declared: ExecutionTarget[]; available: ExecutionTarget[]; auto: ExecutionTarget | null; model: boolean; choice: TargetChoice }[] = [
  { title: 'yalnız uygulamada; Otomatik', declared: ['client', 'worker'], available: ['client'], auto: 'client', model: false, choice: 'auto' },
  { title: 'yalnız uygulamada; saklanan yer burada yok', declared: ['client', 'worker'], available: ['client'], auto: 'client', model: false, choice: 'worker' },
  { title: 'yalnız uygulamada; sunucu yakında', declared: ['client', 'server'], available: ['client'], auto: 'client', model: false, choice: 'client' },
  { title: 'hiçbir yerde çalışamaz', declared: ['server', 'postgis'], available: [], auto: null, model: false, choice: 'auto' },
  { title: 'model, yalnız uygulamada', declared: ['client'], available: ['client'], auto: null, model: true, choice: 'auto' },
  { title: 'sayfa ve worker; Otomatik küçük girdide', declared: ['client', 'worker'], available: ['client', 'worker'], auto: 'client', model: false, choice: 'auto' },
  { title: 'sayfa ve worker; Otomatik büyük girdide', declared: ['client', 'worker'], available: ['client', 'worker'], auto: 'worker', model: false, choice: 'auto' },
  { title: 'sayfa ve worker; worker seçili', declared: ['client', 'worker'], available: ['client', 'worker'], auto: 'client', model: false, choice: 'worker' },
  { title: 'sayfa ve worker; saklanan yer burada yok', declared: ['client', 'worker', 'server'], available: ['client', 'worker'], auto: 'client', model: false, choice: 'server' },
  { title: 'model, sayfa ve worker', declared: ['client', 'worker'], available: ['client', 'worker'], auto: null, model: true, choice: 'auto' },
];

/** Stored values (a last run, history) restored over a tool's defaults on the drawing. */
const RESTORE: { title: string; tool: string; stored: Record<string, unknown> }[] = [
  {
    title: 'hiçbiri uymuyor',
    tool: 'points.numberVertices',
    stored: {
      input: 'selection',
      direction: 'left',
      start: 'center',
      startPoint: { x: 1 },
      prefix: 5,
      length: '6',
      pad: null,
      first: null,
      shared: 'true',
      tolerance: null,
      output: 'lines',
      textHeight: false,
      layer: { name: 'Köşe noktaları' },
      unknown: 1,
    },
  },
  {
    title: 'uyuyor ama sorunlu',
    tool: 'points.numberVertices',
    stored: { input: { scope: 'ids', ids: [1, 2] }, startPoint: null, prefix: 'ABCDEFGHIJKLMNOPQ', length: 40, step: 1.5, textHeight: -1, layer: { layerId: 'yok' } },
  },
  { title: 'girdi türleri: aracın almadığı tür', tool: 'annotation.edgeLengths', stored: { input: { scope: 'all', kinds: ['circle'] }, suffix: ' m', minLength: 0.5 } },
  { title: 'girdi türleri: aracın aldığı tür', tool: 'annotation.edgeLengths', stored: { input: { scope: 'all', kinds: ['line'] } } },
  { title: 'kapsamı olmayan katman', tool: 'annotation.edgeLengths', stored: { input: { scope: 'layer' } } },
  { title: 'isteğe bağlı boş, zorunlu boş', tool: 'attributes.calculate', stored: { where: null, value: null, field: 7, empty: 'drop', label: 0 } },
  { title: 'türü belirtmeyen girdi', tool: 'selection.byExpression', stored: { input: { scope: 'visible', kinds: ['point', 'text'] }, mode: 'within', condition: '' } },
  { title: 'model girdisi', tool: 'model:builtin.parcelSheet', stored: { parcels: { scope: 'layer', layerId: 'parsel' }, prefix: '' } },
];

type Present = { kind: EntityKind; count: number }[];
const KINDS: { title: string; tool: string; value: FeaturesValue; present: Present; click: EntityKind }[] = [
  { title: 'süzgeç yok, iki tür', tool: 'points.numberVertices', value: { scope: 'all' }, present: [{ kind: 'polygon', count: 4 }, { kind: 'polyline', count: 1 }], click: 'polyline' },
  { title: 'bırakılan tür geri alınınca süzgeç kalkar', tool: 'points.numberVertices', value: { scope: 'all', kinds: ['polygon'] }, present: [{ kind: 'polygon', count: 4 }, { kind: 'polyline', count: 1 }], click: 'polyline' },
  { title: 'tek tür, süzgeçte: bırakılınca hiçbiri', tool: 'points.numberVertices', value: { scope: 'layer', layerId: 'parsel', kinds: ['polygon'] }, present: [{ kind: 'polygon', count: 3 }], click: 'polygon' },
  { title: 'hiçbiri seçiliyken', tool: 'points.numberVertices', value: { scope: 'layer', layerId: 'parsel', kinds: [] }, present: [{ kind: 'polygon', count: 3 }], click: 'polygon' },
  { title: 'süzgeçteki tür kapsamda yok', tool: 'points.numberVertices', value: { scope: 'selection', kinds: ['polyline'] }, present: [{ kind: 'polygon', count: 3 }], click: 'polygon' },
  { title: 'türü belirtmeyen araç, tek tür', tool: 'selection.byExpression', value: { scope: 'selection' }, present: [{ kind: 'point', count: 3 }], click: 'point' },
  {
    title: 'kapsamdaki sırayla',
    tool: 'selection.byExpression',
    value: { scope: 'all' },
    present: [
      { kind: 'point', count: 3 },
      { kind: 'polygon', count: 2 },
      { kind: 'line', count: 1 },
    ],
    click: 'polygon',
  },
];

const TOKENS = ['Ada', 'Tapu alanı', 've', 'Veya', 'Değil', 'boş', 'NULL', 'true', 'Doğru', '_x1', '1a', 'ÇİZGİ', 'Nitelik2', 'a-b', 'İl', 'x.y'];

const INSERTS: { text: string; start: number; end: number; insert: string }[] = [
  { text: '', start: 0, end: 0, insert: 'Ada' },
  { text: 'x', start: 1, end: 1, insert: 'Ada' },
  { text: 'x ', start: 2, end: 2, insert: 'Ada' },
  { text: 'metin(', start: 6, end: 6, insert: '$alan' },
  { text: 'yuvarla($alan,', start: 14, end: 14, insert: '2' },
  { text: 'Ada = 1', start: 0, end: 3, insert: '[Tapu alanı]' },
  { text: 'Ada = 1', start: 3, end: 3, insert: 've' },
  { text: 'a\tb', start: 2, end: 2, insert: 'c' },
  { text: 'Ada Parsel', start: 4, end: 10, insert: 'Nitelik' },
];

const NUMBERS = ['12', '12.5', '12,5', ' 7 ', '', ' ', 'abc', '1 000', '12,5,3', '-3', '0', '1,', 'Infinity'];

const LINES = [
  '2 / 3 nesne koşulu sağlıyor.',
  '0 / 3 nesne koşulu sağlıyor. “Mahalle” alanı bu nesnelerde yok.',
  'İlk nesnede (1): “600.00”.',
  'İlk nesnede (1): boş. 3 nesnede sonuç boş.',
  'İlk nesnede: “101/1”.',
  'Önizleme için uygun nesne yok.',
];

it.runIf(!!process.env.GOLDEN_WRITE)('records the processing dialog', async () => {
  const kcad = readFileSync(new URL(DOCUMENT, DIR), 'utf-8');
  const doc = loadDrawing(kcad);
  const runner = new ProcessingRunner({ doc, selectedIds: () => [], visibleBounds: () => null });
  const tools = new Map([...BUILTIN_TOOLS.map((t) => [t.id, t] as const), ...BUILTIN_MODELS.map((m) => [`model:${m.id}`, modelAsTool(m, (id) => BUILTIN_TOOLS.find((t) => t.id === id))] as const)]);

  const sessions = [];
  for (const s of SESSIONS) {
    const { views, picks } = await playSession(s, kcad);
    sessions.push({ ...s, opened: views[0], steps: s.steps.map((st, i) => ({ do: st.do, ...(picks[i] ?? {}), expect: viewDelta(views[i], views[i + 1]) })) });
  }

  const file = {
    format: 'kentos.processing-dialog',
    version: 1,
    document: DOCUMENT,
    texts: fixtureTexts(),
    scopes: SCOPE_SHORT,
    targetLabels: TARGET_LABEL,
    targetShort: TARGET_SHORT,
    forms: builtinForms(),
    sessions,
    status: STATUS.map((c) => ({ ...c, expect: { status: statusLine(c.status, c.attempted, c.issues), footer: footerOf(c.status) } })),
    targets: TARGETS.map((c) => ({ ...c, expect: targetsView({ declared: c.declared, available: new Set(c.available), auto: c.auto, model: c.model }, c.choice) })),
    restore: RESTORE.map((c) => {
      const tool = tools.get(c.tool)!;
      const state = startState(tool, undefined, c.stored, runner.defaults(), 'auto');
      return { ...c, expect: { values: state.values, advancedOpen: state.advancedOpen, issues: runner.validate(tool, state.values) } };
    }),
    kinds: KINDS.map((c) => {
      const def = tools.get(c.tool)!.parameters.find((p) => p.type === 'features')!;
      if (def.type !== 'features') throw new Error(c.tool);
      const after = toggleKind(c.value, c.present, c.click);
      return { ...c, expect: { before: kindsView(def, c.value, c.present), after, afterView: kindsView(def, after, c.present) } };
    }),
    tokens: TOKENS.map((name) => ({ name, token: fieldToken(name) })),
    inserts: INSERTS.map((c) => ({ ...c, expect: insertText(c.text, c.start, c.end, c.insert) })),
    numbers: NUMBERS.map((text) => ({ text, value: numberOfText(text) })),
    icons: LINES.map((text) => ({ text, icon: previewIcon(text) })),
  };
  writeFileSync(new URL('dialog.json', DIR), `${JSON.stringify(file, null, 1)}\n`);
});
