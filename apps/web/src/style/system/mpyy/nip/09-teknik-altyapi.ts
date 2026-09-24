import { BLACK, WHITE, along, framed, grid, groupedHatch, hatch, rgb, shape, sheet, solid, stroke, svg, text, ticks } from '../dsl';
import { pic } from '../pictograms';
import { CAPTION, S, box, captionMark, pictureMarks, picture, stack, strips, turned } from './common';

/**
 * NİP (EK-1ç) › Teknik altyapı: ulaşım (karayolları, demiryolları,
 * denizyolları, kentsel toplu taşıma), enerji ve su-atıksu (EK-1e
 * s.147-192). Yollar gerçek ölçüleriyle çizilir; buradaki çizgi sembolleri
 * EK-1ç çizimindeki kesiti verir.
 */

const GREY = rgb(178, 178, 178);
const PORT = rgb(204, 204, 204);
const ENERGY = rgb(194, 158, 215);

export const teknik = sheet('nip', ['Teknik altyapı'], 'teknik', 90);

// ── Karayolları ────────────────────────────────────────────────────────

export const karayollari = sheet('nip', ['Teknik altyapı', 'Karayolları'], 'karayollari', 91);

/** Metres on the ground (an expression) as paper mm at the drawing's scale. */
const mm = (metres: string) => `(${metres}) * 1000 / $ölçek`;
/** Paper mm of `m` metres at 1/5000, the fallback when an expression gives nothing. */
const at5000 = (m: number) => m / 5;

/** A line `metres` (an expression) from the axis, `side` 1 left and −1 right. */
function sideLine(width: number, metres: string, m0: number, side: 1 | -1) {
  return { ...stroke(BLACK, width, { cap: 'butt' }), offset: { expr: `${side < 0 ? '-' : ''}${mm(metres)}`, fallback: side * at5000(m0) } };
}

/** A black band between `inner` and `outer` metres from the axis on `side` (the carriageway fill). */
function band(inner: string, outer: string, i0: number, o0: number, side: 1 | -1 | 0) {
  const width = side === 0 ? `2 * (${outer})` : `(${outer}) - (${inner})`;
  const centre = side === 0 ? '0' : `((${outer}) + (${inner})) / 2`;
  return {
    ...stroke(BLACK, at5000(side === 0 ? 2 * o0 : o0 - i0), { cap: 'butt' }),
    width: { expr: mm(width), fallback: at5000(side === 0 ? 2 * o0 : o0 - i0) },
    offset: { expr: `${side < 0 ? '-' : ''}${mm(centre)}`, fallback: side * at5000((o0 + i0) / 2) },
  };
}

/**
 * A NİP road drawn at its real width from its axis: 0.8 mm cephe lines at
 * the edges ("Genişlik", cepheden cepheye), 0.3 mm kaldırım lines inside
 * them by the sidewalk width ("Kaldırım"), 0.3 mm refüj lines ("Refüj",
 * the median width) and the carriageways between kaldırım and refüj
 * filled black (yol platformu 0/0/0). Sidewalks and the median stay
 * transparent. Widths are metres, turned into paper offsets at $ölçek.
 */
function road(w0: number, k0: number, r0 = 0) {
  const half = `varsayılan([Genişlik], ${w0}) / 2`;
  const kerb = `${half} - varsayılan([Kaldırım], ${k0})`;
  const median = r0 > 0 ? `varsayılan([Refüj], ${r0}) / 2` : '0';
  const h0 = w0 / 2;
  const kb0 = h0 - k0;
  const m0 = r0 / 2;
  const layers = [
    ...(r0 > 0 ? [band(median, kerb, m0, kb0, 1), band(median, kerb, m0, kb0, -1)] : [band('0', kerb, 0, kb0, 0)]),
    sideLine(0.8, half, h0, 1),
    sideLine(0.8, half, h0, -1),
    sideLine(0.3, kerb, kb0, 1),
    sideLine(0.3, kerb, kb0, -1),
  ];
  if (r0 > 0) layers.push(sideLine(0.3, median, m0, 1), sideLine(0.3, median, m0, -1));
  return layers;
}

