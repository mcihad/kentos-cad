import { BLACK, WHITE, along, circle, double, hatch, label, pattern, rgb, shape, sheet, solid, stroke, text } from '../dsl';
import { FRAME, FRAME_LINE, captionMark, picto, seg } from './common';

/**
 * UİP (EK-1d s.16–18) › Teknik altyapı › Ulaşım: demiryolları,
 * denizyolları, havayolları, kentsel toplu taşıma güzergâhları (EK-1e
 * s.158–172, 107–108).
 */

export const demiryollari = sheet('uip', ['Teknik altyapı', 'Ulaşım', 'Demiryolları'], 'demiryollari', 20);
export const denizyollari = sheet('uip', ['Teknik altyapı', 'Ulaşım', 'Denizyolları'], 'denizyollari', 30);
export const havayollari = sheet('uip', ['Teknik altyapı', 'Ulaşım', 'Havayolları'], 'havayollari', 40);
export const topluTasima = sheet('uip', ['Teknik altyapı', 'Ulaşım', 'Kentsel toplu taşıma güzergâhları'], 'toplu-tasima', 50);

/**
 * "Dik tarama çiftleri, çiftler arası `w` mm, tarama arası `gap` mm, çiftler
 * içinde `angle` derecelik 1 mm aralıklı çizgiler": vertical strips `w`
 * wide (two lines), `gap` apart, filled with slanted lines `spacing` apart
 * that run from one line of the strip to the other. The slants are line
 * markers on a grid (one per strip and step), the grid shifted onto the
 * strips. Angles count counter-clockwise from east, as the regulation's
 * other hatch angles (60°, 315°) do.
 */
function strips(w: number, gap: number, angle: number, width = 0.2, spacing = 1) {
  const period = w + gap;
  const c = Math.abs(Math.cos((angle * Math.PI) / 180));
  const r3 = (v: number) => Math.round(v * 1000) / 1000;
  return [
    hatch(90, period, width),
    // A vertical hatch's offset moves its lines to -offset.
    hatch(90, period, width, BLACK, { offset: -w }),
    pattern(shape('line', r3(w / c), { stroke: BLACK, strokeWidth: width, rotation: angle }), period, r3(spacing / c), { offset: [r3(w / 2 - period / 2), 0] }),
  ];
}
const STRIP_NOTE = (w: number, gap: number, angle: number) =>
  `Tarama: 0.2 mm dik çizgi çiftleri (çift ${w} mm geniş, çiftler arası ${gap} mm), çiftlerin içinde ${angle}° 1 mm aralıklı çizgiler. Açı yataydan (60°, 315° taramalar gibi) okundu: ${angle === 330 ? 'sağa doğru 30° inen' : 'sağa doğru 30° yükselen'}; çizimler dike yakın dik (≈70°) eğik çizgiler gösteriyor, metin esas alındı.`;

// ── Demiryolları ───────────────────────────────────────────────────────

demiryollari.line(
  'katar-duzenleme-triyaj-alani',
  'Katar düzenleme (triyaj) alanı',
  [stroke(BLACK, 0.8), along(shape('rectangle', 5, { height: 2, fill: BLACK }), 7, { offsetAlong: 2.5 })],
  { ref: 'EK-1d s.16; EK-1e s.158', note: 'Demiryolu çizgi tipi: 0.8 mm çizgi, 2×5 mm dolu dikdörtgenler, aralarında 2 mm düz çizgi. Lejant ana hattan dallanan birkaç hat çizer: her hat ayrı çizgidir.' },
);
demiryollari.area('ara-istasyon', 'Ara istasyon', [solid(rgb(178, 178, 178)), strips(2, 5, 330), picto('ara-istasyon', { size: 9, caption: 'İST' })], {
  ref: 'EK-1d s.16; EK-1e s.159',
  note: STRIP_NOTE(2, 5, 330),
});

// ── Denizyolları ───────────────────────────────────────────────────────

