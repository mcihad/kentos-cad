import { BLACK, WHITE, circle, rgb, shape, sheet, solid, stroke } from '../dsl';

/**
 * MSP (EK-1e bölüm I, s.6-9) › Ekonomik ihtisas bölgeleri ve koridorları.
 * "Alan/nokta" gösterimler iki işarettir: %50 saydam alan ve nokta sembolü.
 * Sembol ölçüleri çizimden (MSP satırları ölçü vermez).
 */

export const ekonomik = sheet('msp', ['Ekonomik ihtisas bölgeleri ve koridorları'], 'ekonomik', 50);

ekonomik.line('ekonomik-gelisme-alanlari', 'Ekonomik gelişme alanları', [stroke(BLACK, 0.7)], { ref: 'EK-1e s.6', note: 'Alan şeffaf; sınır 0,7 mm siyah düz çizgi.' });
ekonomik.point('buyuk-olcekli-endustri-ve-uretim-odaklari', 'Büyük ölçekli endüstri ve üretim odakları', [shape('square', 8, { fill: rgb(130, 40, 255), stroke: BLACK, strokeWidth: 0.5 })], {
  ref: 'EK-1e s.7',
  note: 'Mor (130/40/255) kare, siyah dış çizgi; 8 mm çizimden.',
});
ekonomik.point('tarimsal-faaliyet-odaklari', 'Tarımsal faaliyet odakları', [shape('square', 8, { fill: rgb(255, 255, 0), stroke: BLACK, strokeWidth: 0.5 })], {
  ref: 'EK-1e s.7',
  note: 'Sarı (255/255/0) kare, siyah dış çizgi; 8 mm çizimden.',
});
ekonomik.area('stratejik-tabii-kaynak-alanlari', 'Stratejik tabii kaynak alanları', [solid(rgb(180, 160, 200, 0.5))], { ref: 'EK-1e s.7', note: '180/160/200, %50 saydam. Noktası ayrı işaret.' });
ekonomik.point('stratejik-tabii-kaynak-noktasi', 'Stratejik tabii kaynak alanları (nokta)', [circle(5.8, { fill: rgb(110, 50, 160), stroke: BLACK, strokeWidth: 0.25 })], {
  ref: 'EK-1e s.7',
  note: 'Mor (110/50/160) daire, ince siyah dış çizgi; 5,8 mm çizimden.',
});
ekonomik.point('uluslararasi-giris-cikis-kapilari', 'Uluslararası giriş-çıkış kapıları', [shape('star', 7.8, { fill: rgb(255, 0, 0), stroke: BLACK, strokeWidth: 0.5 })], {
  ref: 'EK-1e s.8',
  note: 'Kırmızı (255/0/0) beş köşeli yıldız, siyah dış çizgi; çevrel çember 7,8 mm çizimden.',
});
ekonomik.point('bolgesel-olcekli-kamu-projeleri-ve-stratejik-yatirimlar', 'Bölgesel ölçekli kamu projeleri ve stratejik yatırımlar', [shape('star', 7.8, { fill: WHITE, stroke: rgb(80, 130, 190), strokeWidth: 0.5 })], {
  ref: 'EK-1e s.8',
  note: 'İçi beyaz, çizgisi mavi (80/130/190) beş köşeli yıldız; çevrel çember 7,8 mm çizimden.',
});
ekonomik.area('turizm-gelisim-koridorlari', 'Turizm gelişim odak ve koridorları', [solid(rgb(250, 150, 70, 0.5))], { ref: 'EK-1e s.8', note: '250/150/70, %50 saydam (koridorlar). Odaklar ayrı nokta işareti.' });
ekonomik.point('turizm-gelisim-odaklari', 'Turizm gelişim odak ve koridorları (nokta)', [circle(5.6, { fill: rgb(250, 150, 70), stroke: BLACK, strokeWidth: 0.25 })], {
  ref: 'EK-1e s.8',
  note: 'Turuncu (250/150/70) daire, ince siyah dış çizgi; 5,6 mm çizimden.',
});
ekonomik.area('buyuk-olcekli-yatirim-alanlari', 'Büyük ölçekli yatırım alanları', [solid(rgb(0, 160, 240, 0.5))], { ref: 'EK-1e s.9', note: '0/160/240, %50 saydam. Noktası ayrı işaret.' });
ekonomik.point(
  'buyuk-olcekli-yatirim-noktasi',
  'Büyük ölçekli yatırım alanları (nokta)',
  [circle(5.7, { fill: rgb(0, 160, 240), stroke: BLACK, strokeWidth: 0.25 }), shape('cross', 5.7, { stroke: BLACK, strokeWidth: 0.25 })],
  { ref: 'EK-1e s.9', note: 'Mavi (0/160/240) daire içinde siyah artı (⊕); 5,7 mm çizimden.' },
);
ekonomik.point('depolama-alanlari', 'Depolama alanları', [shape('hexagon', 6.5, { height: 7.6, fill: rgb(255, 160, 0), stroke: BLACK, strokeWidth: 0.25, rotation: 90 })], {
  ref: 'EK-1e s.9',
  note: 'Turuncu (255/160/0) altıgen, üstü ve altı düz; köşeden köşeye 7,5 mm, düzden düze 6,5 mm (çizimden).',
});
