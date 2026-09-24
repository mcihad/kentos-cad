import { BLACK, along, edge, framed, rgb, shape, sheet, solid, stroke } from '../dsl';
import { S, stack } from './common';

/**
 * ÇDP (EK-1c) › Ulaşım: karayolları sabit genişlikte şerit olarak (siyah /
 * sarı / siyah üst üste çizgiler), denizyolları, demiryolları (EK-1e
 * s.107, 147-150, 169). Sarı, EK-1c'de yazılı "Sarı çizgi = 255/255/115".
 */

const YELLOW = rgb(255, 255, 115);
const yellowNote = 'Sarı EK-1c\'de yazılı 255/255/115; EK-1e bu yolun kaldırımını 255/255/0 yazıyor (EK-1c çiziminde otoyolun sarısı parlak, birinci ve ikinci derece yolunki soluk).';

export const ulasim = sheet('cdp', ['Ulaşım'], 'ulasim', 80);
export const karayollari = sheet('cdp', ['Ulaşım', 'Karayolları'], 'karayollari', 81);

// Otoyol: 3 mm in all; per carriageway a 1 mm black platform, a thin yellow kaldırım outside it and a 0.3 mm black cephe line; a white refüj between the platforms.
const MEDIAN = 0.16;
const KERB = 0.12;
const otoyol = [1, -1].flatMap((s) => [
  stroke(BLACK, 1, { offset: s * (MEDIAN / 2 + 0.5) }),
  stroke(YELLOW, KERB, { offset: s * (MEDIAN / 2 + 1 + KERB / 2) }),
  stroke(BLACK, 0.3, { offset: s * 1.35 }),
]);
karayollari.line('erisme-kontrollu-karayolu-otoyol', 'Erişme kontrollü karayolu (otoyol)', [otoyol], {
  ref: 'EK-1c s.4; EK-1e s.147',
  note: `Cephe çizgisi 0,3 mm, yol toplamı 3 mm, yol platformları 1 mm. Kalan 0,4 mm, EK-1c çizimindeki oranla sarı kaldırımlara (0,12 mm) ve beyaz refüje (0,16 mm) bölündü. ${yellowNote}`,
});

/** A cased ribbon with round ends: black `total`, yellow `platform`, black core `core` (centred strokes). */
const ribbon = (total: number, platform: number, core: number) => [stroke(BLACK, total, { cap: 'round' }), stroke(YELLOW, platform, { cap: 'round' }), stroke(BLACK, core, { cap: 'round' })];

karayollari.line('birinci-derece-yol', 'Birinci derece yol', [ribbon(1.8, 1.2, 0.6)], {
  ref: 'EK-1c s.4; EK-1e s.148',
  note: `Cephe çizgisi 0,3 mm, yol toplamı 1,8 mm, yol platformu 1,2 mm; uçlar yuvarlak. Ortadaki siyah çekirdeğin genişliği yazılı değil, EK-1c çiziminden 0,6 mm (toplamın üçte biri). ${yellowNote}`,
});
karayollari.line('ikinci-derece-yol', 'İkinci derece yol', [ribbon(1.2, 0.8, 0.4)], {
  ref: 'EK-1c s.4; EK-1e s.149',
  note: `Yol toplamı 1,2 mm, yol platformu 0,8 mm; uçlar yuvarlak. 1,2 − 0,8 her yanda 0,2 mm siyah bırakır, EK-1e'nin "cephe çizgisi 0,3 mm"si ile çelişiyor: toplam ve platform esas alındı. Çekirdek 0,4 mm (çizimden). ${yellowNote}`,
});
karayollari.line('ucuncu-derece-yol', 'Üçüncü derece yol', [stroke(BLACK, 1), stroke(YELLOW, 0.4)], {
  ref: 'EK-1c s.4; EK-1e s.150',
  note: '1 mm siyah çizgi üzerine 0,4 mm sarı çizgi (255/255/115). EK-1c çizimi sarıyı daha geniş basmış; ölçü metinden.',
});

export const denizyollari = sheet('cdp', ['Ulaşım', 'Denizyolları'], 'denizyollari', 82);

denizyollari.area(
  'kiyi-tesisleri-alani',
  'Kıyı tesisleri alanı',
  [solid(rgb(225, 225, 225)), edge(BLACK, 0.2), stack(framed('KTA', { size: S, strokeWidth: 0.25, background: rgb(150, 150, 150), textSize: 1.9, font: 'sans', weight: 700 }))],
  { ref: 'EK-1c s.4 (AÇIKLAMA 8); EK-1e s.107', note: 'Sınır 0,2 mm siyah. Sembol gri (150/150/150, EK-1e\'de yazılı) karede "KTA".' },
);

export const demiryollari = sheet('cdp', ['Ulaşım', 'Demiryolları'], 'demiryollari', 83);

demiryollari.line(
  'rayli-toplu-tasima-hatti',
  'Raylı toplu taşıma hattı',
  [stroke(BLACK, 0.4, { offset: 2.5 }), stroke(BLACK, 0.4, { offset: -2.5 }), along(shape('rectangle', 5, { height: 3, fill: BLACK }), 8)],
  { ref: 'EK-1c s.4; EK-1e s.169', note: '0,4 mm; 3×5 mm dolu dikdörtgenler 3 mm arayla (5 mm kenar çizgi boyunca), dikdörtgenlerin uzun kenarına 1 mm uzaklıkta iki paralel düz çizgi (eksenden ±2,5 mm).' },
);