const LIMAN = rgb(130, 130, 130);
const DENIZ = rgb(204, 204, 204);

/** Menfez, mahmuz, kıyı koruma yapıları: a bar with two heavy posts and two legs splayed to the frame's corners (EK-1d). */
const pier = () => [
  shape('square', FRAME, { fill: WHITE, stroke: BLACK, strokeWidth: FRAME_LINE }),
  seg(-3.1, 3.9, 3.1, 3.9, 0.45),
  seg(-3.1, 3.9, -4.8, -2.7, 0.45),
  seg(3.1, 3.9, 4.8, -2.7, 0.45),
  seg(-1, 3.9, -1, -4.6, 0.7),
  seg(1, 3.9, 1, -4.6, 0.7),
];

denizyollari.area('konteyner-limani', 'Konteyner limanı', [solid(LIMAN), strips(5, 5, 330), picto('capraz-capalar', { size: 9, caption: 'KO' })], { ref: 'EK-1d s.16; EK-1e s.161', note: STRIP_NOTE(5, 5, 330) });
denizyollari.area('kruvaziyer-limani', 'Kruvaziyer limanı', [solid(LIMAN), strips(5, 5, 330), picto('capraz-capalar', { size: 9, caption: 'KR' })], { ref: 'EK-1d s.16; EK-1e s.162', note: STRIP_NOTE(5, 5, 330) });
denizyollari.area('ro-ro-limani', 'Ro ro limanı', [solid(LIMAN), strips(5, 5, 330), picto('capraz-capalar', { size: 9, caption: 'RR' })], { ref: 'EK-1d s.16; EK-1e s.162', note: STRIP_NOTE(5, 5, 330) });
denizyollari.area('yat-limani', 'Yat limanı', [solid(LIMAN), strips(2, 5, 330), picto('capraz-capalar', { size: 9, caption: 'YAT' })], { ref: 'EK-1d s.16; EK-1e s.163', note: STRIP_NOTE(2, 5, 330) });
denizyollari.area('balikci-barinagi', 'Balıkçı barınağı', [solid(DENIZ), strips(2, 5, 330), picto('yelkenli-bos', { size: 9, caption: 'BB' })], { ref: 'EK-1d s.16; EK-1e s.163', note: STRIP_NOTE(2, 5, 330) });
denizyollari.area('iskele', 'İskele', [solid(DENIZ), strips(2, 5, 330), picto('capa-halat', { size: 9, caption: 'İ' })], { ref: 'EK-1d s.16; EK-1e s.164', note: STRIP_NOTE(2, 5, 330) });
denizyollari.area('tekne-imal-ve-cekek-yeri', 'Tekne imal ve çekek yeri', [solid(rgb(178, 178, 178)), strips(2, 5, 330), picto('yelkenli-dolu', { size: 9, caption: 'ÇY' })], {
  ref: 'EK-1d s.16; EK-1e s.164',
  note: `${STRIP_NOTE(2, 5, 330)} Renk 178/178/178 (komşu deniz kullanımları 204/204/204).`,
});
denizyollari.area('gemi-sokum-yeri', 'Gemi söküm yeri', [solid(DENIZ), strips(2, 5, 330), picto('gemi-sokum', { size: 9, caption: 'GS' })], { ref: 'EK-1d s.16; EK-1e s.165', note: STRIP_NOTE(2, 5, 330) });
denizyollari.area('menfez', 'Menfez', [solid(DENIZ), strips(2, 5, 330), label([...pier(), captionMark('ME')])], {
  ref: 'EK-1d s.16; EK-1e s.166',
  note: `${STRIP_NOTE(2, 5, 330)} Sembol şekillerden kuruldu (piktogram setinde yok). AÇIKLAMA 14: kıyı koruma yapılarındandır.`,
});
denizyollari.area('mahmuz', 'Mahmuz', [solid(DENIZ), strips(2, 5, 330), label([...pier(), captionMark('MA')])], {
  ref: 'EK-1d s.17; EK-1e s.166',
  note: `${STRIP_NOTE(2, 5, 330)} Sembol şekillerden kuruldu (piktogram setinde yok). AÇIKLAMA 14: kıyı koruma yapılarındandır.`,
});
denizyollari.area('rihtim', 'Rıhtım', [solid(DENIZ), strips(2, 5, 330), picto('capa', { size: 9, caption: 'R' })], { ref: 'EK-1d s.17; EK-1e s.167', note: STRIP_NOTE(2, 5, 330) });
denizyollari.area('barinak', 'Barınak', [solid(DENIZ), strips(2, 5, 30), picto('capa', { size: 9, caption: 'B' })], {
  ref: 'EK-1d s.17; EK-1e s.167',
  note: `${STRIP_NOTE(2, 5, 30)} Metin öteki deniz kullanımlarından farklı olarak 30° der.`,
});
denizyollari.area('dolfen-platform', 'Dolfen/platform', [solid(DENIZ), strips(2, 5, 330), picto('dolfen', { size: 9, caption: 'P' })], { ref: 'EK-1d s.17; EK-1e s.168', note: STRIP_NOTE(2, 5, 330) });
denizyollari.area('liman', 'Liman', [solid(DENIZ), strips(5, 5, 330), picto('capraz-capalar', { frame: 'rect', frameSize: 11, height: 13.5, size: 10, caption: 'L' })], {
  ref: 'EK-1d s.17; EK-1e s.161',
  note: `${STRIP_NOTE(5, 5, 330)} Çerçeve EK-1d'deki gibi 11×13.5 mm dikey dikdörtgen.`,
});
denizyollari.area('tekne-imal-ve-bakim-yeri', 'Tekne imal ve bakım yeri', [solid(DENIZ), strips(5, 5, 330), picto('yelkenli-dolu', { size: 9, caption: 'TİB' })], { ref: 'EK-1d s.17; EK-1e s.165', note: STRIP_NOTE(5, 5, 330) });
denizyollari.area('kiyi-koruma-yapilari', 'Kıyı koruma yapıları', [solid(DENIZ), strips(2, 5, 330), label([...pier(), captionMark('KKY')])], {
  ref: 'EK-1d s.17; EK-1e s.107',
  note: `${STRIP_NOTE(2, 5, 330)} Sembol şekillerden kuruldu (piktogram setinde yok). AÇIKLAMA 14: dalgakıran, köprü, menfez, istinat duvarı, mendirek ve mahmuz gibi yapılar.`,
});
denizyollari.area(
  'deniz-inis-rampasi',
  'Deniz iniş rampası',
  [
    solid(DENIZ),
    strips(2, 5, 330),
    label([
      shape('square', FRAME, { fill: WHITE, stroke: BLACK, strokeWidth: 0.5 }),
      shape('square', FRAME - 3, { stroke: BLACK, strokeWidth: 0.25 }),
      seg(0, 3.5, -3.5, -3.5, 0.25),
      seg(0, 3.5, 3.5, -3.5, 0.25),
    ]),
  ],
  { ref: 'EK-1d s.17; EK-1e s.108', note: `${STRIP_NOTE(2, 5, 330)} Sembol: çift kare içinde tepesi üstte ikizkenar üçgen (şekillerden kuruldu).` },
);

