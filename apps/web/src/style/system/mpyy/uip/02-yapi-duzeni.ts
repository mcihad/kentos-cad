import { BLACK, circle, dashDots, shape, sheet, stroke, text } from '../dsl';

/**
 * UİP (EK-1d s.2–3) › Yapı düzeni ve yoğunlukları. The circles are
 * attribute labels (EK-1e: ÖZNİTELİK) placed in the block: 18 mm circle,
 * 0.4 mm line, 5 mm text (EK-1e s.39–43). Their slots come from the
 * object's fields and show the legend's placeholders ("…", "n") when a
 * field is empty. AÇIKLAMA 2: the notations may be used together or alone.
 */

export const yapiDuzeni = sheet('uip', ['Yapı düzeni ve yoğunlukları'], 'yapi-duzeni', 20);

const D = 18;
const LW = 0.4;
const T = 5;
/**
 * Slot rows, measured from EK-1d: top baseline 3.7 mm above the centre,
 * middle row's capitals centred, bottom baseline 6.5 mm below. A text marker
 * is centred on its box, whose baseline lies 0.35 em below the point.
 */
const TOP = 3.7 + 0.35 * T;
const MID = 0;
const BOTTOM = -6.5 + 0.35 * T;

const ring = () => circle(D, { stroke: BLACK, strokeWidth: LW });
const slot = (expr: string, y: number) => text({ expr }, T, { font: 'sans', weight: 700, offset: [0, y] });

const FIELDS = 'Alanlar: "Düzen" (A, B, BL), "Kat" (kat adedi), "Ön bahçe" ve "Yan bahçe" (m); boş alan lejanttaki yer tutucuyla gösterilir.';

/**
 * The yapı düzeni circle: ön bahçe at the top, düzen and kat adedi in the
 * middle ("A-4"), yan bahçe at the bottom. `mid` is the middle row's
 * expression; `top`/`bottom` the placeholders of the garden slots.
 */
const duzen = (mid: string, top = '…', bottom = '…') => [ring(), slot(`varsayılan([Ön bahçe], '${top}')`, TOP), slot(mid, MID), slot(`varsayılan([Yan bahçe], '${bottom}')`, BOTTOM)];

/** Middle row with a fixed düzen letter: "A-" then the storeys (placeholder "…"). */
const fixed = (letter: string) => `'${letter}-' || varsayılan([Kat], ' …')`;
/** Middle row from the fields; `kat` is the storey placeholder. */
const free = (kat: string) => `eğer(boş([Düzen]) ve boş([Kat]), '…  ${kat === '…' ? '…' : '-' + kat}', varsayılan([Düzen], '…') || '-' || varsayılan([Kat], '${kat}'))`;

yapiDuzeni.point('ayrik-duzen', 'Ayrık düzen', [duzen(fixed('A'))], { ref: 'EK-1d s.2; EK-1e s.39', note: `Ortada "A-" ve kat adedi. ${FIELDS}` });
yapiDuzeni.point('bitisik-duzen', 'Bitişik düzen', [duzen(fixed('B'))], { ref: 'EK-1d s.2; EK-1e s.39', note: `Ortada "B-" ve kat adedi. ${FIELDS}` });
yapiDuzeni.point('blok-duzen', 'Blok düzen', [duzen(fixed('BL'))], { ref: 'EK-1d s.2; EK-1e s.40', note: `Ortada "BL-" ve kat adedi. ${FIELDS}` });

/** TAKS / KAKS: the circle halved by a horizontal diameter, the name in one half and the value in the other. */
const halved = (name: string, nameUp: boolean, valueExpr: string) => {
  // Capitals 1.5 mm off the diameter (EK-1d). Above: baseline at 1.5; below: cap top (0.716 em over the baseline) at -1.5.
  const up = 1.5 + 0.35 * T;
  const down = -(1.5 + 0.716 * T) + 0.35 * T;
  return [
    ring(),
    shape('line', D - LW, { stroke: BLACK, strokeWidth: LW }),
    text(name, T, { font: 'sans', weight: 700, offset: [0, nameUp ? up : down] }),
    text({ expr: valueExpr }, T, { font: 'sans', weight: 700, offset: [0, nameUp ? down : up] }),
  ];
};

yapiDuzeni.point('taban-alani-kat-sayisi-taks', 'Taban alanı kat sayısı (TAKS)', [halved('TAKS', true, `varsayılan([TAKS], '')`)], {
  ref: 'EK-1d s.2; EK-1e s.40',
  note: 'Üst yarıda "TAKS", alt yarıda değeri (alan "TAKS"; boşsa boş kalır).',
});
yapiDuzeni.point('kat-alani-kat-sayisi-kaks-emsal', 'Kat alanı kat sayısı (KAKS/Emsal)', [halved('KAKS', false, `varsayılan([KAKS], [Emsal], '')`)], {
  ref: 'EK-1d s.2; EK-1e s.41',
  note: 'Alt yarıda "KAKS", üst yarıda değeri (alan "KAKS", yoksa "Emsal"; boşsa boş kalır).',
});
yapiDuzeni.point('emsal', 'Emsal', [text({ expr: `'E = ' || varsayılan([Emsal], '')` }, T, { font: 'sans', weight: 400 })], {
  ref: 'EK-1d s.2; EK-1e s.41',
  note: 'Çerçevesiz "E =" ve değeri, 5 mm Arial (alan "Emsal").',
});
yapiDuzeni.point('kat-adedi', 'Kat adedi', [duzen(free('n'))], { ref: 'EK-1d s.2; EK-1e s.42', note: `Ortada düzen ve "-n" (kat adedi). ${FIELDS}` });

