import { BLACK, dotGrid, hatch, pattern, rgb, shape, solid, svg } from '../dsl';
import { pic } from '../pictograms';
import { RED, picto, sections, type Level } from './common';

/**
 * EK-1a (s.5) › Korunacak alanlar › Bugünkü arazi kullanımı devam
 * ettirilerek korunacak alanlar, every plan level (EK-1e s.90–97).
 */

/** Zeytinlik: grid of the 1 mm dots per level (EK-1e s.94). */
const OLIVE: Record<Level, number> = { uip: 8, nip: 6, cdp: 5 };
/** Kaplumbağa and fok: 60° line spacing per level (EK-1e s.96–97). */
const SEA: Record<Level, number> = { uip: 12, nip: 10, cdp: 8 };

/** "10 mm karolaj merkezlerinde 3 mm kenarlı diken taraması". */
const thorns = () => pattern(svg(pic('diken'), 3, { fill: BLACK }), 10, 10);

export const ARAZI = sections(['Korunacak alanlar', 'Bugünkü arazi kullanımı devam ettirilerek korunacak alanlar'], 'arazi', 1050, (s, lv) => {
  s.area('orman-alani', 'Orman alanı', [solid(rgb(34, 139, 34)), pattern(shape('triangle', 3, { fill: BLACK }), 10, 10)], {
    ref: 'EK-1a s.5; EK-1e s.93',
    note: 'Alan 34/139/34; 10 mm karolaj merkezlerinde 3 mm kenarlı içi dolu eşkenar üçgen (EK-1a çizimi daha sık, EK-1e ölçüsü uygulandı).',
  });
  s.area('zeytinlik-alan', 'Zeytinlik alan', [solid(rgb(233, 250, 190)), dotGrid(OLIVE[lv], OLIVE[lv], 1, BLACK)], {
    ref: 'EK-1a s.5; EK-1e s.94',
    note: `Alan 233/250/190; ${OLIVE[lv]} mm karelaj merkezlerinde 1 mm çaplı içi dolu daire.`,
  });
  s.area('mera-alani', 'Mera alanı', [solid(rgb(112, 184, 0)), pattern(shape('cross', 2, { stroke: BLACK, strokeWidth: 0.2 }), 8, 8)], {
    ref: 'EK-1a s.5; EK-1e s.94',
    note: 'Alan 112/184/0; 8 mm karelaj merkezinde 0.2 mm çizgili 2×2 mm artı.',
  });
  s.area('dogal-karakteri-korunacak-alan', 'Doğal karakteri korunacak alan', [solid(rgb(180, 215, 158)), thorns()], {
    ref: 'EK-1a s.5; EK-1e s.95',
    note: 'Alan 180/215/158; 10 mm karolaj merkezlerinde 3 mm diken (dikenli öbek çizimi). Çizimde sütunlar yarım göz kaydırılmış; metin "karolaj merkezlerinde" dediği için düzgün ızgara kullanıldı.',
  });
  s.area('deniz-kaplumbagalari-ureme-ve-koruma-alani', 'Deniz kaplumbağaları üreme ve koruma alanı', [hatch(60, SEA[lv], 0.4), picto(lv, 'kaplumbaga')], {
    ref: 'EK-1a s.5; EK-1e s.96',
    note: `Şeffaf; 0.4 mm, ${SEA[lv]} mm aralıklı 60° paralel çizgi (EK-1a ince gri çizer; EK-1e siyah 0.4 mm uygulandı). Sembol: çerçevede kaplumbağa.`,
  });
  s.area('akdeniz-foku-yasam-alani', 'Akdeniz foku yaşam alanı', [hatch(60, SEA[lv], 0.4), picto(lv, 'fok')], {
    ref: 'EK-1a s.5; EK-1e s.97',
    note: `Şeffaf; 0.4 mm, ${SEA[lv]} mm aralıklı 60° paralel çizgi. Sembol: çerçevede fok.`,
  });
  s.area('dogal-ve-ekolojik-yapisi-korunacak-alan', 'Doğal ve ekolojik yapısı korunacak alan', [solid(rgb(209, 255, 155)), thorns()], {
    ref: 'EK-1a s.5; EK-1e s.95',
    note: 'Alan 209/255/155 (EK-1a; EK-1e RGB metni 102/153/205 yazar ama örnek yeşildir, EK-1a değeri kullanıldı); 10 mm karolaj merkezlerinde 3 mm diken.',
  });
  s.area('ekolojik-oneme-sahip-alan', 'Ekolojik öneme sahip alan', [pattern(svg(pic('ot'), 3, { fill: RED }), 10, 10)], {
    ref: 'EK-1a s.5; EK-1e s.90',
    note: 'Alan rengi yok; 1 cm karolaj merkezlerinde kırmızı (255/0/0) 0.3 mm diken (beş ince yapraklı öbek). Öbek boyu verilmemiş: 3 mm çizimden.',
  });
});