const roadNote = (w0: number, k0: number, r0?: number) =>
  `Cephe çizgisi 0,8 mm, kaldırım${r0 ? ' ve refüj' : ''} çizgileri 0,3 mm, hepsi siyah; gerçek ölçülerinde çizilir. Çizgi yolun eksenidir: "Genişlik" (cepheden cepheye, m; boşsa ${w0}), "Kaldırım" (m; boşsa ${k0})${r0 ? `, "Refüj" (m; boşsa ${r0})` : ''} alanlarından çizim ölçeğine göre kaydırılır. Taşıt yolu (yol platformu, 0/0/0) kaldırım${r0 ? ' ve refüj' : ''} çizgileri arasında siyah doldurulur; kaldırımlar${r0 ? ' ve refüj' : ''} şeffaf kalır. Varsayılan genişlikler yönetmelikte yok, EK-1ç çizimindeki oranlardan. EK-1ç örneğindeki "…" yazılı elips kesme (devam) işaretidir, çizgi tipine dahil edilmedi.`;

karayollari.line('erisme-kontrollu-karayolu-otoyol', 'Erişme kontrollü karayolu (otoyol)', [road(40, 3, 4)], { ref: 'EK-1ç s.8; EK-1e s.147', note: roadNote(40, 3, 4) });
karayollari.line('birinci-derece-yol', 'Birinci derece yol', [road(30, 3, 2)], { ref: 'EK-1ç s.8; EK-1e s.148', note: roadNote(30, 3, 2) });
karayollari.line('ikinci-derece-yol', 'İkinci derece yol', [road(20, 3)], {
  ref: 'EK-1ç s.8; EK-1e s.149',
  note: `${roadNote(20, 3)} İkinci derece yolda refüj yok (EK-1e yalnız cephe ve kaldırım çizgisini sayar).`,
});
karayollari.area('genel-otopark', 'Genel otopark', [solid(GREY), stack(framed('P', { frame: 'rect', size: S, height: 5.6, strokeWidth: 0.25, textSize: 4.4, font: 'sans', weight: 400, background: WHITE }))], {
  ref: 'EK-1ç s.9; EK-1e s.152',
  note: 'Tarama yok. Çerçeve EK-1ç çizimindeki gibi yatay dikdörtgen, "P" ince Arial.',
});
karayollari.area('tir-kamyon-makine-parki-ve-garaj', 'Tır, kamyon, makine parkı ve garaj', [solid(rgb(255, 56, 0)), grid(2, 2, 0.2), picture('tir', 'PG')], {
  ref: 'EK-1ç s.9; EK-1e s.153',
  note: '0,2 mm, 2 mm ara ile karolaj.',
});

/** A straight piece from (x0, y0) to (x1, y1) of the symbol's own frame. */
function seg(x0: number, y0: number, x1: number, y1: number, width: number) {
  const deg = (Math.atan2(y1 - y0, x1 - x0) * 180) / Math.PI;
  return shape('line', Math.hypot(x1 - x0, y1 - y0), { stroke: BLACK, strokeWidth: width, rotation: deg, offset: turned((x0 + x1) / 2, (y0 + y1) / 2, deg) });
}
karayollari.line(
  'tunel',
  'Tünel',
  [stroke(BLACK, 0.4), along(seg(0, 0, -3 * Math.SQRT1_2, 3 * Math.SQRT1_2, 0.4), 0, { placement: 'first' }), along(seg(0, 0, 3 * Math.SQRT1_2, 3 * Math.SQRT1_2, 0.4), 0, { placement: 'last' })],
  {
    ref: 'EK-1ç s.9 (AÇIKLAMA 19); EK-1e s.154',
    note: '"Gerçek ölçüleri ile çizilir": tünelin her duvarı ayrı çizgi olarak gerçek yerinde çizilir; iki ucu 45° dışa kırılır (3 mm, EK-1ç\'den). Kanat çizim yönünün soluna döner: duvarı, dışı solda kalacak yönde çizin (UİP ile aynı kural). Kalınlık verilmemiş: 0,4 mm. Karayolu ve demiryolu tünellerinde kullanılır.',
  },
);
karayollari.area(
  'katli-otopark',
  'Katlı otopark',
  [solid(GREY), stack([...framed('', { size: S, strokeWidth: 0.3, background: WHITE }), text('P', 4.6, { font: 'sans', weight: 400, offset: [-0.6, 0.2] }), text('k', 2, { font: 'sans', weight: 400, offset: [1.4, -1.2] })])],
  { ref: 'EK-1ç s.9; EK-1e s.152', note: 'Tarama yok. Sembol çerçevede "P" ve alt simge "k".' },
);
karayollari.area(
  'elektrikli-arac-sarj-istasyonu-alani',
  'Elektrikli araç şarj istasyonu alanı',
  [solid(GREY), stack([...pictureMarks('sarj-istasyonu'), captionMark('EA', CAPTION, S, 'sans')])],
  { ref: 'EK-1ç s.9; EK-1e s.64', note: 'Tarama yok. Piktogramın yeşili 112/185/71 (EK-1ç\'de yazılı).' },
);
karayollari.area('elektronik-haberlesme-altyapi-alani', 'Elektronik haberleşme altyapı alanı', [solid(GREY), hatch(0, 6, 0.2), hatch(45, 2, 0.2), picture('anten')], {
  ref: 'EK-1ç s.9; EK-1e s.182',
  note: '0,2 mm, 6 mm aralıklı yatay çizgiler ve 2 mm aralıklı 45 derecelik çizgiler.',
});
karayollari.area(
  'motokurye-park-alani',
  'Motokurye park alanı',
  [solid(rgb(255, 183, 185)), stack([shape('rectangle', S, { height: 5.4, fill: WHITE, stroke: BLACK, strokeWidth: 0.5 }), shape('rectangle', 2.8, { height: 3.6, stroke: BLACK, strokeWidth: 0.2 }), svg(pic('motosiklet'), 2.5, { fill: BLACK })])],
  {
    ref: 'EK-1ç s.9; EK-1e s.192',
    note: 'Tarama yok. Sembol kalın dış çerçeve içinde ince çerçeveli motosiklet. Renk kodu 255/183/185 yazılı; EK-1ç renk kutusu 255/178/178 boyanmış, yazılı kod esas alındı.',
  },
);