// ── Havayolları ────────────────────────────────────────────────────────

havayollari.area(
  'helikopter-inis-alani',
  'Helikopter iniş alanı',
  [solid(rgb(178, 178, 178)), strips(2, 5, 330, 0.3), label([circle(9.5, { fill: WHITE, stroke: BLACK, strokeWidth: 0.35 }), text('H', 5.3, { font: 'sans', weight: 400 })])],
  { ref: 'EK-1d s.17; EK-1e s.169', note: `${STRIP_NOTE(2, 5, 330).replace('0.2 mm', '0.3 mm')} Eğik çizgilerin aralığı verilmemiş (1 mm alındı). Sembol: çerçevesiz dairede "H".` },
);

// ── Kentsel toplu taşıma güzergâhları ──────────────────────────────────

topluTasima.line('rayli-toplu-tasima-hatti', 'Raylı toplu taşıma hattı', [double(BLACK, 0.3, 4), along(shape('rectangle', 5, { height: 2, fill: BLACK }), 7, { offsetAlong: 2.5 })], {
  ref: 'EK-1d s.17; EK-1e s.169',
  note: '0.3 mm: 2×5 mm dolu dikdörtgenler 2 mm aralı, uzun kenarlarına 1 mm uzaklıkta paralel düz çizgiler (eksenden ±2 mm).',
});
topluTasima.area('rayli-toplu-tasima-istasyonu', 'Raylı toplu taşıma istasyonu', [solid(rgb(178, 178, 178)), picto('metro-on', { size: 9, caption: 'RTİ' })], { ref: 'EK-1d s.18; EK-1e s.170', note: 'Düz dolgu 178/178/178, tarama yok.' });

