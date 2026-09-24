import { BLACK, along, circle, edge, hatch, rgb, shape, solid, stroke, text } from '../dsl';
import { SYMBOL, code, gridDots, picto, sections, seg, stack, type Level } from './common';

/**
 * EK-1a (s.8–9) › Enerji üretim-dağıtım ve depolama; su-atıksu ve atık
 * sistemleri (with su yüzeyi, yapay ada and vahşi çöp alanı, which follow
 * them in the table without a heading), every plan level (EK-1e s.173–191,
 * s.100).
 */

/** Enerji nakil hattı: interval of the zigzag per level (EK-1e s.182). */
const PYLON: Record<Level, number> = { uip: 10, nip: 20, cdp: 30 };

/**
 * The "30° açılı şekildeki obje": the line rises into a spike, drops
 * through the axis into an equal spike on the other side and returns,
 * 30° at both points. Its height (2 mm each side) is measured from EK-1e.
 */
function zigzag(interval: number) {
  const h = 2;
  const a = Math.round(h * Math.tan(Math.PI / 12) * 1000) / 1000;
  const len = 4 * a;
  return [
    stroke(BLACK, 0.4, { dash: [interval - len, len] }),
    along([seg(-2 * a, 0, -a, h, 0.4), seg(-a, h, a, -h, 0.4), seg(a, -h, 2 * a, 0, 0.4)], interval, { offsetAlong: interval - 2 * a }),
  ];
}

/** "karolaj merkezlerinde noktalama": UİP 1.2 mm dots 7 mm apart, NİP/ÇDP 0.4 mm dots 5 mm apart unless given otherwise. */
type Dots = Record<Level, readonly [number, number]>;
const DOTS: Dots = { uip: [7, 1.2], nip: [5, 0.4], cdp: [5, 0.4] };
/** Atıksu tesisleri: NİP as UİP (EK-1e s.185). */
const DOTS_ATIKSU: Dots = { uip: [7, 1.2], nip: [7, 1.2], cdp: [5, 0.4] };
/** Atık geri kazanım: 0.4 mm dots at every level (EK-1e s.187). */
const DOTS_GERI: Dots = { uip: [7, 0.4], nip: [5, 0.4], cdp: [5, 0.4] };

const GREY = rgb(178, 178, 178);
const dots = (t: Dots, lv: Level) => gridDots(t[lv][0], t[lv][1]);
const dotNote = (t: Dots, lv: Level) => `Alan 178/178/178; ${t[lv][0]}×${t[lv][0]} mm karolaj merkezlerinde ${t[lv][1]} mm nokta.`;

export const ENERJI = sections(['Enerji üretim-dağıtım ve depolama'], 'enerji', 1150, (s, lv) => {
  s.area('enerji-uretim-alani', 'Enerji üretim alanı', [solid(rgb(171, 171, 200)), hatch(0, 6, 0.2), hatch(45, 2, 0.2), picto(lv, 'simsek')], {
    ref: 'EK-1a s.8; EK-1e s.173',
    note: 'Alan 171/171/200; 0.2 mm, 6 mm aralıklı yatay çizgiler ve 2 mm aralıklı 45° çizgiler. Sembol: çerçevede şimşek.',
  });
  s.line('boru-hatti', 'Boru hattı', [stroke(BLACK, 0.4), along(shape('triangle', 3, { fill: BLACK, rotation: -90 }), 15)], {
    ref: 'EK-1a s.8; EK-1e s.180',
    note: '0.4 mm düz çizgi üzerinde 15 mm aralıklı 3 mm kenarlı eşkenar üçgenler (dolu, tepesi çizim yönünde).',
  });
  s.line('enerji-nakil-hatti', 'Enerji nakil hattı', [zigzag(PYLON[lv])], {
    ref: 'EK-1a s.8; EK-1e s.182',
    note: `0.4 mm; ${PYLON[lv]} mm aralıklı 30° açılı zikzak (çizginin bir yana yükselip öbür yana indiği kırık). Zikzağın yüksekliği verilmemiş: iki yana 2 mm çizimden.`,
  });
});