/**
 * "Yençok = …m": Y 7 mm, "ençok" 4 mm, Times as EK-1d draws it; "=" and the
 * value line are Y's size (not stated; measured). Laid out from EK-1d,
 * left edges and baselines relative to the composition's centre.
 */
const Y_BASE = 1.7;
yapiDuzeni.point(
  'bina-yuksekligi',
  'Bina yüksekliği',
  [
    text('Y', 7, { font: 'serif', weight: 700, anchor: 'left', offset: [-10.1, Y_BASE + 0.35 * 7] }),
    text('ençok', 4, { font: 'serif', weight: 400, anchor: 'left', offset: [-5.4, Y_BASE + 0.35 * 4] }),
    text('=', 7, { font: 'serif', weight: 400, anchor: 'left', offset: [5.8, 3.0 + 0.35 * 7] }),
    text({ expr: `varsayılan([Yençok], '…') || 'm'` }, 7, { font: 'serif', weight: 400, anchor: 'left', offset: [-5.4, -6.4 + 0.35 * 7] }),
  ],
  {
    ref: 'EK-1d s.2; EK-1e s.42',
    note: 'Y 7 mm, "ençok" 4 mm (EK-1e); "=" ve değer satırı 7 mm (çizimden). Alan "Yençok": metre ya da AÇIKLAMA 3 uyarınca kat adedi; boşsa "…m".',
  },
);
yapiDuzeni.point('on-bahce-mesafesi', 'Ön bahçe mesafesi', [duzen(free('…'), 'n')], { ref: 'EK-1d s.2; EK-1e s.43', note: `Üstte "n" (ön bahçe mesafesi). ${FIELDS}` });
yapiDuzeni.point('yan-bahce-mesafesi', 'Yan bahçe mesafesi', [duzen(free('…'), '…', 'n')], { ref: 'EK-1d s.2; EK-1e s.43', note: `Altta "n" (yan bahçe mesafesi). ${FIELDS}` });

yapiDuzeni.line('yapi-yaklasma-siniri', 'Yapı yaklaşma sınırı', [dashDots(BLACK, 0.4, { dash: 10, gap: 2, dots: 1, dot: 0.4, dotGap: 0 })], {
  ref: 'EK-1d s.2; EK-1e s.44',
  note: '0.4 mm: 10 mm çizgi, 2 mm ara, nokta, 2 mm ara (il sınırının "1 cm çizgi, 1 mm ara, nokta" yazımı gibi okundu). Nokta çapı verilmemiş; çizgi kalınlığı (0.4 mm) alındı. EK-1d çizimi kısa (≈2.8 mm) çizgiler gösteriyor; metin esas alındı.',
});
yapiDuzeni.line('kademe-hatti', 'Kademe hattı', [stroke(BLACK, 0.5)], { ref: 'EK-1d s.3; EK-1e s.44', note: '0.5 mm düz siyah çizgi.' });
yapiDuzeni.line('ifraz-hatti', 'İfraz hattı', [stroke(BLACK, 0.3, { dash: [2, 2] })], { ref: 'EK-1d s.3; EK-1e s.45', note: '0.3 mm: 2 mm çizgi, 2 mm ara.' });

/** Cephe çizgileri: "Çizgi kalınlığı yol genişliğine göre": the weight is a field; the fallback is EK-1d's drawing. */
const cephe = (fallback: number, dash?: readonly number[]) => ({ ...stroke(BLACK, fallback, { dash }), width: { expr: '[Çizgi kalınlığı]', fallback } });
const CEPHE_NOTE = 'Kalınlık "yol genişliğine göre" (EK-1e sayı vermiyor): "Çizgi kalınlığı" alanından (mm) okunur, boşsa EK-1d çiziminden ölçülen';
yapiDuzeni.line('korunan-cephe-cizgisi', 'Korunan cephe çizgisi', [cephe(0.5, [2, 2])], { ref: 'EK-1d s.3; EK-1e s.45', note: `2 mm çizgi, 2 mm ara. ${CEPHE_NOTE} 0.5 mm.` });
yapiDuzeni.line('duzeltilen-cephe-cizgisi', 'Düzeltilen cephe çizgisi', [cephe(0.55, [7, 2])], { ref: 'EK-1d s.3; EK-1e s.46', note: `7 mm çizgi, 2 mm ara. ${CEPHE_NOTE} 0.55 mm.` });
yapiDuzeni.line('onerilen-cephe-cizgisi', 'Önerilen cephe çizgisi', [cephe(0.8)], { ref: 'EK-1d s.3; EK-1e s.46', note: `Düz çizgi. ${CEPHE_NOTE} 0.8 mm.` });