// ── Demiryolları ───────────────────────────────────────────────────────

export const demiryollari = sheet('nip', ['Teknik altyapı', 'Demiryolları'], 'demiryollari', 92);

demiryollari.line('katar-duzenleme-triyaj-alani', 'Katar düzenleme (triyaj) alanı', [stroke(BLACK, 0.8), along(shape('rectangle', 5, { height: 2, fill: BLACK }), 7)], {
  ref: 'EK-1ç s.9; EK-1e s.158',
  note: '0,8 mm çizgi üzerinde 2×5 mm dolu dikdörtgenler, aralarında 2 mm düz çizgi. EK-1ç her hattı bu çizgiyle çizilmiş, makaslarla ayrılan bir hat demeti olarak gösteriyor.',
});

// ── Denizyolları ───────────────────────────────────────────────────────

export const denizyollari = sheet('nip', ['Teknik altyapı', 'Denizyolları'], 'denizyollari', 93);

const stripNote = (pair: number) =>
  `0,2 mm dik tarama çiftleri: çift çizgiler ${pair} mm arayla, çiftler arası 5 mm; çiftlerin içinde 330 derecelik (yataydan 30° sağa inen), 1 mm aralıklı çizgiler. "Çiftler arası ${pair} mm" EK-1ç/EK-1e çizimlerine göre çiftin iki çizgisi arası, "tarama arası 5 mm" çiftler arası okundu. Çizimlerde iç çizgiler daha dik (düşeyden 30°); metindeki 330° esas alındı.`;

denizyollari.area(
  'liman',
  'Liman',
  [solid(PORT), strips(5, 5), stack([...pictureMarks('capraz-capalar', 'L'), text({ expr: `eğer(boş([Liman türü]), '', '(' || büyük([Liman türü]) || ')')` }, 2.2, { font: 'serif', weight: 700, offset: [0, -S / 2 - 4.6], halo: { color: WHITE, width: 0.3 } })])],
  { ref: 'EK-1ç s.9 (AÇIKLAMA 17); EK-1e s.161', note: `${stripNote(5)} Liman türü "Liman türü" özniteliğinden parantez içinde büyük harfle yazılır.` },
);
denizyollari.area('balikci-barinagi', 'Balıkçı barınağı', [solid(PORT), strips(2, 5), picture('yelkenli-bos', 'BB')], { ref: 'EK-1ç s.10; EK-1e s.163', note: stripNote(2) });
denizyollari.area('iskele', 'İskele', [solid(PORT), strips(2, 5), picture('capa-halat', 'İ')], { ref: 'EK-1ç s.10; EK-1e s.164', note: stripNote(2) });
denizyollari.area('tekne-imal-ve-bakim-yeri', 'Tekne imal ve bakım yeri', [solid(PORT), strips(5, 5), picture('yelkenli-dolu', 'TİB')], { ref: 'EK-1ç s.10; EK-1e s.165', note: stripNote(5) });
denizyollari.area('tekne-imal-ve-cekek-yeri', 'Tekne imal ve çekek yeri', [solid(GREY), strips(2, 5), picture('yelkenli-dolu', 'ÇY')], {
  ref: 'EK-1ç s.10; EK-1e s.164',
  note: `${stripNote(2)} Renk 178/178/178 (öteki deniz yapılarından koyu).`,
});