/** Hangers: 2 mm lines on the right every 10 mm, a 2×5 mm cabin hung from each at the middle of its long side. */
const cabins = (filled: boolean) => [
  stroke(BLACK, 0.3),
  along([seg(0, 0, 0, -2, 0.3), shape('rectangle', 5, { height: 2, fill: filled ? BLACK : null, stroke: filled ? null : BLACK, strokeWidth: 0.3, offset: [0, -3] })], 10, { offsetAlong: 5 }),
];
topluTasima.line('havai-hat', 'Havai hat', [cabins(true)], {
  ref: 'EK-1d s.18; EK-1e s.170',
  note: '0.3 mm düz çizgi, 10 mm aralıklı 2 mm dik çizgiler ve bunlara uzun kenarlarının ortasından bağlı 2×5 mm dolu dikdörtgenler; çizim yönünün sağında (EK-1d\'de altta).',
});
topluTasima.area('havai-hat-istasyonu', 'Havai hat istasyonu', [solid(rgb(178, 178, 178)), strips(2, 5, 330), picto('teleferik', { size: 9 })], { ref: 'EK-1d s.18; EK-1e s.171', note: STRIP_NOTE(2, 5, 330) });
topluTasima.line('havaray', 'Havaray', [cabins(false)], {
  ref: 'EK-1d s.18; EK-1e s.171',
  note: '0.3 mm düz çizgi, 10 mm aralıklı 2 mm dik çizgiler ve bunlara bağlı 2×5 mm içi boş dikdörtgenler; çizim yönünün sağında. EK-1d dikdörtgenleri kalın çizer; metin 0.3 mm der.',
});
topluTasima.area('havaray-istasyonu', 'Havaray istasyonu', [solid(rgb(178, 178, 178)), strips(2, 5, 330), picto('havaray', { size: 9, caption: 'HRİ' })], { ref: 'EK-1d s.18; EK-1e s.172', note: STRIP_NOTE(2, 5, 330) });
topluTasima.area(
  'toplutasim-turleri-arasi-degisim-ve-aktarma-alani',
  'Toplutaşım türleri arası değişim ve aktarma alanı',
  [
    solid(rgb(178, 178, 178)),
    label([shape('square', FRAME, { fill: WHITE, stroke: BLACK, strokeWidth: FRAME_LINE }), shape('square', 6.4, { fill: BLACK }), shape('x', 6.4 * Math.SQRT2, { stroke: WHITE, strokeWidth: 0.35 })]),
  ],
  { ref: 'EK-1d s.18; EK-1e s.172', note: 'Düz dolgu 178/178/178, tarama yok. Sembol: çerçevede dolu siyah kare ve beyaz köşegenleri (şekillerden kuruldu).' },
);