export const ATIK = sections(['Su-atıksu ve atık sistemleri'], 'atik', 1160, (s, lv) => {
  s.area('su-kaynaklari-toplama-yeri-kaptaj-alani', 'Su kaynakları toplama yeri (kaptaj alanı)', [solid(GREY), dots(DOTS, lv), picto(lv, 'kaptaj', { caption: 'SUKT' })], {
    ref: 'EK-1a s.8; EK-1e s.183',
    note: `${dotNote(DOTS, lv)} Sembol: çerçevede havza üstünde üç dalga, altında SUKT.`,
  });
  s.line('icme-suyu-ana-iletim-hatti', 'İçme suyu ana iletim hattı', [stroke(BLACK, 0.4), along(circle(2.5, { stroke: BLACK, strokeWidth: 0.3 }), 10, { offset: -1.25 })], {
    ref: 'EK-1a s.8; EK-1e s.184',
    note: '0.4 mm düz çizgi; 0.3 mm çizgili, 10 mm aralıklı 2.5 mm çaplı boş daireler çizgiye teğet, çizim yönünün sağında (EK-1a gibi).',
  });
  s.area('icme-suyu-tesisleri-alani', 'İçme suyu tesisleri alanı (depolama, arıtma, terfi merkezi)', [solid(GREY), dots(DOTS, lv), picto(lv, 'vana')], {
    ref: 'EK-1a s.8; EK-1e s.184',
    note: `${dotNote(DOTS, lv)} Sembol: çerçevede vana.`,
  });
  s.area('atiksu-tesisleri-alani', 'Atıksu tesisleri alanı (arıtma, terfi merkezi)', [solid(GREY), dots(DOTS_ATIKSU, lv), picto(lv, 'atiksu')], {
    ref: 'EK-1a s.8; EK-1e s.185',
    note: `${dotNote(DOTS_ATIKSU, lv)} Sembol: çerçevede oklarla bağlı üç daire.`,
  });
  s.area('kati-atik-tesisleri-alani', 'Katı atık tesisleri alanı (boşaltma, bertaraf, işleme, transfer ve depolama)', [solid(GREY), dots(DOTS, lv), picto(lv, 'geri-donusum')], {
    ref: 'EK-1a s.9; EK-1e s.186',
    note: `${dotNote(DOTS, lv)} Sembol: çerçevede dolu geri dönüşüm işareti.`,
  });
  s.area('tehlikeli-atik-tesisleri-alani', 'Tehlikeli atık tesisleri alanı (bertaraf ve depolama)', [solid(GREY), dots(DOTS, lv), picto(lv, 'biyolojik-tehlike')], {
    ref: 'EK-1a s.9; EK-1e s.187',
    note: `${dotNote(DOTS, lv)} Sembol: çerçevede biyolojik tehlike işareti.`,
  });
  s.area('atik-geri-kazanim-tesisleri-alani', 'Atık geri kazanım tesisleri alanı', [solid(GREY), dots(DOTS_GERI, lv), picto(lv, 'geri-donusum-yesil')], {
    ref: 'EK-1a s.9; EK-1e s.187',
    note: `${dotNote(DOTS_GERI, lv)} Sembol: çerçevede yeşil (0/176/80) geri dönüşüm işareti.`,
  });
  s.area('teknik-altyapi-alani', 'Teknik altyapı alanı', [solid(GREY), dots(DOTS, lv), code(lv, 'TA')], {
    ref: 'EK-1a s.9; EK-1e s.188',
    note: `${dotNote(DOTS, lv)} Sembol: çerçevede kalın "TA".`,
  });
  s.area('su-yuzeyi', 'Su yüzeyi', [solid(rgb(115, 223, 235))], {
    ref: 'EK-1a s.9; EK-1e s.188',
    note: 'Alan 115/223/235. AÇIKLAMA 4: mevcut ya da öneri göl, gölet, deniz ve barajlar bu gösterimle işlenir.',
  });
  const S = SYMBOL[lv];
  s.area(
    'yapay-ada',
    'Yapay ada',
    [solid(rgb(225, 225, 225)), edge(BLACK, 0.2), stack([shape('square', S, { fill: rgb(150, 150, 150), stroke: BLACK, strokeWidth: S * 0.03 }), text('YA', S * 0.42, { font: 'sans', weight: 400 })])],
    { ref: 'EK-1a s.9; EK-1e s.100', note: 'Alan 225/225/225; sınır 0.2 mm düz çizgi. Sembol: 150/150/150 dolu kare içinde normal Arial "YA".' },
  );
  s.area('vahsi-cop-depolama-rehabilitasyon-ve-kapatma-alani', 'Vahşi çöp depolama rehabilitasyon ve kapatma alanı', [solid(GREY), dots(DOTS, lv), picto(lv, 'geri-donusum-bos')], {
    ref: 'EK-1a s.9; EK-1e s.191',
    note: `${dotNote(DOTS, lv)} Sembol: çerçevede boş (yalnız dış çizgi) geri dönüşüm işareti.`,
  });
});