/** A groyne: a bar near the top with its ends sloping down-outward, two heavy posts under it (EK-1ç s.10). */
const slope = { x0: 1.96, y0: 2.8, x1: 3.35, y1: -0.35 };
const slopeLen = Math.hypot(slope.x1 - slope.x0, slope.y1 - slope.y0);
const slopeDeg = (Math.atan2(slope.y1 - slope.y0, slope.x1 - slope.x0) * 180) / Math.PI;
const kky = stack([
  ...framed('', { size: S, strokeWidth: 0.25, background: WHITE }),
  captionMark('KKY'),
  shape('line', 2 * slope.x0, { stroke: BLACK, strokeWidth: 0.4, offset: [0, slope.y0] }),
  shape('line', slopeLen, { stroke: BLACK, strokeWidth: 0.4, rotation: slopeDeg, offset: turned((slope.x0 + slope.x1) / 2, (slope.y0 + slope.y1) / 2, slopeDeg) }),
  shape('line', slopeLen, { stroke: BLACK, strokeWidth: 0.4, rotation: 180 - slopeDeg, offset: turned(-(slope.x0 + slope.x1) / 2, (slope.y0 + slope.y1) / 2, 180 - slopeDeg) }),
  shape('line', 5.6, { stroke: BLACK, strokeWidth: 0.55, rotation: 90, offset: turned(-1.3, 0, 90) }),
  shape('line', 5.6, { stroke: BLACK, strokeWidth: 0.55, rotation: 90, offset: turned(1.3, 0, 90) }),
]);
denizyollari.area('kiyi-koruma-yapilari', 'Kıyı koruma yapıları', [solid(PORT), strips(2, 5), kky], {
  ref: 'EK-1ç s.10 (AÇIKLAMA 18); EK-1e s.107',
  note: `${stripNote(2)} Sembol şekillerle kuruldu (piktogram listesinde yok). Dalgakıran, köprü, menfez, istinat duvarı, mendirek ve mahmuz gibi yapıları kapsar.`,
});
denizyollari.area(
  'deniz-inis-rampasi',
  'Deniz iniş rampası',
  [solid(PORT), strips(2, 5), stack([...framed('', { frame: 'double', size: S, strokeWidth: 0.3, background: WHITE }), shape('arrowhead', 0.8 * S, { height: S, stroke: BLACK, strokeWidth: 0.25, rotation: 90 })])],
  { ref: 'EK-1ç s.10; EK-1e s.108', note: `${stripNote(2)} Sembol iç içe iki kare, içte tabanı iç karenin alt kenarında boş üçgen.` },
);

// ── Kentsel toplu taşıma güzergâhları ──────────────────────────────────

export const topluTasima = sheet('nip', ['Teknik altyapı', 'Kentsel toplu taşıma güzergâhları'], 'toplu-tasima', 94);

topluTasima.line(
  'rayli-toplu-tasima-hatti',
  'Raylı toplu taşıma hattı',
  [stroke(BLACK, 0.3, { offset: 2 }), stroke(BLACK, 0.3, { offset: -2 }), along(shape('rectangle', 5, { height: 2, fill: BLACK }), 7)],
  { ref: 'EK-1ç s.10; EK-1e s.169', note: '0,3 mm; 2×5 mm dolu dikdörtgenler 2 mm arayla, dikdörtgenlerin uzun kenarına 1 mm uzaklıkta iki paralel düz çizgi (eksenden ±2 mm).' },
);
topluTasima.area('rayli-toplu-tasima-istasyonu', 'Raylı toplu taşıma istasyonu', [solid(GREY), picture('tren-on', 'RTİ')], { ref: 'EK-1ç s.10; EK-1e s.170', note: 'Tarama yok.' });

