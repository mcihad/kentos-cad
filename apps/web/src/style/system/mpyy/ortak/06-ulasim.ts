import { BLACK, WHITE, along, grid, rgb, shape, solid, stroke } from '../dsl';
import { heading, picto, pictoMarks, sections, stripHatch, type Level } from './common';

/**
 * EK-1a (s.7–8) › Ulaşım: karayolları, demiryolları, denizyolları, diğer
 * su yolları, havayolları; the common items, every plan level (EK-1e
 * s.151–168).
 *
 * "Dik tarama çiftleri … çiftler içinde 30/330 derecelik 1 mm aralıklı
 * çizgiler": the angle is read as the brief's convention (counter-
 * clockwise from east): 330° falls to the right 30° below the horizontal,
 * 30° rises to the right. EK-1e and EK-1a draw every one of them falling
 * to the right about 25° from the vertical; the text is followed.
 */

export const HEAD = heading(['Ulaşım'], 1095);

/** Terminal: square grid spacing per level (EK-1e s.151). */
const TERMINAL_GRID: Record<Level, number> = { uip: 3, nip: 2, cdp: 2 };

export const KARAYOLLARI = sections(['Ulaşım', 'Karayolları'], 'karayollari', 1100, (s, lv) => {
  const g = TERMINAL_GRID[lv];
  s.area('terminal-otogar', 'Terminal (otogar)', [solid(rgb(255, 56, 0)), grid(g, g, 0.2), picto(lv, 'terminal')], {
    ref: 'EK-1a s.7; EK-1e s.151',
    note: `Alan 255/56/0; 0.2 mm, ${g} mm ara ile karelaj. Sembol: çerçevede üç kollu daire.`,
  });
});

export const DEMIRYOLLARI = sections(['Ulaşım', 'Demiryolları'], 'demiryollari', 1110, (s, lv) => {
  s.line('demiryolu', 'Demiryolu', [stroke(BLACK, 0.8), along(shape('rectangle', 5, { height: 2, fill: BLACK }), 7, { offsetAlong: 2.5 })], {
    ref: 'EK-1a s.7; EK-1e s.157',
    note: '0.8 mm düz çizgi üzerinde 2×5 mm içi dolu dikdörtgenler, aralarında 2 mm düz çizgi.',
  });
  s.line(
    'hizli-tren-hatti',
    'Hızlı tren hattı',
    [
      stroke(BLACK, 0.3),
      stroke(BLACK, 0.3, { offset: 2 }),
      stroke(BLACK, 0.3, { offset: 3 }),
      stroke(BLACK, 0.3, { offset: -2 }),
      stroke(BLACK, 0.3, { offset: -3 }),
      along(shape('rectangle', 5, { height: 2, fill: BLACK }), 14, { offsetAlong: 2.5 }),
      along(shape('rectangle', 5, { height: 2, fill: WHITE, stroke: BLACK, strokeWidth: 0.3 }), 14, { offsetAlong: 9.5 }),
    ],
    {
      ref: 'EK-1a s.7; EK-1e s.157',
      note: '0.3 mm; 2×5 mm bir dolu bir boş dikdörtgen, dikdörtgenlerin uzun kenarına paralel 1 mm uzaklıkta 1 mm aralıklı çift düz çizgi (eksenden ±2 ve ±3 mm). Dikdörtgenler arası verilmemiş: 2 mm çizimden.',
    },
  );
  const gar = 'Sembol: çerçevede lokomotif, istasyon binası ve yolcu, altında GAR.';
  if (lv === 'cdp')
    s.point('ana-istasyon-gar', 'Ana istasyon (gar)', [pictoMarks(lv, 'gar', { caption: 'GAR' })], {
      ref: 'EK-1a s.7; EK-1e s.158',
      note: `ÇDP'de nokta (EK-1e geometri "nokta", tarama açıklaması boş). ${gar}`,
    });
  else
    s.area('ana-istasyon-gar', 'Ana istasyon (gar)', [solid(rgb(178, 178, 178)), stripHatch(5, 5, 30, 0.2), picto(lv, 'gar', { caption: 'GAR' })], {
      ref: 'EK-1a s.7; EK-1e s.158',
      note: `Alan 178/178/178; 0.2 mm dik tarama çiftleri (şeritler 5 mm geniş, aralarında 5 mm), şeritlerin içinde 30° açılı 1 mm aralıklı çizgiler (açı metindeki gibi sağa yükselir; çizim sağa düşen dik çizgiler gösterir). ${gar}`,
    });
});

export const DENIZYOLLARI = sections(['Ulaşım', 'Denizyolları'], 'denizyollari', 1120, (s, lv) => {
  s.line('deniz-ulasim-baglantisi', 'Deniz ulaşım bağlantısı', [stroke(BLACK, 0.8, { dash: [6, 2] })], {
    ref: 'EK-1a s.7; EK-1e s.159',
    note: '0.8 mm; 6 mm düz çizgi, 2 mm boşluk.',
  });
  s.area('tersane-alani', 'Tersane alanı', [solid(rgb(130, 130, 130)), stripHatch(5, 5, 330, 0.2), picto(lv, 'gemi', { caption: 'T' })], {
    ref: 'EK-1a s.8; EK-1e s.160',
    note: 'Alan 130/130/130; 0.2 mm dik tarama çiftleri (şeritler 5 mm geniş, aralarında 5 mm), şeritlerin içinde 330° açılı 1 mm aralıklı çizgiler (sağa 30° düşer; çizim daha dik çizer). Sembol: çerçevede gemi, altında T.',
  });
});

export const DIGER_SU_YOLLARI = sections(['Ulaşım', 'Diğer su yolları'], 'diger-su-yollari', 1130, (s) => {
  s.line('su-ulasim-baglantisi', 'Su ulaşım bağlantısı', [stroke(rgb(0, 77, 168), 0.8, { dash: [5, 0.5, 1, 0.5], cap: 'butt' })], {
    ref: 'EK-1a s.8; EK-1e s.160',
    note: 'Mavi 0/77/168, 0.8 mm; 5 mm uzunluklu noktalı kesik çizgi. Metin hem 0.8 mm hem "1 mm kalınlığında" der: çizgi 0.8 mm, nokta 1 mm uzunluğunda alındı; aralar 0.5 mm çizimden.',
  });
});

export const HAVAYOLLARI = sections(['Ulaşım', 'Havayolları'], 'havayollari', 1140, (s, lv) => {
  s.area('havaalani-hava-limani', 'Havaalanı / hava limanı', [solid(rgb(178, 178, 178)), stripHatch(5, 10, 330, 0.3), picto(lv, 'ucak')], {
    ref: 'EK-1a s.8; EK-1e s.168',
    note: 'Alan 178/178/178; 0.3 mm dik tarama çiftleri (şeritler 5 mm geniş, aralarında 10 mm), şeritlerin içinde 330° açılı 1 mm aralıklı çizgiler. Sembol: çerçevede uçak.',
  });
});
