import { BLACK, WHITE, along, circle, dashMarkers, framed, rgb, shape, sheet } from '../dsl';
import { S } from './common';

/**
 * ÇDP (EK-1c) › Sınırlar: planlama alt bölgesi, stratejik alt bölge ve
 * stratejik karar (EK-1e s.37-38).
 */

const RED = rgb(255, 0, 0);

export const sinirlar = sheet('cdp', ['Sınırlar'], 'sinirlar', 20);

sinirlar.line('planlama-alt-bolgesi', 'Planlama alt bölgesi', [dashMarkers(RED, 0.3, 1, 3, circle(3, { fill: RED }))], {
  ref: 'EK-1c s.1; EK-1e s.37',
  note: '0,3 mm; 3 mm çaplı dolu daireler, aralarında 1 mm düz çizgi (boncuk dizisi).',
});

/** A circle with an X inside (the legend's ⊗). */
const D = 3.4;
sinirlar.line('stratejik-alt-bolge', 'Stratejik alt bölge', [along([circle(D, { stroke: BLACK, strokeWidth: 0.3 }), shape('x', D, { stroke: BLACK, strokeWidth: 0.3 })], D + 1)], {
  ref: 'EK-1c s.1; EK-1e s.37',
  note: 'İçi çapraz çizgili daireler, daireler arası 1 mm, 0,3 mm çizgi; bağlayan çizgi yok. EK-1e yarıçapı "0,2 mm" yazıyor (içine çarpı sığmaz, yazım hatası); çap EK-1c çiziminden 3,4 mm ölçüldü (çizim bu satırlarda 1:1: planlama alt bölgesinin 3 mm dairesi 3,05 mm ölçülüyor). "3\'lü" gruplama çizimde görünmüyor; daireler eşit aralıklı.',
});
sinirlar.point('stratejik-karar', 'Stratejik karar', [framed('SK', { frame: 'circle', size: S, strokeWidth: 0.25, textSize: 2, font: 'sans', weight: 400, background: WHITE })], {
  ref: 'EK-1c s.1; EK-1e s.38',
  note: 'Daire içinde "SK". EK-1e: proje alanına uygun büyüklükte yazılır; varsayılan boyut ÇDP sembol boyu 5 mm (AÇIKLAMA 4), gerekirse ölçeklenir.',
});