/** A black square with a white X in a thin frame (EK-1ç s.10). */
const aktarmaMarks = [shape('square', S, { fill: WHITE, stroke: BLACK, strokeWidth: 0.25 }), shape('square', 5, { fill: BLACK }), shape('x', 5 * Math.SQRT2, { stroke: WHITE, strokeWidth: 0.35 })];
topluTasima.area('toplutasim-turleri-arasi-degisim-ve-aktarma-alani', 'Toplutaşım türleri arası değişim ve aktarma alanı', [solid(GREY), stack(aktarmaMarks)], {
  ref: 'EK-1ç s.10; EK-1e s.172',
  note: 'Tarama yok. EK-1e NİP geometrisi "çoklu poligon/nokta": noktası için ayrı işaret var.',
});
topluTasima.point('toplutasim-turleri-arasi-degisim-ve-aktarma-noktasi', 'Toplutaşım türleri arası değişim ve aktarma alanı (nokta)', [aktarmaMarks], {
  ref: 'EK-1ç s.10; EK-1e s.172',
  note: 'Alanı çizilemeyecek kadar küçük aktarma noktaları için (EK-1e NİP geometrisi "çoklu poligon/nokta").',
});

/** A cable line: 2 mm hangers every 10 mm on one side, each holding a 2×5 mm cabin by the middle of its long side. */
const cableNote = '0,3 mm düz çizgi; 10 mm aralıklı, çizgiye dik 2 mm askılar; askılara uzun kenarının ortasından bağlı 2×5 mm dikdörtgenler. Askılar çizim yönünün sağında (EK-1ç örneğinde çizginin altında).';
topluTasima.line('havai-hat', 'Havai hat', [stroke(BLACK, 0.3), ticks(BLACK, 0.3, 2, 10, -1, { offsetAlong: 5 }), along(shape('rectangle', 5, { height: 2, fill: BLACK }), 10, { offsetAlong: 5, offset: -3 })], {
  ref: 'EK-1ç s.10; EK-1e s.170',
  note: `${cableNote} Dikdörtgenler dolu.`,
});
topluTasima.area('havai-hat-istasyonu', 'Havai hat istasyonu', [solid(GREY), strips(2, 5), picture('teleferik')], { ref: 'EK-1ç s.10; EK-1e s.171', note: stripNote(2) });
topluTasima.line(
  'havaray',
  'Havaray',
  [stroke(BLACK, 0.3), ticks(BLACK, 0.3, 2, 10, -1, { offsetAlong: 5 }), along(shape('rectangle', 5, { height: 2, stroke: BLACK, strokeWidth: 0.4 }), 10, { offsetAlong: 5, offset: -3 })],
  { ref: 'EK-1ç s.11; EK-1e s.171', note: `${cableNote} Dikdörtgenler içi boş (EK-1ç'de kalın çizgili). EK-1ç bu satıra 178/178/178 alan rengi yazmış; çizgi sembolünde dolgu yok.` },
);
topluTasima.area('havaray-istasyonu', 'Havaray istasyonu', [solid(GREY), strips(2, 5), picture('havaray', 'HRİ')], { ref: 'EK-1ç s.11; EK-1e s.172', note: stripNote(2) });

// ── Enerji üretim, dağıtım ve depolama ─────────────────────────────────

export const enerji = sheet('nip', ['Teknik altyapı', 'Enerji üretim, dağıtım ve depolama'], 'enerji', 95);

const banded = () => [hatch(0, 6, 0.2), hatch(45, 2, 0.2)];
const bandedNote = '0,2 mm, 6 mm aralıklı yatay çizgiler ve 2 mm aralıklı 45 derecelik çizgiler.';

enerji.area('regulator-alani', 'Regülatör alanı', [solid(ENERGY), banded(), picture('simsek', 'R')], { ref: 'EK-1ç s.11; EK-1e s.176', note: bandedNote });
enerji.area('turbin-alani', 'Türbin alanı', [solid(ENERGY), banded(), picture('simsek', 'T')], { ref: 'EK-1ç s.11; EK-1e s.177', note: bandedNote });
enerji.area('dogalgaz-iletim-dagitim-tesisi-alani', 'Doğalgaz iletim/dağıtım tesisi alanı', [solid(ENERGY), banded(), picture('simsek', 'DT')], { ref: 'EK-1ç s.11; EK-1e s.177', note: bandedNote });
enerji.area(
  'yanici-parlayici-ve-patlayici-maddeler-uretim-ve-depo-alani',
  'Yanıcı parlayıcı ve patlayıcı maddeler üretim ve depo alanı',
  [solid(ENERGY), groupedHatch(45, 12, 2, 1, 0.2), hatch(135, 12, 0.2), picture('dinamit')],
  { ref: 'EK-1ç s.11; EK-1e s.178', note: '0,2 mm; 45 derecede 1 mm aralıklı tarama çiftleri, 135 derecede tek tarama, ara mesafe 12 mm. EK-1ç örneği daha sık (≈4 mm) basılmış; ölçü metinden.' },
);
enerji.area(
  'enerji-depolama-alani',
  'Enerji depolama alanı',
  [solid(ENERGY), banded(), stack([...pictureMarks('simsek'), captionMark('EDA', CAPTION, S, 'sans')])],
  { ref: 'EK-1ç s.11; EK-1e s.178', note: `${bandedNote} Alt yazı EK-1ç'deki gibi Arial (öteki alt yazılar Times).` },
);
enerji.area('yenilenebilir-enerji-kaynaklarina-dayali-uretim-tesisi-alani', 'Yenilenebilir enerji kaynaklarına dayalı üretim tesisi alanı', [solid(rgb(171, 171, 200)), banded(), picture('simsek', 'YED')], {
  ref: 'EK-1ç s.11; EK-1e s.174',
  note: bandedNote,
});
enerji.area('rafineri-petrokimya-tesisi-alani', 'Rafineri-petrokimya tesisi alanı', [solid(ENERGY), banded(), box('R', { weight: 900, heavy: true, max: 4.4 })], { ref: 'EK-1ç s.11; EK-1e s.179', note: bandedNote });

// ── Su-atıksu ve arıtma sistemleri ─────────────────────────────────────

export const su = sheet('nip', ['Teknik altyapı', 'Su-atıksu ve arıtma sistemleri'], 'su', 96);

su.line(
  'sogutma-suyu-alma-hatti',
  'Soğutma suyu alma hattı',
  [stroke(BLACK, 0.4), along(shape('triangle', 3, { fill: BLACK, rotation: 90 }), 15, { offsetAlong: 0 }), along(text('S', 5, { font: 'sans', weight: 400 }), 15, { offsetAlong: 7.5 })],
  {
    ref: 'EK-1ç s.11; EK-1e s.183',
    note: '0,4 mm düz çizgi üzerinde 15 mm aralıklı, 3 mm kenarlı dolu eşkenar üçgenler; iki üçgenin ortasında 5 mm "S" harfi (çizgi harfin içinden geçer). Üçgenler çizgi başına doğru bakar (suyun alındığı yöne; EK-1ç örneğindeki gibi).',
  },
);

const CANAL = 'varsayılan([Genişlik], 30)';
/** Ticks 3 mm long every 15 mm on the bank `side`, pointing into the channel. */
const canalTicks = (side: 1 | -1, offsetAlong: number) => ({
  ...ticks(BLACK, 0.3, 3, 15, side === 1 ? -1 : 1, { offsetAlong }),
  offset: { expr: `${side < 0 ? '-' : ''}${mm(`${CANAL} / 2`)}`, fallback: side * at5000(15) },
});
su.line(
  'sulama-hatti',
  'Sulama hattı',
  [
    { ...stroke(rgb(115, 223, 235), at5000(30), { cap: 'butt' }), width: { expr: mm(CANAL), fallback: at5000(30) } },
    { ...sideLine(0.3, `${CANAL} / 2`, 15, 1) },
    { ...sideLine(0.3, `${CANAL} / 2`, 15, -1) },
    canalTicks(1, 3.75),
    canalTicks(-1, 11.25),
    along(text({ expr: `varsayılan([Durum], '')` }, 2.2, { font: 'sans', weight: 700 }), 0, { placement: 'center' }),
  ],
  {
    ref: 'EK-1ç s.12; EK-1e s.99',
    note: 'Çizgi kanalın eksenidir: kanal genişliğindeki ("Genişlik", m; boşsa 30) iki 0,3 mm hat çizgisi, araları 115/223/235; hat çizgilerine 15 mm aralıklarla dik, 3 mm yüksekliğinde, karşılıklı şaşırtmalı (7,5 mm kaydırılmış) içe dönük çizgiler. Açık, kapalı, yeraltı, hemzemin ya da havai hat olduğu hat üstünde belirtilir: "Durum" alanı doluysa eksene yazılır. Genişlik çizim ölçeğine göre kâğıda çevrilir; varsayılan 30 m EK-1ç çiziminden (≈6 mm).',
  },
);
